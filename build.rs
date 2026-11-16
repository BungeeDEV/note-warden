fn main() {
    println!("cargo:rerun-if-env-changed=NOTE_WARDEN_UPDATE_URL");
    println!("cargo:rerun-if-env-changed=NOTE_WARDEN_UPDATE_PUBKEY");
    println!("cargo:rerun-if-changed=branding/app.ico");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=CHANGELOG.md");
    // Windows erlaubt das Umbenennen einer laufenden EXE, aber nicht ihr
    // Überschreiben. Nur das eigene Cargo-Artefakt aus dem Weg räumen;
    // laufende Instanzen und ihre Daten bleiben unberührt.
    if cfg!(windows) {
        let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
        if let Some(profile) = out.ancestors().nth(3) {
            for (index, exe) in [
                profile.join("note-warden.exe"),
                profile.join("deps/note_warden.exe"),
            ]
            .into_iter()
            .enumerate()
            {
                if exe.is_file() {
                    let stamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("Systemzeit")
                        .as_nanos();
                    let previous = out.join(format!("note-warden-running-{stamp}-{index}.exe"));
                    std::fs::rename(&exe, &previous).expect("Eigene Build-EXE konnte nicht umbenannt werden; Dateiberechtigungen prüfen");
                    let _ = std::fs::remove_file(previous);
                }
            }
            if let Ok(entries) = std::fs::read_dir(&out) {
                for entry in entries.flatten() {
                    if entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with("note-warden-running-")
                    {
                        let _ = std::fs::remove_file(entry.path());
                    }
                }
            }
        }
    }
    // Die verschachtelte GPUI-Elementstruktur braucht im Debug-Build mehr als die
    // 1 MiB Standard-Stack der Windows-Hauptthread. 16 MiB reichen mit Reserve.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bins=/STACK:16777216");
    }
    // App-Icon (Explorer, Taskleiste, Titelleiste) als Ressource in die Exe einbetten.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("branding/app.ico");
        res.set("ProductName", "Note Warden");
        res.set("FileDescription", "Note Warden");
        if let Err(error) = res.compile() {
            println!("cargo:warning=Icon konnte nicht eingebettet werden: {error}");
        }
    }
}
