use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub config_dir:       PathBuf,
    pub language:         String,
    pub theme:            ThemeConfig,
    pub compositor:       CompositorConfig,
    pub proton:           ProtonConfig,
    pub stores:           StoresConfig,
    pub night_mode:       NightModeConfig,
    pub perf_overlay:     PerfOverlayConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub name:          String,    // "hacker-gold" (default)
    pub accent_color:  String,    // hex, e.g. "#f5a623"
    pub bg_color:      String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositorConfig {
    pub vsync:          bool,
    pub hdr:            bool,
    pub adaptive_sync:  bool,
    pub xwayland:       bool,
    pub scaling:        f32,  // 1.0, 1.25, 1.5, 2.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtonConfig {
    pub default_version: Option<String>,
    pub force_proton:    bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoresConfig {
    pub epic_enabled:    bool,
    pub gog_enabled:     bool,
    pub amazon_enabled:  bool,
    pub itchio_enabled:  bool,
    pub lutris_enabled:  bool,
    pub auto_import:     bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NightModeConfig {
    pub enabled:        bool,
    pub auto_schedule:  bool,
    pub start_hour:     u8,
    pub end_hour:       u8,
    pub temperature_k:  u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfOverlayConfig {
    pub enabled:   bool,
    pub show_fps:  bool,
    pub show_gpu:  bool,
    pub show_cpu:  bool,
    pub show_ram:  bool,
    pub corner:    String,  // "top-left" | "top-right" | "bottom-left" | "bottom-right"
}

impl Default for Config {
    fn default() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("/etc"))
            .join("hackeros/hacker-mode");
        Self {
            config_dir: config_dir.clone(),
            language: "pl".to_owned(),
            theme: ThemeConfig {
                name:         "hacker-gold".to_owned(),
                accent_color: "#f5a623".to_owned(),
                bg_color:     "#0a0a08".to_owned(),
            },
            compositor: CompositorConfig {
                vsync:         true,
                hdr:           false,
                adaptive_sync: true,
                xwayland:      true,
                scaling:       1.0,
            },
            proton: ProtonConfig {
                default_version: None,
                force_proton:    false,
            },
            stores: StoresConfig {
                epic_enabled:   true,
                gog_enabled:    true,
                amazon_enabled: true,
                itchio_enabled: true,
                lutris_enabled: true,
                auto_import:    true,
            },
            night_mode: NightModeConfig {
                enabled:       false,
                auto_schedule: true,
                start_hour:    22,
                end_hour:       7,
                temperature_k: 3500,
            },
            perf_overlay: PerfOverlayConfig {
                enabled:   true,
                show_fps:  true,
                show_gpu:  true,
                show_cpu:  true,
                show_ram:  true,
                corner:    "top-right".to_owned(),
            },
        }
    }
}

impl Config {
    pub fn load_or_default(config_dir: &Path) -> Result<Self> {
        let path = config_dir.join("config.toml");
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let mut cfg: Config = toml::from_str(&content)?;
            cfg.config_dir = config_dir.to_path_buf();
            Ok(cfg)
        } else {
            let cfg = Config::default();
            cfg.save()?;
            Ok(cfg)
        }
    }

    pub fn save(&self) -> Result<()> {
        std::fs::create_dir_all(&self.config_dir)?;
        let path = self.config_dir.join("config.toml");
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn db_path(&self) -> PathBuf {
        dirs::data_dir()
            .unwrap_or_default()
            .join("hackeros/hacker-mode/library.db")
    }
}
