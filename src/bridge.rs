use anyhow::Result;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use tokio::process::Command as TokioCommand;

pub struct AdbBridge {
    path: String,
}

pub struct ScrcpyBridge {
    path: String,
}

impl AdbBridge {
    pub fn new(path: String) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn get_devices(&self) -> Result<Vec<String>> {
        let output = Command::new(&self.path).args(["devices"]).output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to execute adb devices"));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let devices: Vec<String> = output_str
            .lines()
            .skip(1)
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 2 && parts[1] == "device" {
                    Some(parts[0].to_string())
                } else {
                    None
                }
            })
            .collect();

        Ok(devices)
    }

    pub fn shell(&self, command: &str, device_id: Option<&str>) -> Result<String> {
        let mut cmd = Command::new(&self.path);

        if let Some(device) = device_id {
            cmd.args(["-s", device]);
        }

        cmd.args(["shell", command]);

        let output = cmd.output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Shell command failed"));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub fn tcpip(&self, port: u16, device_id: Option<&str>) -> Result<()> {
        let mut cmd = Command::new(&self.path);

        if let Some(device) = device_id {
            cmd.args(["-s", device]);
        }

        cmd.args(["-d", "tcpip", &port.to_string()]);

        let status = cmd.status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("TCP/IP command failed"));
        }

        Ok(())
    }

    pub fn connect(&self, ip: &str, port: u16) -> Result<()> {
        let output = Command::new(&self.path)
            .args(["connect", &format!("{}:{}", ip, port)])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
            let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
            
            // Check for specific error patterns
            if stderr.contains("connection refused") || stdout.contains("connection refused") {
                return Err(anyhow::anyhow!(
                    "Connection refused: Unable to connect to {}:{}. Please check if:\n\
                    • The device is powered on and connected to the same network\n\
                    • The IP address {} is correct\n\
                    • Port {} is not blocked by firewall\n\
                    • ADB TCP/IP is enabled on the device (run 'adb tcpip 5555' on USB first)",
                    ip, port, ip, port
                ));
            } else if stderr.contains("no route to host") || stdout.contains("no route to host") {
                return Err(anyhow::anyhow!(
                    "No route to host: Cannot reach {}:{}. Please check if:\n\
                    • The IP address {} is correct\n\
                    • The device is on the same network\n\
                    • Your network allows the connection",
                    ip, port, ip
                ));
            } else if stderr.contains("timeout") || stdout.contains("timeout") {
                return Err(anyhow::anyhow!(
                    "Connection timeout: Unable to reach {}:{}. Please check if:\n\
                    • The device is powered on\n\
                    • The IP address {} is correct\n\
                    • The device is on the same network",
                    ip, port, ip
                ));
            } else if stderr.contains("already connected") || stdout.contains("already connected") {
                return Err(anyhow::anyhow!(
                    "Already connected to {}:{}",
                    ip, port
                ));
            } else {
                // Generic error with captured output for debugging
                let error_msg = if !stderr.is_empty() {
                    stderr
                } else if !stdout.is_empty() {
                    stdout
                } else {
                    "Unknown connection error".to_string()
                };
                
                return Err(anyhow::anyhow!(
                    "Failed to connect to {}:{} - {}",
                    ip, port, error_msg.trim()
                ));
            }
        }

        Ok(())
    }

    pub fn pair(&self, ip: &str, port: u16, pairing_code: &str) -> Result<()> {
        let status = Command::new(&self.path)
            .args(["pair", &format!("{}:{}", ip, port), pairing_code])
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("Pairing command failed"));
        }

        Ok(())
    }
}

impl ScrcpyBridge {
    pub fn new(path: String) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn start(&self, args: &[String]) -> Result<Child> {
        let mut cmd = Command::new(&self.path);
        cmd.args(args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Log the full command being executed for debugging
        tracing::info!("Starting scrcpy with path: {}", self.path);
        tracing::info!("Full command: {} {}", self.path, args.join(" "));

        // Log environment variables that might affect scrcpy
        if let Ok(path) = std::env::var("PATH") {
            tracing::info!("PATH environment: {}", path);
        }
        if let Ok(display_var) = std::env::var("DISPLAY") {
            tracing::info!("DISPLAY environment: {}", display_var);
        }
        if let Ok(wayland_display) = std::env::var("WAYLAND_DISPLAY") {
            tracing::info!("WAYLAND_DISPLAY environment: {}", wayland_display);
        }

        let mut child = cmd.spawn()?;

        // Wait a moment to see if the process exits immediately
        std::thread::sleep(std::time::Duration::from_millis(500));

        // Check if the process is still running
        match child.try_wait() {
            Ok(Some(status)) => {
                tracing::error!(
                    "Scrcpy process exited immediately with status: {:?}",
                    status
                );

                // Try to capture any stderr output that might explain the exit
                if let Some(stderr) = child.stderr.take() {
                    let reader = BufReader::new(stderr);
                    let mut stderr_lines = Vec::new();
                    for line in reader.lines() {
                        if let Ok(line) = line {
                            stderr_lines.push(line);
                        }
                    }
                    if !stderr_lines.is_empty() {
                        tracing::error!("Scrcpy stderr output:");
                        for line in stderr_lines {
                            tracing::error!("  {}", line);
                        }
                    }
                }

                return Err(anyhow::anyhow!(
                    "Scrcpy process exited immediately with status: {:?}",
                    status
                ));
            }
            Ok(None) => {
                tracing::info!("Scrcpy process started successfully and is still running");

                // Spawn a background thread to monitor stderr output
                if let Some(stderr) = child.stderr.take() {
                    let reader = BufReader::new(stderr);
                    std::thread::spawn(move || {
                        for line in reader.lines() {
                            if let Ok(line) = line {
                                tracing::info!("Scrcpy stderr: {}", line);
                            }
                        }
                    });
                }
            }
            Err(e) => {
                tracing::error!("Error checking scrcpy process status: {}", e);
            }
        }

        Ok(child)
    }

    pub async fn start_async(&self, args: &[String]) -> Result<tokio::process::Child> {
        let mut cmd = TokioCommand::new(&self.path);
        cmd.args(args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Log the command for async version too
        tracing::info!("Starting scrcpy async with args: {:?}", args);

        let child = cmd.spawn()?;
        Ok(child)
    }

    pub fn build_args(
        &self,
        device_id: Option<&str>,
        bitrate: &str,
        orientation: Option<String>,
        show_touches: bool,
        fullscreen: bool,
        dimension: Option<u32>,
        extra_args: &str,
        turn_screen_off: bool,
    ) -> Vec<String> {
        let mut args = Vec::new();

        if let Some(device) = device_id {
            args.extend_from_slice(&["-s".to_string(), device.to_string()]);
        }

        args.extend_from_slice(&["-b".to_string(), bitrate.to_string()]);

        if let Some(orientation) = orientation {
            if !orientation.is_empty() {
                args.extend_from_slice(&["--orientation".to_string(), orientation]);
            }
        }

        if show_touches {
            args.push("--show-touches".to_string());
        }

        if fullscreen {
            args.push("--fullscreen".to_string());
        }

        if let Some(dim) = dimension {
            args.extend_from_slice(&["--max-size".to_string(), dim.to_string()]);
        }

        if turn_screen_off {
            args.push("-S".to_string());
        }

        // Parse extra arguments
        if !extra_args.is_empty() {
            let extra: Vec<String> = extra_args
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            args.extend(extra);
        }

        args
    }
}
