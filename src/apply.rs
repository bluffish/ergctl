//! The decision + apply engine and the persisted override flag.

use crate::config::{self, Config, StateCfg};
use crate::sysfs;
use std::fs;

const STATE_DIR: &str = "/run/ergctl";
const STATE_FILE: &str = "/run/ergctl/mode";

/// Persisted override: "turbo" if explicitly set, otherwise "auto".
pub fn read_override() -> String {
    fs::read_to_string(STATE_FILE)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| s == "turbo")
        .unwrap_or_else(|| "auto".to_string())
}

fn write_override(mode: &str) {
    let _ = fs::create_dir_all(STATE_DIR);
    if let Err(e) = fs::write(STATE_FILE, mode) {
        eprintln!("ergctl: persist override: {e}");
    }
}

fn apply_state(label: &str, s: &StateCfg) {
    sysfs::set_platform_profile(&s.profile);
    sysfs::set_boost(s.boost);
    sysfs::set_epp(&s.epp);
    // dGPU: cardwire blocks it (integrated) on battery / makes it available (hybrid)
    // on AC. With cardwire's experimental_nvidia_block on, integrated blocks the
    // proprietary /dev/nvidia* nodes too, so nothing can wake it; RTD3 then D3colds it.
    sysfs::cardwire_set(&s.gpu_mode);
    println!(
        "[ergctl] {label}: profile={} boost={} epp={} gpu={}",
        s.profile, s.boost, s.epp, s.gpu_mode
    );
}

/// Apply the coherent state for (override, power source). Idempotent.
pub fn apply_current() {
    let cfg = Config::load(&config::path());
    if let Some(cl) = cfg.charge_limit {
        sysfs::set_charge_limit(cl);
    }
    // Backstop: if the dGPU HDMI-audio function re-appeared (boot/resume PCI
    // re-enumeration), re-remove it so the GPU can D3cold. The udev rule is the
    // fast path for mid-session re-adds; this covers boot/resume deterministically.
    crate::audioguard::enforce();
    match read_override().as_str() {
        "turbo" => apply_state("turbo", &cfg.turbo),
        _ if sysfs::on_ac() => apply_state("ac", &cfg.ac),
        _ => apply_state("battery", &cfg.battery),
    }
}

pub fn set_mode_and_apply(mode: &str) {
    write_override(mode);
    apply_current();
}

pub fn status() {
    println!("override         : {}", read_override());
    println!(
        "AC online        : {}",
        if sysfs::on_ac() { "yes" } else { "no" }
    );
    println!(
        "platform_profile : {}",
        sysfs::read_trim("/sys/firmware/acpi/platform_profile").unwrap_or_default()
    );
    println!(
        "cpu boost        : {}",
        sysfs::read_trim("/sys/devices/system/cpu/cpufreq/boost").unwrap_or_default()
    );
    println!(
        "EPP (cpu0)       : {}",
        sysfs::read_trim("/sys/devices/system/cpu/cpu0/cpufreq/energy_performance_preference")
            .unwrap_or_default()
    );
    println!("GPU mode         : {}", sysfs::cardwire_get());
    println!("dGPU power       : {}", sysfs::dgpu_runtime_status());
    println!(
        "audio-guard      : {}",
        if crate::audioguard::is_on() { "on" } else { "off" }
    );
    let cl = sysfs::bat_dir()
        .and_then(|d| sysfs::read_trim(&format!("{d}/charge_control_end_threshold")))
        .unwrap_or_default();
    println!("charge limit     : {cl}");
    if let Some(d) = sysfs::bat_dir() {
        let e = sysfs::read_u64(&format!("{d}/energy_now")).unwrap_or(0);
        let wh = if e > 0 {
            e as f64 / 1e6
        } else {
            let c = sysfs::read_u64(&format!("{d}/charge_now")).unwrap_or(0) as f64;
            let v = sysfs::read_u64(&format!("{d}/voltage_now")).unwrap_or(0) as f64;
            c * v / 1e12
        };
        let cap = sysfs::read_trim(&format!("{d}/capacity")).unwrap_or_default();
        println!("remaining        : {wh:.1} Wh ({cap}%)");
    }
}
