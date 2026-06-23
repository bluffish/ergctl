//! "Audio guard": the dGPU's HDMI/DP audio function is a PCI runtime-PM device-link
//! *consumer* of the GPU, so while it is present at D0 the GPU can never reach
//! D3cold. Unbinding the driver is NOT enough — a driverless PCI function stays at
//! D0 (RPM_ACTIVE, no runtime-suspend path) and keeps pinning the GPU. So we fully
//! REMOVE the function, and re-remove it on every enumeration via a udev rule.
//!
//! Scope is deliberately narrow (vendor 0x10de + class 0x040300) so the AMD iGPU
//! HDMI audio (vendor 0x1002, same class) and onboard analog audio are untouched.
//! Tradeoff: while the guard is on, audio over a dGPU-routed HDMI/DP display is
//! unavailable (the dGPU does expose connectors); `audio-guard off` restores it.

use std::fs;
use std::path::Path;
use std::process::Command;

const RULE: &str = "/etc/udev/rules.d/90-ergctl-dgpu-audio.rules";

const CONTENT: &str = "\
# Installed by ergctl (audio-guard).
# The NVIDIA dGPU HDMI/DP audio function (vendor 0x10de, class 0x040300) is a
# runtime-PM device-link consumer of the GPU; while present at D0 it pins the GPU
# and blocks D3cold. Unbinding leaves it driverless but still at D0, so we fully
# REMOVE the function the moment the PCI core enumerates it.
#   - ACTION==add fires on boot/coldplug, S3/hibernate firmware re-enum, pci/rescan.
#   - ACTION==bind closes the add->bind race (the fn can briefly bind+re-pin the GPU
#     before the add rule is processed).
#   (s2idle resume does NOT re-enumerate, so the removal simply persists.)
# Narrow vendor/class match leaves the AMD iGPU HDMI audio (vendor 0x1002) alone.
ACTION==\"add\",  SUBSYSTEM==\"pci\", ATTR{vendor}==\"0x10de\", ATTR{class}==\"0x040300\", ATTR{remove}=\"1\"
ACTION==\"bind\", SUBSYSTEM==\"pci\", ATTR{vendor}==\"0x10de\", ATTR{class}==\"0x040300\", ATTR{remove}=\"1\"
";

/// Currently-present NVIDIA HDMI-audio functions (vendor 10de, class 0403xx).
fn nvidia_audio_devs() -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir("/sys/bus/pci/devices") {
        for e in entries.flatten() {
            let p = e.path();
            let vendor = fs::read_to_string(p.join("vendor")).unwrap_or_default();
            let class = fs::read_to_string(p.join("class")).unwrap_or_default();
            if vendor.trim() == "0x10de" && class.trim().starts_with("0x0403") {
                out.push(e.file_name().to_string_lossy().to_string());
            }
        }
    }
    out
}

fn reload_udev() {
    let _ = Command::new("udevadm").args(["control", "--reload"]).status();
}

/// Remove every currently-present NVIDIA HDMI-audio function. Cheap (no rule
/// rewrite, no udev reload) — safe to call from the boot/resume apply path as a
/// backstop. Does NOT wake the GPU (writing .1/remove doesn't touch .0).
pub fn enforce() {
    if !is_on() {
        return;
    }
    for d in nvidia_audio_devs() {
        let _ = fs::write(format!("/sys/bus/pci/devices/{d}/remove"), "1");
    }
}

pub fn on() {
    if let Err(e) = fs::write(RULE, CONTENT) {
        eprintln!("ergctl: write {RULE}: {e}");
        return;
    }
    reload_udev();
    // Remove (not unbind) any present function now so the GPU can D3cold immediately.
    for d in nvidia_audio_devs() {
        let _ = fs::write(format!("/sys/bus/pci/devices/{d}/remove"), "1");
    }
    println!("audio-guard ON  -> {RULE} (dGPU HDMI audio removed; GPU can now D3cold)");
    println!("note: dGPU-routed HDMI/DP display audio is unavailable until 'audio-guard off'.");
}

pub fn off() {
    let existed = Path::new(RULE).exists();
    let _ = fs::remove_file(RULE);
    reload_udev();
    // The function was physically removed; rescan the bus to bring it back, then
    // nudge a driver probe. NOTE: a bus rescan reads the GPU's config space and
    // WAKES it from D3cold — acceptable only for this explicit user action, which
    // is why off() must never run automatically.
    let _ = fs::write("/sys/bus/pci/rescan", "1");
    for d in nvidia_audio_devs() {
        let _ = fs::write("/sys/bus/pci/drivers_probe", &d);
    }
    println!(
        "audio-guard OFF{}; HDMI audio re-added (dGPU woken by rescan).",
        if existed { "" } else { " (was already off)" }
    );
}

pub fn status() {
    println!("audio-guard : {}", if is_on() { "ON" } else { "off" });
}

pub fn is_on() -> bool {
    Path::new(RULE).exists()
}
