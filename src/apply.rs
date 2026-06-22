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
    sysfs::cardwire_set(&s.gpu);
    // nvidia-powerd (Dynamic Boost) keeps the dGPU at D0; only run it when the
    // dGPU is actually available, so Integrated mode can reach D3cold.
    sysfs::set_service("nvidia-powerd", s.gpu != "integrated");
    println!(
        "[ergctl] {label}: profile={} gpu={} boost={} epp={}",
        s.profile, s.gpu, s.boost, s.epp
    );
}

/// Apply the coherent state for (override, power source). Idempotent.
pub fn apply_current() {
    let cfg = Config::load(&config::path());
    if let Some(cl) = cfg.charge_limit {
        sysfs::set_charge_limit(cl);
    }
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
    let cl = sysfs::bat_dir()
        .and_then(|d| sysfs::read_trim(&format!("{d}/charge_control_end_threshold")))
        .unwrap_or_default();
    println!("charge limit     : {cl}");
}
