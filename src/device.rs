use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub identifier: String,
    pub status: DeviceStatus,
    pub product: String,
    pub model: String,
    pub device: String,
    pub transport_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceStatus {
    Device,
    Offline,
    Unauthorized,
    NoPermission,
    Unknown(String),
}

impl From<&str> for DeviceStatus {
    fn from(s: &str) -> Self {
        match s {
            "device" => DeviceStatus::Device,
            "offline" => DeviceStatus::Offline,
            "unauthorized" => DeviceStatus::Unauthorized,
            "no_permission" => DeviceStatus::NoPermission,
            _ => DeviceStatus::Unknown(s.to_string()),
        }
    }
}

impl Device {
    pub fn is_usable(&self) -> bool {
        matches!(self.status, DeviceStatus::Device)
    }

    pub fn get_dimensions(&self, adb_path: &str) -> Result<Option<(u32, u32)>> {
        let output = Command::new(adb_path)
            .args(["-s", &self.identifier, "shell", "wm", "size"])
            .output()?;

        if !output.status.success() {
            return Ok(None);
        }

        let output_str = String::from_utf8_lossy(&output.stdout);

        // Parse output like "Physical size: 1080x2400" or "Override size: 1080x2400"
        for line in output_str.lines() {
            if line.contains("Physical size:") || line.contains("Override size:") {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() == 2 {
                    let size_str = parts[1].trim();
                    let dimensions: Vec<&str> = size_str.split('x').collect();
                    if dimensions.len() == 2 {
                        if let (Ok(width), Ok(height)) =
                            (dimensions[0].parse::<u32>(), dimensions[1].parse::<u32>())
                        {
                            return Ok(Some((width, height)));
                        }
                    }
                }
            }
        }

        Ok(None)
    }
}

pub fn get_devices(adb_path: &str) -> Result<Vec<Device>> {
    let output = Command::new(adb_path).args(["devices", "-l"]).output()?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed to execute adb devices"));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut devices = Vec::new();

    for line in output_str.lines().skip(1) {
        if line.trim().is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }

        let identifier = parts[0].to_string();
        let status = if parts.len() > 1 && parts[1] == "no_permission" {
            DeviceStatus::NoPermission
        } else {
            DeviceStatus::from(parts[1])
        };

        let product = parts
            .iter()
            .find(|&&p| p.starts_with("product:"))
            .map(|p| p.split(':').nth(1).unwrap_or("unknown").to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let model = parts
            .iter()
            .find(|&&p| p.starts_with("model:"))
            .map(|p| p.split(':').nth(1).unwrap_or("unknown").to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let device = parts
            .iter()
            .find(|&&p| p.starts_with("device:"))
            .map(|p| p.split(':').nth(1).unwrap_or("unknown").to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let transport_id = parts
            .iter()
            .find(|&&p| p.starts_with("transport_id:"))
            .map(|p| p.split(':').nth(1).unwrap_or("unknown").to_string())
            .unwrap_or_else(|| "unknown".to_string());

        devices.push(Device {
            identifier,
            status,
            product,
            model,
            device,
            transport_id,
        });
    }

    Ok(devices)
}

pub fn restart_adb_server(adb_path: &str) -> Result<()> {
    let status = Command::new(adb_path).arg("kill-server").status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("Failed to kill ADB server"));
    }

    let status = Command::new(adb_path).arg("start-server").status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("Failed to start ADB server"));
    }

    Ok(())
}
