//! Read-only local observations. A snapshot is not a reservation or backend test.
use crate::error::{GoblinError, Result};
use serde::{Deserialize, Serialize};
use std::{fs, io::Read, path::Path};

pub const SCHEMA: &str = "goblin.ai-hardware.v1";
pub const MAX_BYTES: u64 = 1 << 52;
pub const MAX_DEVICES: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Cpu {
    pub usable_logical_threads: u32,
    pub instruction_features: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MemoryLimit {
    pub source: String,
    pub limit_bytes: u64,
    pub current_bytes: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub api: String,
    pub unified_memory: bool,
    pub total_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
    pub recommended_working_set_bytes: Option<u64>,
    pub compute_capability: Option<[u32; 2]>,
    pub backend_execution_tested: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Hardware {
    pub schema: String,
    pub observed_utc: String,
    pub os: String,
    pub architecture: String,
    pub cpu: Cpu,
    pub total_ram_bytes: Option<u64>,
    pub available_ram_bytes: Option<u64>,
    pub memory_limits: Vec<MemoryLimit>,
    pub available_disk_bytes: Option<u64>,
    pub devices: Vec<Device>,
    pub observations: Vec<String>,
}

fn error() -> GoblinError {
    GoblinError::new("G910", "Invalid or unsupported AI hardware observation.")
}
pub fn bounded_text(path: &Path) -> Option<String> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .ok()?
        .take(65_537)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() > 65_536 {
        return None;
    }
    String::from_utf8(bytes).ok()
}
pub fn validate(h: &Hardware) -> Result<()> {
    let within = |v: Option<u64>| v.is_none_or(|v| v <= MAX_BYTES);
    if h.schema != SCHEMA
        || chrono::DateTime::parse_from_rfc3339(&h.observed_utc).is_err()
        || h.os.len() > 32
        || h.architecture.len() > 32
        || h.cpu.usable_logical_threads > 65_536
        || h.cpu.instruction_features.len() > 32
        || h.cpu.instruction_features.iter().any(|s| s.len() > 40)
        || !within(h.total_ram_bytes)
        || !within(h.available_ram_bytes)
        || !within(h.available_disk_bytes)
        || matches!((h.total_ram_bytes, h.available_ram_bytes), (Some(t), Some(a)) if a > t)
        || h.memory_limits.len() > 128
        || h.devices.len() > MAX_DEVICES
        || h.observations.len() > 64
        || h.observations.iter().any(|s| s.len() > 256)
        || h.memory_limits
            .iter()
            .any(|m| m.source.len() > 96 || m.limit_bytes > MAX_BYTES || !within(m.current_bytes))
        || h.devices.iter().any(|d| {
            d.id.len() > 64
                || d.name.len() > 128
                || d.api.len() > 32
                || !within(d.total_bytes)
                || !within(d.available_bytes)
                || !within(d.recommended_working_set_bytes)
                || matches!((d.total_bytes,d.available_bytes), (Some(t),Some(a)) if a > t)
                || d.backend_execution_tested
        })
    {
        return Err(error());
    }
    let mut ids = std::collections::BTreeSet::new();
    if h.devices.iter().any(|d| !ids.insert(&d.id)) {
        return Err(error());
    }
    Ok(())
}

pub fn probe(workspace: &Path) -> Result<Hardware> {
    let mut h = Hardware {
        schema: SCHEMA.into(), observed_utc: chrono::Utc::now().to_rfc3339(),
        os: std::env::consts::OS.into(), architecture: std::env::consts::ARCH.into(),
        cpu: Cpu { usable_logical_threads: std::thread::available_parallelism().ok()
            .and_then(|n| u32::try_from(n.get()).ok()).unwrap_or(0), instruction_features: features() },
        total_ram_bytes: None, available_ram_bytes: None, memory_limits: Vec::new(),
        available_disk_bytes: disk_available(workspace), devices: Vec::new(), observations: vec![
            "Live estimates only; no resources reserved, stress test, model load or network access.".into(),
            "CPU features are observed, not a validated training-kernel dispatch table.".into(),
            "System clock is not a certified timestamp. External workload changes can invalidate this snapshot.".into(),
        ],
    };
    #[cfg(target_os = "linux")]
    linux(&mut h);
    #[cfg(target_os = "macos")]
    macos(&mut h);
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    h.observations.push(
        "Memory and GPU probing are unsupported on this platform; unknown is not zero.".into(),
    );
    validate(&h)?;
    Ok(h)
}
fn features() -> Vec<String> {
    let mut features = Vec::new();
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    for (name, detected) in [
        ("sse2", std::is_x86_feature_detected!("sse2")),
        ("avx", std::is_x86_feature_detected!("avx")),
        ("avx2", std::is_x86_feature_detected!("avx2")),
        ("fma", std::is_x86_feature_detected!("fma")),
        ("avx512f", std::is_x86_feature_detected!("avx512f")),
    ] {
        if detected {
            features.push(name.into());
        }
    }
    #[cfg(target_arch = "aarch64")]
    for (name, detected) in [
        ("neon", std::arch::is_aarch64_feature_detected!("neon")),
        ("fp16", std::arch::is_aarch64_feature_detected!("fp16")),
        ("bf16", std::arch::is_aarch64_feature_detected!("bf16")),
    ] {
        if detected {
            features.push(name.into());
        }
    }
    features
}
#[cfg(unix)]
fn disk_available(path: &Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt;
    let name = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: CString is NUL terminated; statvfs initializes stat on success.
    if unsafe { libc::statvfs(name.as_ptr(), stat.as_mut_ptr()) } != 0 {
        return None;
    }
    let stat = unsafe { stat.assume_init() };
    u64::try_from(u128::from(stat.f_bavail) * u128::from(stat.f_frsize))
        .ok()
        .filter(|n| *n <= MAX_BYTES)
}
#[cfg(not(unix))]
fn disk_available(_: &Path) -> Option<u64> {
    None
}

pub fn meminfo(text: &str) -> Option<(u64, u64)> {
    fn item(text: &str, key: &str) -> Option<u64> {
        let mut entries = text.lines().filter(|l| l.starts_with(key));
        let fields = entries.next()?.split_whitespace().collect::<Vec<_>>();
        if entries.next().is_some() || fields.len() != 3 || fields[0] != key || fields[2] != "kB" {
            return None;
        }
        fields[1]
            .parse::<u64>()
            .ok()?
            .checked_mul(1024)
            .filter(|n| *n <= MAX_BYTES)
    }
    let t = item(text, "MemTotal:")?;
    let a = item(text, "MemAvailable:")?;
    (a <= t).then_some((t, a))
}

/// Visible cgroup v2 ancestors, including memory.high as an operator throttle cap.
/// No cgroup paths/identities are emitted. Failure to read a finite cap's usage
/// keeps an unknown remaining capacity, which refuses optimistic admission.
pub fn cgroup_limits(mount: &Path, relative: &Path) -> Option<Vec<MemoryLimit>> {
    use std::path::Component;
    if relative
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return None;
    }
    let mut current = mount.join(relative);
    if !current.is_dir() {
        return None;
    }
    let mut limits = Vec::new();
    for depth in 0..64 {
        if !current.is_dir() || current != mount && !current.join("memory.max").exists() {
            return None;
        }
        let usage = bounded_text(&current.join("memory.current"))
            .and_then(|s| s.trim().parse::<u64>().ok());
        for field in ["memory.max", "memory.high"] {
            let file = current.join(field);
            if file.exists() {
                let text = bounded_text(&file)?;
                if text.trim() != "max" {
                    let limit = text.trim().parse::<u64>().ok()?;
                    if limit > MAX_BYTES {
                        return None;
                    }
                    limits.push(MemoryLimit {
                        source: format!("cgroup-v2-ancestor-{depth}.{field}"),
                        limit_bytes: limit,
                        current_bytes: usage,
                    });
                }
            }
        }
        if current == mount {
            return Some(limits);
        }
        if !current.pop() || !current.starts_with(mount) {
            return None;
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn linux(h: &mut Hardware) {
    if let Some((t, a)) = bounded_text(Path::new("/proc/meminfo")).and_then(|s| meminfo(&s)) {
        h.total_ram_bytes = Some(t);
        h.available_ram_bytes = Some(a);
    }
    let membership = bounded_text(Path::new("/proc/self/cgroup"));
    let v2 = membership
        .as_ref()
        .and_then(|s| s.lines().find_map(|l| l.strip_prefix("0::")));
    if let Some(v2) = v2 {
        let relative = Path::new(v2.trim_start_matches('/'));
        if Path::new("/sys/fs/cgroup/cgroup.controllers").is_file() {
            if let Some(limits) = cgroup_limits(Path::new("/sys/fs/cgroup"), relative) {
                h.memory_limits.extend(limits);
            } else {
                h.available_ram_bytes = None;
                h.observations.push(
                    "Cgroup v2 memory limits could not be resolved; RAM availability is unknown."
                        .into(),
                );
            }
        } else {
            h.available_ram_bytes = None;
            h.observations
                .push("Nonstandard cgroup mount: remaining memory capacity is unknown.".into());
        }
        h.observations.push("Only visible cgroup v2 ancestors are observed; hidden host/VM/scheduler constraints are not certified.".into());
    } else if membership.as_ref().is_none_or(|s| !s.trim().is_empty()) {
        h.available_ram_bytes = None;
        h.observations.push(
            "Legacy/unreadable cgroup constraints: remaining memory capacity is unknown.".into(),
        );
    }
    // RLIMIT_AS is virtual address space, not physical RAM. Account separately.
    let mut limit = std::mem::MaybeUninit::<libc::rlimit>::uninit();
    if unsafe { libc::getrlimit(libc::RLIMIT_AS, limit.as_mut_ptr()) } == 0 {
        let limit = unsafe { limit.assume_init() }.rlim_cur;
        if limit != libc::RLIM_INFINITY && limit <= MAX_BYTES {
            let used = bounded_text(Path::new("/proc/self/status")).and_then(|s| {
                let line = s.lines().find(|l| l.starts_with("VmSize:"))?;
                line.split_whitespace()
                    .nth(1)?
                    .parse::<u64>()
                    .ok()?
                    .checked_mul(1024)
            });
            h.memory_limits.push(MemoryLimit {
                source: "process-rlimit-as".into(),
                limit_bytes: limit,
                current_bytes: used,
            });
        }
    }
    h.devices = linux_gpus();
    h.observations.push("Linux GPU observations use bounded read-only sysfs records; missing VRAM/driver capability remains unknown. No CUDA/ROCm kernel was executed.".into());
}
#[cfg(target_os = "linux")]
fn linux_gpus() -> Vec<Device> {
    let mut devices = Vec::new();
    let Ok(entries) = fs::read_dir("/sys/class/drm") else {
        return devices;
    };
    let mut cards = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name().and_then(|s| s.to_str()).is_some_and(|s| {
                s.strip_prefix("card")
                    .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            })
        })
        .collect::<Vec<_>>();
    cards.sort();
    for card in cards.into_iter().take(MAX_DEVICES) {
        let root = card.join("device");
        let Some(vendor) = bounded_text(&root.join("vendor")) else {
            continue;
        };
        let read = |name| {
            bounded_text(&root.join(name))
                .and_then(|s| s.trim().parse::<u64>().ok())
                .filter(|v| *v <= MAX_BYTES)
        };
        let total = read("mem_info_vram_total");
        let used = read("mem_info_vram_used");
        let id = card.file_name().unwrap().to_string_lossy().into_owned();
        devices.push(Device {
            id,
            name: match vendor.trim() {
                "0x10de" => "NVIDIA (driver capability untested)",
                "0x1002" => "AMD (driver capability untested)",
                "0x8086" => "Intel (driver capability untested)",
                _ => "DRM device (vendor unvalidated)",
            }
            .into(),
            api: "drm-sysfs".into(),
            unified_memory: false,
            total_bytes: total,
            available_bytes: total.zip(used).and_then(|(t, u)| t.checked_sub(u)),
            recommended_working_set_bytes: None,
            compute_capability: None,
            backend_execution_tested: false,
        });
    }
    devices
}

#[cfg(target_os = "macos")]
fn macos(h: &mut Hardware) {
    use std::ffi::{CString, c_void};
    fn sys_u64(key: &str) -> Option<u64> {
        let key = CString::new(key).ok()?;
        let mut value = 0u64;
        let mut size = 8usize;
        let status = unsafe {
            libc::sysctlbyname(
                key.as_ptr(),
                (&mut value as *mut u64).cast(),
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        };
        (status == 0 && size == 8 && value <= MAX_BYTES).then_some(value)
    }
    h.total_ram_bytes = sys_u64("hw.memsize");
    // Mach returns a counted array of 32-bit integers. Only the documented
    // free/active/inactive/wired prefix is read; the aligned buffer has spare capacity.
    unsafe extern "C" {
        fn mach_host_self() -> u32;
        fn host_statistics64(host: u32, flavor: i32, info: *mut i32, count: *mut u32) -> i32;
        fn host_page_size(host: u32, size: *mut libc::vm_size_t) -> i32;
        fn mach_port_deallocate(task: u32, name: u32) -> i32;
        static mach_task_self_: u32;
    }
    let host = unsafe { mach_host_self() };
    let mut buffer = [0u64; 64];
    let mut count = 128;
    let mut page: libc::vm_size_t = 0;
    if unsafe { host_page_size(host, &mut page) } == 0
        && unsafe { host_statistics64(host, 4, buffer.as_mut_ptr().cast(), &mut count) } == 0
        && (4..=128).contains(&count)
    {
        let words = buffer.as_ptr().cast::<u32>();
        let free = unsafe { *words };
        let inactive = unsafe { *words.add(2) };
        h.available_ram_bytes = (u64::from(free) + u64::from(inactive))
            .checked_mul(page as u64)
            .map(|v| v.min(h.total_ram_bytes.unwrap_or(v)));
    }
    unsafe { mach_port_deallocate(mach_task_self_, host) };
    h.observations.push("macOS available RAM estimate = free + inactive pages; compressed/purgeable pools are not added again. It is not guaranteed allocatable memory.".into());
    h.observations.push("Only the default Metal device is observed. MLX availability/operator support has not been tested.".into());
    #[link(name = "Metal", kind = "framework")]
    unsafe extern "C" {
        fn MTLCreateSystemDefaultDevice() -> *mut c_void;
    }
    #[link(name = "objc")]
    unsafe extern "C" {
        fn sel_registerName(name: *const std::ffi::c_char) -> *mut c_void;
        fn objc_msgSend();
        fn objc_release(object: *mut c_void);
    }
    unsafe fn supports(object: *mut c_void, selector: &std::ffi::CStr) -> bool {
        let send: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> i8 =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        unsafe {
            send(
                object,
                sel_registerName(c"respondsToSelector:".as_ptr()),
                sel_registerName(selector.as_ptr()),
            ) != 0
        }
    }
    unsafe fn number(object: *mut c_void, selector: &std::ffi::CStr) -> Option<u64> {
        if !unsafe { supports(object, selector) } {
            return None;
        }
        let send: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u64 =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        Some(unsafe { send(object, sel_registerName(selector.as_ptr())) })
    }
    unsafe fn unified(object: *mut c_void) -> bool {
        if !unsafe { supports(object, c"hasUnifiedMemory") } {
            return false;
        }
        let send: unsafe extern "C" fn(*mut c_void, *mut c_void) -> i8 =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        unsafe { send(object, sel_registerName(c"hasUnifiedMemory".as_ptr())) != 0 }
    }
    let device = unsafe { MTLCreateSystemDefaultDevice() };
    if !device.is_null() {
        let unified = unsafe { unified(device) };
        let recommended = unsafe { number(device, c"recommendedMaxWorkingSetSize") };
        h.devices.push(Device {
            id: "metal:default".into(),
            name: if unified {
                "Default unified-memory Metal device"
            } else {
                "Default discrete Metal device"
            }
            .into(),
            api: "metal".into(),
            unified_memory: unified,
            total_bytes: if unified { h.total_ram_bytes } else { None },
            available_bytes: None,
            recommended_working_set_bytes: recommended.filter(|v| *v <= MAX_BYTES),
            compute_capability: None,
            backend_execution_tested: false,
        });
        unsafe { objc_release(device) };
    } else {
        h.observations.push("No accessible default Metal device was returned. This does not establish that the computer has no GPU.".into());
    }
}
