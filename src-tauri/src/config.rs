use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

static CONFIG: OnceLock<NotchConfig> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotchConfig {
    pub animation: AnimationConfig,
    pub dimensions: DimensionsConfig,
    pub hover: HoverConfig,
    pub window: WindowConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationConfig {
    pub expand_duration: ConfigValue<f64>,
    pub collapse_duration: ConfigValue<f64>,
    pub expand_timing: ConfigValue<Vec<f64>>,
    pub collapse_timing: ConfigValue<Vec<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionsConfig {
    pub corner_radius: ConfigValue<f64>,
    pub collapsed_width: ConfigValue<f64>,
    pub collapsed_height: ConfigValue<f64>,
    pub expanded_width: ConfigValue<f64>,
    pub expanded_height: ConfigValue<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoverConfig {
    pub collapsed_zone_width: ConfigValue<f64>,
    pub collapsed_zone_height: ConfigValue<f64>,
    pub expanded_zone_width: ConfigValue<f64>,
    pub expanded_zone_height: ConfigValue<f64>,
    pub expand_delay_ms: ConfigValue<u64>,
    pub collapse_delay_ms: ConfigValue<u64>,
    pub poll_interval_ms: ConfigValue<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    pub level_offset: ConfigValue<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigValue<T> {
    pub value: T,
    pub description: String,
}

impl NotchConfig {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        // Try multiple paths
        let paths = vec![
            // Current directory (dev mode)
            std::env::current_dir().ok().map(|p| p.join("notch-config.json")),
            // Relative to executable (production)
            std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("../notch-config.json"))),
            // Workspace root (dev mode with cargo)
            std::env::var("CARGO_MANIFEST_DIR").ok().map(|p| std::path::PathBuf::from(p).join("../notch-config.json")),
        ];
        
        for path in paths.into_iter().flatten() {
            if let Ok(config_str) = std::fs::read_to_string(&path) {
                eprintln!("Loading config from: {}", path.display());
                let config: NotchConfig = serde_json::from_str(&config_str)?;
                return Ok(config);
            }
        }
        
        Err("Config file not found in any expected location".into())
    }

    pub fn get() -> &'static NotchConfig {
        CONFIG.get_or_init(|| {
            Self::load().unwrap_or_else(|e| {
                eprintln!("Failed to load config, using defaults: {}", e);
                Self::default()
            })
        })
    }
}

impl Default for NotchConfig {
    fn default() -> Self {
        Self {
            animation: AnimationConfig {
                expand_duration: ConfigValue {
                    value: 0.50,
                    description: "Duration in seconds for the expand animation".to_string(),
                },
                collapse_duration: ConfigValue {
                    value: 0.35,
                    description: "Duration in seconds for the collapse animation".to_string(),
                },
                expand_timing: ConfigValue {
                    value: vec![0.16, 1.0, 0.3, 1.0],
                    description: "Cubic bezier control points for expand animation".to_string(),
                },
                collapse_timing: ConfigValue {
                    value: vec![0.25, 0.1, 0.25, 1.0],
                    description: "Cubic bezier control points for collapse animation".to_string(),
                },
            },
            dimensions: DimensionsConfig {
                corner_radius: ConfigValue {
                    value: 12.0,
                    description: "Corner radius in points".to_string(),
                },
                collapsed_width: ConfigValue {
                    value: 460.0,
                    description: "Width when collapsed".to_string(),
                },
                collapsed_height: ConfigValue {
                    value: 50.0,
                    description: "Height when collapsed".to_string(),
                },
                expanded_width: ConfigValue {
                    value: 700.0,
                    description: "Width when expanded".to_string(),
                },
                expanded_height: ConfigValue {
                    value: 200.0,
                    description: "Height when expanded".to_string(),
                },
            },
            hover: HoverConfig {
                collapsed_zone_width: ConfigValue {
                    value: 460.0,
                    description: "Hover zone width when collapsed".to_string(),
                },
                collapsed_zone_height: ConfigValue {
                    value: 50.0,
                    description: "Hover zone height when collapsed".to_string(),
                },
                expanded_zone_width: ConfigValue {
                    value: 700.0,
                    description: "Hover zone width when expanded".to_string(),
                },
                expanded_zone_height: ConfigValue {
                    value: 200.0,
                    description: "Hover zone height when expanded".to_string(),
                },
                expand_delay_ms: ConfigValue {
                    value: 250,
                    description: "Milliseconds to wait before expanding when hovering over the notch area".to_string(),
                },
                collapse_delay_ms: ConfigValue {
                    value: 150,
                    description: "Milliseconds to wait before collapsing when leaving the hover area".to_string(),
                },
                poll_interval_ms: ConfigValue {
                    value: 50,
                    description: "Mouse polling interval in milliseconds".to_string(),
                },
            },
            window: WindowConfig {
                level_offset: ConfigValue {
                    value: 3,
                    description: "Window level offset above main menu".to_string(),
                },
            },
        }
    }
}

