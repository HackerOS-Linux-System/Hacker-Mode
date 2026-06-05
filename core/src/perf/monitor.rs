use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerfSnapshot {
    pub cpu_pct:      f32,
    pub gpu_pct:      Option<f32>,
    pub ram_used_mb:  u64,
    pub ram_total_mb: u64,
    pub fps:          Option<u32>,
    pub gpu_temp_c:   Option<u32>,
    pub cpu_temp_c:   Option<u32>,
    pub vram_used_mb: Option<u64>,
}

// ── CPU ───────────────────────────────────────────────────────────────────────

struct CpuState {
    prev_idle:  u64,
    prev_total: u64,
}

impl CpuState {
    fn new() -> Self { Self { prev_idle: 0, prev_total: 0 } }

    fn read(&mut self) -> Option<f32> {
        let content = std::fs::read_to_string("/proc/stat").ok()?;
        let line    = content.lines().next()?;
        let fields: Vec<u64> = line
            .split_whitespace()
            .skip(1)
            .filter_map(|v| v.parse().ok())
            .collect();
        if fields.len() < 4 { return None; }

        let idle  = fields[3] + fields.get(4).copied().unwrap_or(0);
        let total: u64 = fields.iter().sum();
        let d_total = total.saturating_sub(self.prev_total);
        let d_idle  = idle.saturating_sub(self.prev_idle);
        self.prev_total = total;
        self.prev_idle  = idle;

        if d_total == 0 { return Some(0.0); }
        Some(100.0 * (1.0 - d_idle as f32 / d_total as f32))
    }
}

// ── RAM ───────────────────────────────────────────────────────────────────────

fn read_ram() -> (u64, u64) {
    let Ok(content) = std::fs::read_to_string("/proc/meminfo") else { return (0, 0); };
    let mut total = 0u64;
    let mut available = 0u64;
    for line in content.lines() {
        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("MemTotal:")     => { total     = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0); }
            Some("MemAvailable:") => { available = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0); }
            _ => {}
        }
    }
    let used = total.saturating_sub(available);
    (used / 1024, total / 1024) // kB → MB
}

// ── GPU ───────────────────────────────────────────────────────────────────────

enum GpuBackend { Amd, Nvidia, Unknown }

fn detect_gpu() -> GpuBackend {
    if std::path::Path::new("/sys/class/drm/card0/device/gpu_busy_percent").exists() {
        return GpuBackend::Amd;
    }
    // Check nvidia-smi in PATH
    if std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .any(|p| std::path::Path::new(p).join("nvidia-smi").exists())
    {
        return GpuBackend::Nvidia;
    }
    GpuBackend::Unknown
}

fn read_amd_gpu() -> (Option<f32>, Option<u32>, Option<u64>) {
    let base = "/sys/class/drm/card0/device";
    let busy: Option<f32> = std::fs::read_to_string(format!("{base}/gpu_busy_percent"))
        .ok().and_then(|s| s.trim().parse().ok());
    let temp: Option<u32> = std::fs::read_to_string(format!("{base}/hwmon/hwmon0/temp1_input"))
        .ok().and_then(|s| s.trim().parse::<u32>().ok()).map(|t| t / 1000);
    let vram: Option<u64> = std::fs::read_to_string(format!("{base}/mem_info_vram_used"))
        .ok().and_then(|s| s.trim().parse::<u64>().ok()).map(|b| b / 1024 / 1024);
    (busy, temp, vram)
}

fn read_nvidia_gpu() -> (Option<f32>, Option<u32>, Option<u64>) {
    let out = std::process::Command::new("nvidia-smi")
        .args(["--query-gpu=utilization.gpu,temperature.gpu,memory.used",
               "--format=csv,noheader,nounits"])
        .output();
    let Ok(out) = out else { return (None, None, None); };
    let s = String::from_utf8_lossy(&out.stdout);
    let parts: Vec<&str> = s.trim().split(',').collect();
    let busy: Option<f32> = parts.first().and_then(|v| v.trim().parse().ok());
    let temp: Option<u32> = parts.get(1).and_then(|v| v.trim().parse().ok());
    let vram: Option<u64> = parts.get(2).and_then(|v| v.trim().parse().ok());
    (busy, temp, vram)
}

fn read_cpu_temp() -> Option<u32> {
    for i in 0..10 {
        let path = format!("/sys/class/hwmon/hwmon{i}/temp1_input");
        if let Ok(v) = std::fs::read_to_string(&path) {
            if let Ok(t) = v.trim().parse::<u32>() {
                return Some(t / 1000);
            }
        }
    }
    None
}

// ── Public API ────────────────────────────────────────────────────────────────

pub struct PerfMonitor {
    cpu:         Mutex<CpuState>,
    gpu_backend: GpuBackend,
    fps:         Mutex<Option<u32>>,
}

impl PerfMonitor {
    pub fn new() -> Self {
        Self {
            cpu:         Mutex::new(CpuState::new()),
            gpu_backend: detect_gpu(),
            fps:         Mutex::new(None),
        }
    }

    pub fn snapshot(&self) -> PerfSnapshot {
        let cpu_pct = self.cpu.lock().unwrap().read().unwrap_or(0.0);
        let (ram_used, ram_total) = read_ram();
        let (gpu_pct, gpu_temp, vram_used) = match self.gpu_backend {
            GpuBackend::Amd    => read_amd_gpu(),
            GpuBackend::Nvidia => read_nvidia_gpu(),
            GpuBackend::Unknown => (None, None, None),
        };
        PerfSnapshot {
            cpu_pct,
            gpu_pct,
            ram_used_mb:  ram_used,
            ram_total_mb: ram_total,
            fps:          *self.fps.lock().unwrap(),
            gpu_temp_c:   gpu_temp,
            cpu_temp_c:   read_cpu_temp(),
            vram_used_mb: vram_used,
        }
    }

    pub fn update_fps(&self, fps: u32) {
        *self.fps.lock().unwrap() = Some(fps);
    }
}

impl Default for PerfMonitor {
    fn default() -> Self { Self::new() }
}
