use anyhow::Result;
use smithay::{
    output::{Mode, Output, PhysicalProperties, Subpixel},
    utils::Transform,
};
use tracing::{info, warn};

use crate::core::state::HackerModeState;

pub struct DrmOutput {
    pub output: Output,
}

pub fn init(state: &mut HackerModeState) -> Result<()> {
    info!("Initialising DRM/KMS backend");
    let nodes = discover_drm_nodes();
    if nodes.is_empty() {
        warn!("No DRM devices found — using virtual fallback output");
        add_output(state, "virtual", "Virtual", "Fallback");
        return Ok(());
    }
    for path in nodes {
        let name = path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "card0".to_owned());
        let make = detect_gpu_make(&path);
        add_output(state, &name, &make, "Display");
    }
    Ok(())
}

fn add_output(state: &mut HackerModeState, name: &str, make: &str, model: &str) {
    let output = Output::new(
        name.to_owned(),
        PhysicalProperties {
            size:          (0, 0).into(),
            subpixel:      Subpixel::Unknown,
            make:          make.to_owned(),
            model:         model.to_owned(),
            serial_number: String::new(),
        },
    );

    let mode = Mode { size: (1920, 1080).into(), refresh: 60_000 };
    output.add_mode(mode);
    output.set_preferred(mode);
    output.change_current_state(
        Some(mode), Some(Transform::Normal), None, Some((0, 0).into()),
    );

    output.create_global::<HackerModeState>(&state.display_handle);
    state.space.map_output(&output, (0, 0));
    info!("Created output: {name} ({make})");
    state.drm_outputs.push(DrmOutput { output });
}

fn discover_drm_nodes() -> Vec<std::path::PathBuf> {
    (0..8u32)
        .map(|i| std::path::PathBuf::from(format!("/dev/dri/card{i}")))
        .filter(|p| p.exists())
        .collect()
}

fn detect_gpu_make(path: &std::path::Path) -> String {
    let node = path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    if let Ok(v) = std::fs::read_to_string(format!("/sys/class/drm/{node}/device/vendor")) {
        return match v.trim() {
            "0x1002" => "AMD".to_owned(),
            "0x8086" => "Intel".to_owned(),
            "0x10de" => "NVIDIA".to_owned(),
            _        => "Unknown".to_owned(),
        };
    }
    "Unknown".to_owned()
}
