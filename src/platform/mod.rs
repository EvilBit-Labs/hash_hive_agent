#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

use crate::api::types::DeviceInfo;

/// Collect current system metrics (CPU, memory, temperature).
pub fn collect_device_info() -> DeviceInfo {
    let sys = sysinfo::System::new_all();

    let cpu_usage = sys.global_cpu_usage() as f64;

    let total_mem = sys.total_memory() as f64;
    let used_mem = sys.used_memory() as f64;
    let memory_usage = if total_mem > 0.0 {
        used_mem / total_mem * 100.0
    } else {
        0.0
    };

    DeviceInfo {
        cpu_usage,
        memory_usage,
        temperature: gpu_temperature(),
    }
}

/// Return the platform identifier string matching the server's expectations.
pub fn platform_name() -> &'static str {
    #[cfg(target_os = "linux")]
    {
        "linux"
    }
    #[cfg(target_os = "macos")]
    {
        "darwin"
    }
    #[cfg(target_os = "windows")]
    {
        "windows"
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        "unknown"
    }
}

/// Attempt to read GPU temperature. Returns `None` if unavailable.
fn gpu_temperature() -> Option<f64> {
    // GPU temperature reading is platform-specific and often requires
    // nvidia-smi, rocm-smi, or similar tooling. Stubbed for now.
    None
}
