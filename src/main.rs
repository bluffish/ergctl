//! ergctl — power cockpit for the ASUS ProArt P16. CLI + TUI frontends over the
//! ergctl core library.

use ergctl::{apply, audioguard, dgpuwatch, gpuguard, tui, waybar};
use std::io::IsTerminal;
use std::process::{exit, Command};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // No subcommand: open the TUI when interactive, else print status.
    let default = if std::io::stdout().is_terminal() {
        "tui"
    } else {
        "status"
    };
    let cmd = args.get(1).map(String::as_str).unwrap_or(default);

    match cmd {
        "status" => apply::status(),
        "auto" => {
            ensure_root();
            apply::set_mode_and_apply("auto");
        }
        "turbo" => {
            ensure_root();
            apply::set_mode_and_apply("turbo");
        }
        "apply" => {
            ensure_root();
            apply::apply_current();
        }
        "tui" => {
            ensure_root();
            if let Err(e) = tui::run() {
                eprintln!("ergctl: tui error: {e}");
                exit(1);
            }
        }
        // Trace what wakes the dGPU (records to /run/ergctl/dgpu-waker + a log).
        // Long-running; needs root + bpftrace. Driven by ergctl-dgpu-watch.service.
        "dgpu-watch" => {
            ensure_root();
            dgpuwatch::watch();
        }
        // Read-only status for a Waybar custom module; no root needed.
        "waybar" => waybar::emit(args.iter().any(|a| a == "--watch")),
        "gpu-guard" => match args.get(2).map(String::as_str).unwrap_or("status") {
            "on" => {
                ensure_root();
                gpuguard::on();
            }
            "off" => {
                ensure_root();
                gpuguard::off();
            }
            "status" => gpuguard::status(),
            other => {
                eprintln!("ergctl: gpu-guard: unknown '{other}' (use on|off|status)");
                exit(2);
            }
        },
        "audio-guard" => match args.get(2).map(String::as_str).unwrap_or("status") {
            "on" => {
                ensure_root();
                audioguard::on();
            }
            "off" => {
                ensure_root();
                audioguard::off();
            }
            "status" => audioguard::status(),
            other => {
                eprintln!("ergctl: audio-guard: unknown '{other}' (use on|off|status)");
                exit(2);
            }
        },
        "-h" | "--help" | "help" => print_help(),
        "-V" | "--version" => println!("ergctl {}", env!("CARGO_PKG_VERSION")),
        other => {
            eprintln!("ergctl: unknown command '{other}'\n");
            print_help();
            exit(2);
        }
    }
}

fn print_help() {
    println!(
        "ergctl {}\n\n\
         USAGE:\n  ergctl [command]\n\n\
         COMMANDS:\n\
         \x20 (none)   Open the TUI cockpit (when run in a terminal)\n\
         \x20 tui      Open the TUI cockpit explicitly\n\
         \x20 auto     Hand control to automatic battery/AC switching (default mode)\n\
         \x20 turbo    Force max performance, even on battery (until 'auto' or reboot)\n\
         \x20 status   Print current mode and live power state\n\
         \x20 waybar [--watch]  Emit Waybar JSON (power draw + dGPU state); --watch streams\n\
         \x20 apply    Re-apply the correct state for the current override + power source\n\
         \x20          (used by the systemd service on boot/resume/power events)\n\
         \x20 gpu-guard {{on|off|status}}  Default GL/EGL to the iGPU so Electron/Chromium\n\
         \x20          apps don't wake the dGPU (prime-run still overrides for games)\n\
         \x20 audio-guard {{on|off|status}}  Remove the dGPU's HDMI audio function so it\n\
         \x20          stops pinning the GPU at D0 (off restores it; wakes the dGPU)\n\
         \x20 dgpu-watch  Trace + log what wakes the dGPU (needs bpftrace; the\n\
         \x20          ergctl-dgpu-watch.service runs this and the TUI shows the last waker)\n\n\
         CONFIG:\n  /etc/ergctl.conf",
        env!("CARGO_PKG_VERSION")
    );
}

/// Effective UID from /proc/self/status (no libc dependency).
fn euid() -> u32 {
    if let Ok(s) = std::fs::read_to_string("/proc/self/status") {
        for line in s.lines() {
            if let Some(rest) = line.strip_prefix("Uid:") {
                if let Some(e) = rest.split_whitespace().nth(1) {
                    return e.parse().unwrap_or(1);
                }
            }
        }
    }
    1
}

/// Mutating commands (and the TUI, which can write + reads RAPL) need root. If we
/// aren't root, re-exec the same invocation under sudo.
fn ensure_root() {
    if euid() == 0 {
        return;
    }
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("ergctl: cannot find own path: {e}");
            exit(1);
        }
    };
    let rest: Vec<String> = std::env::args().skip(1).collect();
    match Command::new("sudo").arg(exe).args(&rest).status() {
        Ok(s) => exit(s.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("ergctl: failed to re-exec under sudo: {e}");
            exit(1);
        }
    }
}
