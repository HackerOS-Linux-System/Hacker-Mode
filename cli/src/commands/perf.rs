use anyhow::Result;

pub async fn run() -> Result<()> {
    let mon = hm_core::perf::PerfMonitor::new();
    let snap = mon.snapshot();
    println!("CPU:  {:.1}%", snap.cpu_pct);
    println!("RAM:  {} MB / {} MB", snap.ram_used_mb, snap.ram_total_mb);
    if let Some(gpu) = snap.gpu_pct  { println!("GPU:  {gpu:.1}%"); }
    if let Some(fps) = snap.fps      { println!("FPS:  {fps}"); }
    if let Some(t)   = snap.gpu_temp_c { println!("GPU temp: {t}°C"); }
    if let Some(t)   = snap.cpu_temp_c { println!("CPU temp: {t}°C"); }
    Ok(())
}
