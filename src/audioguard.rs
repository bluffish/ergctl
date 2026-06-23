//! "Audio guard": the dGPU's HDMI audio function is a PCI device-link *consumer*
//! of the GPU, so while snd_hda_intel keeps it bound the GPU can't reach D3cold.
//! No display uses that audio, so we unbind it (and keep it unbound via udev).

use std::fs;
use std::path::Path;
use std::process::Command;

const RULE: &str = "/etc/udev/rules.d/90-ergctl-dgpu-audio.rules";

const CONTENT: &str = "\
# Installed by ergctl (audio-guard): the NVIDIA dGPU's HDMI audio function is a
# device-link consumer of the GPU and pins it at D0. Nothing uses it (no display
# on the dGPU), so unbind it whenever snd_hda_intel grabs it.
ACTION==\"bind\", SUBSYSTEM==\"pci\", ATTR{vendor}==\"0x10de\", ATTR{class}==\"0x040300\", RUN+=\"/bin/sh -c 'echo %k > /sys/bus/pci/drivers/snd_hda_intel/unbind'\"
";

/// PCI ids of NVIDIA HDMI-audio functions (vendor 10de, class 0403xx).
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

pub fn on() {
    if let Err(e) = fs::write(RULE, CONTENT) {
        eprintln!("ergctl: write {RULE}: {e}");
        return;
    }
    reload_udev();
    for d in nvidia_audio_devs() {
        let _ = fs::write("/sys/bus/pci/drivers/snd_hda_intel/unbind", &d);
    }
    println!("audio-guard ON  -> {RULE} (dGPU HDMI audio unbound; GPU can now D3cold)");
}

pub fn off() {
    let existed = Path::new(RULE).exists();
    let _ = fs::remove_file(RULE);
    reload_udev();
    // rebind so HDMI audio works again
    for d in nvidia_audio_devs() {
        let _ = fs::write("/sys/bus/pci/drivers_probe", &d);
    }
    println!(
        "audio-guard OFF{}; HDMI audio rebound.",
        if existed { "" } else { " (was already off)" }
    );
}

pub fn status() {
    println!("audio-guard : {}", if is_on() { "ON" } else { "off" });
}

pub fn is_on() -> bool {
    Path::new(RULE).exists()
}
