//! Periodic sampling of CPU, memory and disk into a plain Snapshot, with short history.

use std::collections::VecDeque;
use std::path::Path;

use sysinfo::{Disks, ProcessRefreshKind, ProcessesToUpdate, System};

use crate::mac::{self, Pressure, VmStat};

/// One mounted volume worth showing.
#[derive(Debug, Clone)]
pub struct DiskInfo {
    pub name: String,
    pub total: u64,
    pub available: u64,
}

impl DiskInfo {
    pub fn used(&self) -> u64 {
        self.total.saturating_sub(self.available)
    }
    pub fn used_pct(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.used() as f64 * 100.0 / self.total as f64
        }
    }
}

/// One of the busiest processes by CPU.
#[derive(Debug, Clone)]
pub struct TopProc {
    pub name: String,
    pub cpu: f32,
    pub mem: u64,
}

/// How many processes the CPU section lists, like iStat's "top processes".
pub const TOP_N: usize = 5;
/// History depth in samples; 60 × 2 s = two minutes of graph.
pub const HIST: usize = 60;

/// Memory split as fractions of total: [app, wired, compressed].
pub type MemFrac = [f32; 3];

/// Everything the menu renders, captured at one instant.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub cpu_total: f32,
    pub cores: Vec<f32>,
    pub load: [f64; 3],
    pub mem_total: u64,
    pub vm: VmStat,
    pub pressure: Option<Pressure>,
    pub swap_used: u64,
    pub swap_total: u64,
    pub disks: Vec<DiskInfo>,
    pub top: Vec<TopProc>,
    pub uptime_secs: u64,
    pub cpu_hist: Vec<f32>,
    pub mem_hist: Vec<MemFrac>,
}

impl Snapshot {
    pub fn mem_used_pct(&self) -> f64 {
        if self.mem_total == 0 {
            0.0
        } else {
            self.vm.used() as f64 * 100.0 / self.mem_total as f64
        }
    }
}

/// Holds sysinfo state between ticks so CPU deltas are meaningful.
pub struct Sampler {
    sys: System,
    disks: Disks,
    tick: u64,
    cpu_hist: VecDeque<f32>,
    mem_hist: VecDeque<MemFrac>,
}

/// Disk list is re-scanned only every Nth tick; sizes refresh every tick.
const DISK_RESCAN_EVERY: u64 = 5;

impl Sampler {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        refresh_procs(&mut sys);
        Self {
            sys,
            disks: Disks::new_with_refreshed_list(),
            tick: 0,
            cpu_hist: VecDeque::with_capacity(HIST),
            mem_hist: VecDeque::with_capacity(HIST),
        }
    }

    pub fn sample(&mut self) -> Snapshot {
        self.tick += 1;
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        refresh_procs(&mut self.sys);
        if self.tick % DISK_RESCAN_EVERY == 1 {
            self.disks.refresh(true);
        } else {
            for d in self.disks.list_mut() {
                d.refresh();
            }
        }
        let la = System::load_average();
        let vm_raw = mac::vm_stat();
        let vm = vm_raw.unwrap_or_else(|| fallback_vm(&self.sys));
        let total = self.sys.total_memory().max(1) as f32;
        let cpu_total = self.sys.global_cpu_usage();
        push_capped(&mut self.cpu_hist, cpu_total);
        push_capped(
            &mut self.mem_hist,
            [
                vm.app as f32 / total,
                vm.wired as f32 / total,
                vm.compressed as f32 / total,
            ],
        );
        Snapshot {
            cpu_total,
            cores: self.sys.cpus().iter().map(|c| c.cpu_usage()).collect(),
            load: [la.one, la.five, la.fifteen],
            mem_total: self.sys.total_memory(),
            vm,
            pressure: vm_raw.map(|_| mac::pressure()),
            swap_used: self.sys.used_swap(),
            swap_total: self.sys.total_swap(),
            disks: collect_disks(&self.disks),
            top: top_procs(&self.sys),
            uptime_secs: System::uptime(),
            cpu_hist: self.cpu_hist.iter().copied().collect(),
            mem_hist: self.mem_hist.iter().copied().collect(),
        }
    }
}

fn push_capped<T>(q: &mut VecDeque<T>, v: T) {
    if q.len() == HIST {
        q.pop_front();
    }
    q.push_back(v);
}

/// Only CPU and memory are refreshed; everything else about a process is skipped.
fn refresh_procs(sys: &mut System) {
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_cpu().with_memory(),
    );
}

/// Top N processes by CPU; cpu is percent of one core, as in Activity Monitor.
fn top_procs(sys: &System) -> Vec<TopProc> {
    let mut v: Vec<TopProc> = sys
        .processes()
        .values()
        .filter(|p| p.cpu_usage() > 0.0)
        .map(|p| TopProc {
            name: p.name().to_string_lossy().into_owned(),
            cpu: p.cpu_usage(),
            mem: p.memory(),
        })
        .collect();
    v.sort_by(|a, b| b.cpu.total_cmp(&a.cpu));
    v.truncate(TOP_N);
    v
}

/// If the Mach call fails, fold everything into "app" so totals still add up.
fn fallback_vm(sys: &System) -> VmStat {
    VmStat {
        app: sys.used_memory(),
        free: sys.free_memory(),
        ..Default::default()
    }
}

/// Keep root plus user-visible /Volumes mounts; drop APFS helper volumes.
fn collect_disks(disks: &Disks) -> Vec<DiskInfo> {
    let mut out: Vec<DiskInfo> = Vec::new();
    for d in disks.list() {
        let mp = d.mount_point();
        if !is_user_volume(mp) || d.total_space() == 0 {
            continue;
        }
        // Same APFS container can appear twice; keep the first mount of it.
        if out
            .iter()
            .any(|o| o.total == d.total_space() && o.available == d.available_space())
        {
            continue;
        }
        let name = d.name().to_string_lossy().trim().to_string();
        let name = if name.is_empty() {
            mp.display().to_string()
        } else {
            name
        };
        out.push(DiskInfo {
            name,
            total: d.total_space(),
            available: d.available_space(),
        });
    }
    out
}

fn is_user_volume(mp: &Path) -> bool {
    if mp == Path::new("/") {
        return true;
    }
    match mp.strip_prefix("/Volumes") {
        Ok(rest) => {
            let s = rest.to_string_lossy();
            !s.is_empty() && !s.starts_with('.') && !s.contains('/')
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_volumes_only() {
        assert!(is_user_volume(Path::new("/")));
        assert!(is_user_volume(Path::new("/Volumes/Backup")));
        assert!(!is_user_volume(Path::new("/Volumes/.timemachine")));
        assert!(!is_user_volume(Path::new("/Volumes/Backup/sub")));
        assert!(!is_user_volume(Path::new("/System/Volumes/Data")));
        assert!(!is_user_volume(Path::new("/private/var/vm")));
    }

    #[test]
    fn history_is_capped() {
        let mut q = VecDeque::new();
        for i in 0..(HIST + 5) {
            push_capped(&mut q, i as f32);
        }
        assert_eq!(q.len(), HIST);
        assert_eq!(q.front().copied(), Some(5.0));
    }
}
