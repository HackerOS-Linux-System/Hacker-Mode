mod core;
mod drm;
mod xwayland;
mod ipc;

use anyhow::Result;
use tracing::info;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    // Logging
    tracing_subscriber::fmt()
    .with_env_filter(
        EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,hm_compositor=debug,smithay=warn")),
    )
    .init();

    info!("HackerOS Hacker Mode compositor starting");

    // Run the compositor on a single-threaded tokio runtime for calloop compat
    let rt = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()?;

    rt.block_on(async { core::run().await })
}
