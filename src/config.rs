//! Flat `key = value` config (no TOML dependency). One file, declarative.

use std::collections::HashMap;
use std::fs;

/// The knobs that make up one power state.
pub struct StateCfg {
    pub profile: String, // ACPI platform_profile: quiet|balanced|performance
    pub gpu: String,     // cardwire mode: integrated|hybrid|smart
    pub boost: bool,     // CPU turbo
    pub epp: String,     // energy_performance_preference
}

pub struct Config {
    pub charge_limit: Option<u32>,
    pub battery: StateCfg,
    pub ac: StateCfg,
    pub turbo: StateCfg,
}

pub const DEFAULT_PATH: &str = "/etc/ergctl.conf";

/// Active config path (overridable via ERGCTL_CONFIG, for testing).
pub fn path() -> String {
    std::env::var("ERGCTL_CONFIG").unwrap_or_else(|_| DEFAULT_PATH.to_string())
}

impl Config {
    pub fn load(path: &str) -> Config {
        let mut m: HashMap<String, String> = HashMap::new();
        if let Ok(text) = fs::read_to_string(path) {
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=') {
                    m.insert(k.trim().to_string(), v.trim().to_string());
                }
            }
        }

        let s = |k: &str, d: &str| m.get(k).cloned().unwrap_or_else(|| d.to_string());
        let b = |k: &str, d: bool| {
            m.get(k)
                .map(|v| matches!(v.as_str(), "true" | "1" | "on" | "yes"))
                .unwrap_or(d)
        };

        Config {
            charge_limit: m.get("charge_limit").and_then(|v| v.parse().ok()),
            battery: StateCfg {
                profile: s("battery_profile", "quiet"),
                // hybrid (not integrated): RTD3 + gpu-guard keep the dGPU asleep
                // without the Integrated-mode block that traps it at D0.
                gpu: s("battery_gpu", "hybrid"),
                boost: b("battery_boost", false),
                epp: s("battery_epp", "power"),
            },
            ac: StateCfg {
                profile: s("ac_profile", "balanced"),
                gpu: s("ac_gpu", "hybrid"),
                boost: b("ac_boost", true),
                epp: s("ac_epp", "balance_performance"),
            },
            turbo: StateCfg {
                profile: s("turbo_profile", "performance"),
                gpu: s("turbo_gpu", "hybrid"),
                boost: b("turbo_boost", true),
                epp: s("turbo_epp", "performance"),
            },
        }
    }
}

/// Update (or append) a single `key = value` line in the config file, preserving
/// comments and other keys. Used by the TUI to persist e.g. the charge limit.
pub fn set_key(path: &str, key: &str, val: &str) -> std::io::Result<()> {
    let mut lines: Vec<String> = fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect();
    let mut found = false;
    for line in lines.iter_mut() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        if let Some((k, _)) = line.split_once('=') {
            if k.trim() == key {
                *line = format!("{key} = {val}");
                found = true;
                break;
            }
        }
    }
    if !found {
        lines.push(format!("{key} = {val}"));
    }
    fs::write(path, lines.join("\n") + "\n")
}
