//! macOS-only memory breakdown via Mach host statistics (Activity Monitor style).
#![allow(deprecated)]

use libc::{c_void, host_statistics64, mach_host_self, sysctlbyname, vm_statistics64};

/// Memory split in bytes, matching Activity Monitor's categories.
#[derive(Debug, Clone, Copy, Default)]
pub struct VmStat {
    pub app: u64,
    pub wired: u64,
    pub compressed: u64,
    pub cached: u64,
    pub free: u64,
}

impl VmStat {
    /// Used = app + wired + compressed, same formula Activity Monitor shows.
    pub fn used(&self) -> u64 {
        self.app + self.wired + self.compressed
    }
}

/// Memory pressure as reported by the kernel memorystatus subsystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pressure {
    Normal,
    Warning,
    Critical,
    Unknown,
}

impl Pressure {
    pub fn label(self) -> &'static str {
        match self {
            Pressure::Normal => "Normal",
            Pressure::Warning => "Warning",
            Pressure::Critical => "Critical",
            Pressure::Unknown => "?",
        }
    }
}

/// Read host VM statistics; None if the Mach call fails.
pub fn vm_stat() -> Option<VmStat> {
    // SAFETY: plain Mach FFI with a correctly sized out-struct and count.
    unsafe {
        let page = libc::sysconf(libc::_SC_PAGESIZE);
        if page <= 0 {
            return None;
        }
        let page = page as u64;
        let mut info: vm_statistics64 = std::mem::zeroed();
        let mut count = libc::HOST_VM_INFO64_COUNT;
        let kr = host_statistics64(
            mach_host_self(),
            libc::HOST_VM_INFO64,
            &mut info as *mut vm_statistics64 as libc::host_info64_t,
            &mut count,
        );
        if kr != libc::KERN_SUCCESS {
            return None;
        }
        let pages = |n: u32| u64::from(n) * page;
        Some(VmStat {
            app: pages(
                info.internal_page_count
                    .saturating_sub(info.purgeable_count),
            ),
            wired: pages(info.wire_count),
            compressed: pages(info.compressor_page_count),
            cached: pages(info.external_page_count) + pages(info.purgeable_count),
            free: pages(info.free_count.saturating_sub(info.speculative_count)),
        })
    }
}

/// Memory pressure level from kern.memorystatus_vm_pressure_level (1/2/4).
pub fn pressure() -> Pressure {
    let mut level: i32 = 0;
    let mut len = std::mem::size_of::<i32>();
    // SAFETY: sysctlbyname with a NUL-terminated name and correctly sized buffer.
    let rc = unsafe {
        sysctlbyname(
            c"kern.memorystatus_vm_pressure_level".as_ptr(),
            &mut level as *mut i32 as *mut c_void,
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    if rc != 0 {
        return Pressure::Unknown;
    }
    match level {
        1 => Pressure::Normal,
        2 => Pressure::Warning,
        4 => Pressure::Critical,
        _ => Pressure::Unknown,
    }
}
