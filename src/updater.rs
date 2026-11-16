//! Auto-updater. Active only if the endpoint and the public signing key were set at build time
//! (`NOTE_WARDEN_UPDATE_URL`, `NOTE_WARDEN_UPDATE_PUBKEY`, see docs/releasing.md). The app's only network request.
//! The installer is only started after the signature has been verified.
use std::{
    io::Read,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

const MAX_MANIFEST_BYTES: u64 = 256 * 1024;
const MAX_INSTALLER_BYTES: usize = 256 * 1024 * 1024;

const ENDPOINT: Option<&str> = option_env!("NOTE_WARDEN_UPDATE_URL");
const PUBLIC_KEY: Option<&str> = option_env!("NOTE_WARDEN_UPDATE_PUBKEY");

#[derive(Clone, PartialEq)]
pub struct Info {
    pub version: String,
    pub notes: String,
    url: String,
    signature: String,
}

#[derive(Clone, Default, PartialEq)]
pub enum State {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available(Info),
    /// Fortschritt 0.0..=1.0
    Downloading(Info, f32),
    Ready(Info, PathBuf),
    Failed,
}

pub type Shared = Arc<Mutex<State>>;

pub fn configured() -> bool {
    // A portable copy must not be replaced by the installer; it is updated by hand.
    !crate::storage::is_portable()
        && ENDPOINT.is_some_and(|endpoint| endpoint.starts_with("https://"))
        && PUBLIC_KEY.is_some_and(|key| minisign_verify::PublicKey::from_base64(key.trim()).is_ok())
}

fn set(shared: &Shared, state: State) {
    if let Ok(mut slot) = shared.lock() {
        *slot = state;
    }
    crate::system::send(crate::system::Event::UpdaterChanged);
}

#[derive(serde::Deserialize)]
struct Manifest {
    version: String,
    #[serde(default)]
    notes: String,
    url: String,
    signature: String,
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .https_only(true)
        .timeout_global(Some(Duration::from_secs(120)))
        .build()
        .into()
}

fn fetch_manifest() -> Result<Manifest, String> {
    let mut body = agent()
        .get(ENDPOINT.unwrap_or_default())
        .call()
        .map_err(|e| e.to_string())?
        .into_body();
    let text = body
        .with_config()
        .limit(MAX_MANIFEST_BYTES)
        .read_to_string()
        .map_err(|e| e.to_string())?;
    let manifest: Manifest = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

fn validate_manifest(manifest: &Manifest) -> Result<(), String> {
    // The version becomes part of a local filename. Accept only an exact stable
    // version, and reject traversal, whitespace and unbounded numeric components.
    let parts: Vec<_> = manifest.version.split('.').collect();
    if parts.len() != 3
        || parts.iter().any(|part| {
            part.is_empty()
                || !part.bytes().all(|byte| byte.is_ascii_digit())
                || part.parse::<u64>().is_err()
        })
    {
        return Err("Ungültige Update-Version".into());
    }
    if !manifest.url.starts_with("https://") {
        return Err("Update-Downloads benötigen HTTPS".into());
    }
    minisign_verify::Signature::decode(&manifest.signature).map_err(|e| e.to_string())?;
    Ok(())
}

fn verify_installer(bytes: &[u8], public_key: &str, signature: &str) -> Result<(), String> {
    let key =
        minisign_verify::PublicKey::from_base64(public_key.trim()).map_err(|e| e.to_string())?;
    let signature = minisign_verify::Signature::decode(signature).map_err(|e| e.to_string())?;
    key.verify(bytes, &signature, false)
        .map_err(|_| "Signatur ungültig".to_owned())
}

/// Checks in the background for a newer version.
pub fn check(shared: &Shared) {
    if !configured() {
        return;
    }
    set(shared, State::Checking);
    let shared = shared.clone();
    std::thread::spawn(move || {
        crate::log::info("Update-Prüfung");
        match fetch_manifest() {
            Ok(manifest) if crate::md::is_newer(&manifest.version, crate::VERSION) => {
                let info = Info {
                    version: manifest.version,
                    notes: manifest.notes,
                    url: manifest.url,
                    signature: manifest.signature,
                };
                set(&shared, State::Available(info));
            }
            Ok(_) => set(&shared, State::UpToDate),
            Err(error) => {
                crate::log::error(&format!("Update-Prüfung fehlgeschlagen: {error}"));
                set(&shared, State::Failed);
            }
        }
    });
}

fn download_and_verify(shared: &Shared, info: &Info) -> Result<PathBuf, String> {
    let response = agent().get(&info.url).call().map_err(|e| e.to_string())?;
    let total = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());
    if total.is_some_and(|length| length > MAX_INSTALLER_BYTES as u64) {
        return Err("Update-Download ist zu groß".into());
    }
    let mut reader = response.into_body().into_reader();
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 64 * 1024];
    loop {
        let read = reader.read(&mut chunk).map_err(|e| e.to_string())?;
        if read == 0 {
            break;
        }
        if bytes.len() + read > MAX_INSTALLER_BYTES {
            return Err("Update-Download ist zu groß".into());
        }
        bytes.extend_from_slice(&chunk[..read]);
        if let Some(total) = total.filter(|t| *t > 0) {
            set(
                shared,
                State::Downloading(info.clone(), (bytes.len() as f32 / total as f32).min(1.)),
            );
        }
    }
    verify_installer(&bytes, PUBLIC_KEY.unwrap_or_default(), &info.signature)?;
    let dir = std::env::temp_dir().join("note-warden-update");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("NoteWarden-{}-setup.exe", info.version));
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(path)
}

pub fn download(shared: &Shared, info: Info) {
    set(shared, State::Downloading(info.clone(), 0.));
    let shared = shared.clone();
    std::thread::spawn(move || match download_and_verify(&shared, &info) {
        Ok(path) => set(&shared, State::Ready(info, path)),
        Err(error) => {
            crate::log::error(&format!("Update-Download fehlgeschlagen: {error}"));
            set(&shared, State::Failed);
        }
    });
}

/// Starts the verified installer; its explicit update mode restarts the app.
pub fn launch_installer(path: &std::path::Path) -> bool {
    let mut command = std::process::Command::new(path);
    command.args([
        "/SP-",
        "/SILENT",
        "/NORESTART",
        "/CLOSEAPPLICATIONS",
        "/NOTEWARDENUPDATE=1",
    ]);
    // Keep an existing installation in its actual directory. A development or
    // portable binary instead installs to the standard per-user location.
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
        && dir.join("unins000.exe").is_file()
    {
        let mut target = std::ffi::OsString::from("/DIR=");
        target.push(dir);
        command.arg(target);
    }
    command.spawn().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "Requires the published GitHub endpoint and compile-time release URL/public key"]
    fn published_update_download_verifies() {
        assert!(
            configured(),
            "Build this test with the official update URL and public key"
        );
        let manifest =
            fetch_manifest().expect("Published update manifest must be reachable and valid");
        let info = Info {
            version: manifest.version,
            notes: manifest.notes,
            url: manifest.url,
            signature: manifest.signature,
        };
        let shared = Arc::new(Mutex::new(State::Idle));
        let installer = download_and_verify(&shared, &info)
            .expect("Official installer must download and pass signature verification");
        assert!(installer.metadata().unwrap().len() > 0);
    }

    #[test]
    fn configured_release_key_matches_signed_fixture() {
        let public_key = include_str!("../installer/update-signing.pub")
            .lines()
            .nth(1)
            .unwrap();
        let message = include_bytes!("../installer/release-key-check.txt");
        let signature = include_str!("../installer/release-key-check.txt.minisig");
        assert!(verify_installer(message, public_key, signature).is_ok());
        assert!(verify_installer(b"changed release fixture", public_key, signature).is_err());
    }

    // Published minisign-verify interoperability vector (message: "test").
    const KEY: &str = "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";
    const SIGNATURE: &str = "untrusted comment: signature from minisign secret key
RUQf6LRCGA9i559r3g7V1qNyJDApGip8MfqcadIgT9CuhV3EMhHoN1mGTkUidF/z7SrlQgXdy8ofjb7bNJJylDOocrCo8KLzZwo=
trusted comment: timestamp:1633700835\tfile:test\tprehashed
wLMDjy9FLAuxZ3q4NlEvkgtyhrr0gtTu6KC4KBJdITbbOeAi1zBIYo0v4iTgt8jJpIidRJnp94ABQkJAgAooBQ==";

    #[test]
    fn signed_download_rejects_tampering_and_invalid_keys() {
        assert!(verify_installer(b"test", KEY, SIGNATURE).is_ok());
        assert!(verify_installer(b"tampered", KEY, SIGNATURE).is_err());
        assert!(verify_installer(b"test", "invalid", SIGNATURE).is_err());
        assert!(verify_installer(b"test", KEY, "invalid").is_err());
    }

    #[test]
    fn manifest_rejects_unsafe_versions_and_transport() {
        let mut manifest = Manifest {
            version: "1.2.3".into(),
            notes: String::new(),
            url: "https://github.com/example/app/releases/download/v1.2.3/setup.exe".into(),
            signature: SIGNATURE.into(),
        };
        assert!(validate_manifest(&manifest).is_ok());
        for version in [
            "../1.2.3",
            "1.2.3/../setup",
            " 1.2.3",
            "1.2",
            "1.2.3-beta",
            "1.2.18446744073709551616",
        ] {
            manifest.version = version.into();
            assert!(validate_manifest(&manifest).is_err(), "{version}");
        }
        manifest.version = "1.2.3".into();
        manifest.url = "http://example.com/setup.exe".into();
        assert!(validate_manifest(&manifest).is_err());
    }
}
