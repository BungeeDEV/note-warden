fn main() {
    println!("cargo:rerun-if-env-changed=NOTE_WARDEN_UPDATE_URL");
    println!("cargo:rerun-if-env-changed=NOTE_WARDEN_UPDATE_PUBKEY");
    println!("cargo:rerun-if-changed=branding/app.ico");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=CHANGELOG.md");
    // Windows allows renaming a running EXE, but not overwriting it.
    // Only move our own Cargo artifact out of the way;
    // running instances and their data stay untouched.
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
                    std::fs::rename(&exe, &previous)
                        .expect("Could not rename our own build EXE; check the file permissions");
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
    // The nested GPUI element structure needs more in the debug build than the
    // 1 MiB default stack of the Windows main thread. 16 MiB is enough with headroom.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bins=/STACK:16777216");
    }
    // Embed the app icon (Explorer, taskbar, title bar) into the exe as a resource.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("branding/app.ico");
        res.set("ProductName", "Note Warden");
        res.set("FileDescription", "Note Warden");
        if let Err(error) = res.compile() {
            println!("cargo:warning=Could not embed the icon: {error}");
        }
    }
}
