use anyhow::Result;
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub adb_path: Option<String>,
    pub scrcpy_path: Option<String>,
    pub bitrate: String,
    pub orientation: Option<String>,
    pub show_touches: bool,
    pub turn_screen_off: bool,
    pub fullscreen: bool,
    pub dimension: Option<u32>,
    pub extra_args: String,
    pub force_adb_forward: bool,
    pub panels: PanelConfig,
    pub theme: String,
    pub wireless_adb: WirelessAdbConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelConfig {
    pub swipe: bool,
    pub toolkit: bool,
    pub bottom: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WirelessAdbConfig {
    pub last_tcpip_ip: String,
    pub last_tcpip_port: String,
    pub last_pairing_ip: String,
    pub last_pairing_port: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            adb_path: None,
            scrcpy_path: None,
            bitrate: "8M".to_string(),
            orientation: None,
            show_touches: false,
            turn_screen_off: false,
            fullscreen: false,
            dimension: None,
            extra_args: String::new(),
            force_adb_forward: false,
            panels: PanelConfig {
                swipe: true,
                toolkit: true,
                bottom: true,
            },
            theme: "default".to_string(),
            wireless_adb: WirelessAdbConfig {
                last_tcpip_ip: String::new(),
                last_tcpip_port: "5555".to_string(),
                last_pairing_ip: String::new(),
                last_pairing_port: "5555".to_string(),
            },
        }
    }
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content = fs::read_to_string(config_path)?;
            let config: AppConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        // Ensure config directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        fs::write(config_path, content)?;
        Ok(())
    }

    fn config_path() -> Result<PathBuf> {
        let mut path =
            config_dir().ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
        path.push("DroidView");
        path.push("config.toml");
        Ok(path)
    }
}
