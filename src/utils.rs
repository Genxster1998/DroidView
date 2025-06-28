use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;
use tracing;

pub fn find_executable(name: &str) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        // On Windows, use 'where' command to find executable in PATH
        if let Ok(output) = Command::new("where").arg(name).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).lines().next().unwrap_or("").trim().to_string();
                if !path.is_empty() {
                    return Some(PathBuf::from(path));
                }
            }
        }

        // Try common Windows paths
        let common_paths = [
            "C:\\Program Files\\Android\\android-sdk\\platform-tools",
            "C:\\Program Files (x86)\\Android\\android-sdk\\platform-tools",
            "C:\\Users\\%USERNAME%\\AppData\\Local\\Android\\Sdk\\platform-tools",
            "C:\\Android\\platform-tools",
        ];

        for path in &common_paths {
            let expanded_path = path.replace("%USERNAME%", &std::env::var("USERNAME").unwrap_or_default());
            let full_path = PathBuf::from(&expanded_path).join(format!("{}.exe", name));
            if full_path.exists() {
                return Some(full_path);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // On Unix-like systems, use 'which' command
        if let Ok(output) = Command::new("which").arg(name).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return Some(PathBuf::from(path));
            }
        }

        // Try common Unix paths
        let common_paths = [
            "/usr/bin",
            "/usr/local/bin",
            "/opt/homebrew/bin",
            "/usr/local/opt/android-platform-tools/bin",
        ];

        for path in &common_paths {
            let full_path = PathBuf::from(path).join(name);
            if full_path.exists() {
                return Some(full_path);
            }
        }
    }

    None
}

pub fn find_adb() -> Option<PathBuf> {
    find_executable("adb")
}

pub fn find_scrcpy() -> Option<PathBuf> {
    find_executable("scrcpy")
}

pub fn is_process_running(process_name: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("tasklist")
            .args(["/FI", &format!("IMAGENAME eq {}", process_name)])
            .output();

        if let Ok(output) = output {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let is_running = output_str.contains(process_name);
            tracing::debug!(
                "Windows process check for '{}': {}",
                process_name,
                is_running
            );
            is_running
        } else {
            tracing::debug!("Windows process check for '{}' failed", process_name);
            false
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = Command::new("pgrep").arg(process_name).output();

        if let Ok(output) = output {
            let is_running = output.status.success();
            tracing::debug!("Unix process check for '{}': {}", process_name, is_running);
            if is_running {
                let pids = String::from_utf8_lossy(&output.stdout);
                tracing::debug!("Found PIDs: {}", pids.trim());
            }
            is_running
        } else {
            tracing::debug!("Unix process check for '{}' failed", process_name);
            false
        }
    }
}

pub fn open_url(url: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd").args(["/C", "start", url]).spawn()?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(url).spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(url).spawn()?;
    }

    Ok(())
}

pub fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes < KB {
        format!("{} B", bytes)
    } else if bytes < MB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    }
}

pub fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
