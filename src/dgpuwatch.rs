//! dGPU wake watcher — traces the kernel `rpm:rpm_resume` tracepoint for the dGPU
//! via bpftrace and records the waking process to /run/ergctl/dgpu-waker (shown in
//! the TUI / status) plus a rolling log. Needs root + bpftrace; runs as the
//! ergctl-dgpu-watch.service. The recorded comm/pid is whoever the kernel resumed
//! the device for — usually the culprit, occasionally a kworker (async/driver wake).

use crate::sysfs;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

pub const WAKER_FILE: &str = "/run/ergctl/dgpu-waker";
const LOG_FILE: &str = "/var/log/ergctl-dgpu-wakes.log";

/// PCI id of the dGPU as the rpm tracepoint reports it, e.g. "0000:64:00.0".
fn dgpu_id() -> &'static str {
    sysfs::DGPU_PCI.trim_start_matches("/sys/bus/pci/devices/")
}

/// The last process that woke the dGPU, if the watcher has recorded one.
pub fn last_waker() -> Option<String> {
    sysfs::read_trim(WAKER_FILE).filter(|s| !s.is_empty())
}

/// Run the watcher (blocks): spawn bpftrace on the dGPU's rpm_resume, and on every
/// wake write "<comm> (<pid>)" to WAKER_FILE and append a timestamped line to the log.
pub fn watch() {
    let _ = fs::create_dir_all("/run/ergctl");
    let script = format!(
        "tracepoint:rpm:rpm_resume /str(args->name) == \"{}\"/ \
         {{ printf(\"WAKE\\t%d\\t%s\\n\", pid, comm); }}",
        dgpu_id()
    );
    let mut child = match Command::new("bpftrace")
        .arg("-e")
        .arg(&script)
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("ergctl: cannot start bpftrace ({e}); install bpftrace and run as root");
            return;
        }
    };
    let Some(stdout) = child.stdout.take() else {
        return;
    };
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        // Only our own "WAKE\t<pid>\t<comm>" lines; ignore bpftrace's banner.
        let parts: Vec<&str> = line.trim().splitn(3, '\t').collect();
        if parts.len() == 3 && parts[0] == "WAKE" {
            let entry = format!("{} ({})", parts[2], parts[1]);
            let _ = fs::write(WAKER_FILE, &entry);
            if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(LOG_FILE) {
                let _ = writeln!(f, "{entry}");
            }
        }
    }
}
