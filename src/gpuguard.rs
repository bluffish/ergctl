//! "GPU guard": default GL/EGL/GLX to the integrated GPU for the whole graphical
//! session, so Electron/Chromium (and anything else) don't incidentally probe and
//! wake the NVIDIA dGPU at launch. Explicit offload via `prime-run` overrides these
//! per-process, so the dGPU stays usable on demand.

use std::fs;
use std::path::Path;

const DROPIN: &str = "/etc/environment.d/90-ergctl-igpu.conf";

const CONTENT: &str = "\
# Installed by ergctl (gpu-guard): default GL/EGL/GLX to the integrated GPU so
# Electron/Chromium and other apps don't incidentally initialise the NVIDIA dGPU
# (which pins it at D0). `prime-run` still overrides these per-process for games.
__EGL_VENDOR_LIBRARY_FILENAMES=/usr/share/glvnd/egl_vendor.d/50_mesa.json
__GLX_VENDOR_LIBRARY_NAME=mesa
";

pub fn on() {
    if let Err(e) = fs::create_dir_all("/etc/environment.d") {
        eprintln!("ergctl: create /etc/environment.d: {e}");
        return;
    }
    match fs::write(DROPIN, CONTENT) {
        Ok(_) => println!(
            "gpu-guard ON  -> {DROPIN}\n\
             Log out and back in (then restart Electron apps) for it to take effect."
        ),
        Err(e) => eprintln!("ergctl: write {DROPIN}: {e}"),
    }
}

pub fn off() {
    match fs::remove_file(DROPIN) {
        Ok(_) => println!("gpu-guard OFF (removed {DROPIN}); log out/in to revert."),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            println!("gpu-guard already off.")
        }
        Err(e) => eprintln!("ergctl: remove {DROPIN}: {e}"),
    }
}

pub fn status() {
    if Path::new(DROPIN).exists() {
        println!("gpu-guard : ON ({DROPIN})");
    } else {
        println!("gpu-guard : off");
    }
}

pub fn is_on() -> bool {
    Path::new(DROPIN).exists()
}
