//! Live system snapshot for the TUI. One cheap poll per tick.

use crate::{apply, sysfs};

pub struct Snapshot {
    // power
    pub ac_online: bool,
    pub override_mode: String,
    pub draw_w: f64,
    pub rapl_uj: u64,
    // battery
    pub capacity: u32,
    pub bat_status: String,
    pub charge_limit: u32,
    pub energy_now: f64,    // Wh remaining
    pub energy_full: f64,   // Wh
    pub energy_design: f64, // Wh
    pub cycles: u32,
    pub time_left_h: f64,
    // cpu
    pub profile: String,
    pub boost: bool,
    pub epp: String,
    pub governor: String,
    pub cpu_mhz: u32,
    pub cpu_temp: f64,
    // gpu
    pub dgpu_state: String,
    pub cardwire_mode: String,
    pub dgpu_waker: Option<String>,
    // system
    pub fan_rpm: u32,
    pub brightness_pct: u32,
    pub gpu_guard: bool,
    pub audio_guard: bool,
}

/// Service health — refreshed infrequently (each check is a systemctl fork, which
/// we don't want to spawn every tick on battery). Read via read_services().
pub struct Services {
    pub tlp: bool,
    pub asusd: bool,
}

pub fn read_services() -> Services {
    Services {
        tlp: sysfs::service_active("tlp"),
        asusd: sysfs::service_active("asusd"),
    }
}

impl Snapshot {
    pub fn dgpu_awake(&self) -> bool {
        self.dgpu_state == "active"
    }
    pub fn health_pct(&self) -> f64 {
        if self.energy_design > 0.0 {
            100.0 * self.energy_full / self.energy_design
        } else {
            0.0
        }
    }
}

fn bat_u64(bat: &Option<String>, attr: &str) -> u64 {
    bat.as_ref()
        .and_then(|d| sysfs::read_u64(&format!("{d}/{attr}")))
        .unwrap_or(0)
}

pub fn read() -> Snapshot {
    let bat = sysfs::bat_dir();
    let cur = bat_u64(&bat, "current_now") as f64; // µA
    let volt = bat_u64(&bat, "voltage_now") as f64; // µV
    let draw_w = cur * volt / 1e12;
    let charge_now = bat_u64(&bat, "charge_now") as f64; // µAh
    let charge_full = bat_u64(&bat, "charge_full") as f64;
    let charge_design = bat_u64(&bat, "charge_full_design") as f64;
    // Prefer energy_now (µWh) if the battery reports it; else charge_now·voltage.
    let energy_now = {
        let e = bat_u64(&bat, "energy_now") as f64;
        if e > 0.0 {
            e / 1e6
        } else {
            charge_now * volt / 1e12
        }
    };
    let status = bat
        .as_ref()
        .and_then(|d| sysfs::read_trim(&format!("{d}/status")))
        .unwrap_or_default();
    let time_left_h = if status == "Discharging" && cur > 0.0 {
        charge_now / cur
    } else {
        0.0
    };

    Snapshot {
        ac_online: sysfs::on_ac(),
        override_mode: apply::read_override(),
        draw_w,
        rapl_uj: sysfs::read_u64(sysfs::RAPL_ENERGY).unwrap_or(0),
        capacity: bat_u64(&bat, "capacity") as u32,
        bat_status: status,
        charge_limit: bat_u64(&bat, "charge_control_end_threshold") as u32,
        energy_now,
        energy_full: charge_full * volt / 1e12,
        energy_design: charge_design * volt / 1e12,
        cycles: bat_u64(&bat, "cycle_count") as u32,
        time_left_h,
        profile: sysfs::read_trim("/sys/firmware/acpi/platform_profile").unwrap_or_default(),
        boost: sysfs::read_trim("/sys/devices/system/cpu/cpufreq/boost").as_deref() == Some("1"),
        epp: sysfs::read_trim("/sys/devices/system/cpu/cpu0/cpufreq/energy_performance_preference")
            .unwrap_or_default(),
        governor: sysfs::read_trim("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
            .unwrap_or_default(),
        cpu_mhz: sysfs::avg_cpu_mhz(),
        cpu_temp: sysfs::cpu_temp_c(),
        dgpu_state: sysfs::dgpu_runtime_status(),
        cardwire_mode: sysfs::cardwire_get(),
        dgpu_waker: crate::dgpuwatch::last_waker(),
        fan_rpm: sysfs::fan_rpm(),
        brightness_pct: sysfs::brightness_pct(),
        gpu_guard: crate::gpuguard::is_on(),
        audio_guard: crate::audioguard::is_on(),
    }
}
