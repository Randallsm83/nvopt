use anyhow::{Context, Result};
use std::process::Command;

use crate::models::{GpuTier, HardwareProfile};

pub fn detect_hardware() -> Result<HardwareProfile> {
    let (gpu_name, gpu_vram_mb, driver_version) = detect_gpu()?;
    let cpu_name = detect_cpu()?;
    let ram_total_mb = detect_ram()?;
    let (res, refresh) = detect_display()?;
    let gpu_tier = GpuTier::from_gpu_name(&gpu_name, gpu_vram_mb);

    Ok(HardwareProfile {
        gpu_name,
        gpu_vram_mb,
        driver_version,
        cpu_name,
        ram_total_mb,
        display_resolution: res,
        display_refresh_hz: refresh,
        gpu_tier,
    })
}

fn detect_gpu() -> Result<(String, u32, String)> {
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total,driver_version",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .context("Failed to run nvidia-smi. Is an NVIDIA GPU installed?")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout.trim().splitn(3, ", ").collect();

    if parts.len() < 3 {
        anyhow::bail!("Unexpected nvidia-smi output: {stdout}");
    }

    let name = parts[0].trim().to_string();
    let vram: u32 = parts[1].trim().parse().unwrap_or(0);
    let driver = parts[2].trim().to_string();

    Ok((name, vram, driver))
}

fn detect_cpu() -> Result<String> {
    let output = Command::new("wmic")
        .args(["cpu", "get", "name", "/value"])
        .output()
        .context("Failed to query CPU info")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("Name=") {
            return Ok(val.trim().to_string());
        }
    }
    Ok("Unknown CPU".to_string())
}

fn detect_ram() -> Result<u64> {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_OperatingSystem).TotalVisibleMemorySize",
        ])
        .output()
        .context("Failed to query RAM")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let kb: u64 = stdout.trim().parse().unwrap_or(0);
    Ok(kb / 1024) // convert to MB
}

fn detect_display() -> Result<((u32, u32), u32)> {
    // Try nvidia-smi first for resolution info
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            r#"
            Add-Type -AssemblyName System.Windows.Forms
            $screen = [System.Windows.Forms.Screen]::PrimaryScreen
            $bounds = $screen.Bounds
            Write-Output "$($bounds.Width)x$($bounds.Height)"
            "#,
        ])
        .output();

    let (width, height) = if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let parts: Vec<&str> = stdout.trim().split('x').collect();
        if parts.len() == 2 {
            let w: u32 = parts[0].parse().unwrap_or(2560);
            let h: u32 = parts[1].parse().unwrap_or(1440);
            (w, h)
        } else {
            (2560, 1440)
        }
    } else {
        (2560, 1440)
    };

    // Try to get refresh rate
    let refresh_output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            r#"
            $dm = Get-CimInstance -ClassName Win32_VideoController | Select-Object -First 1
            $freq = (Get-CimInstance -ClassName Win32_DesktopMonitor | Select-Object -First 1).ScreenHeight
            # Fallback to registry
            $path = 'HKLM:\SYSTEM\CurrentControlSet\Control\GraphicsDrivers\Configuration'
            if (Test-Path $path) {
                $configs = Get-ChildItem $path -Recurse -ErrorAction SilentlyContinue
                foreach ($c in $configs) {
                    $vsr = $c.GetValue('VSyncFreq.Numerator')
                    if ($vsr -and $vsr -gt 0) { Write-Output $vsr; return }
                }
            }
            Write-Output "60"
            "#,
        ])
        .output();

    let refresh: u32 = if let Ok(out) = refresh_output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        stdout.trim().lines().next().unwrap_or("60").parse().unwrap_or(60)
    } else {
        60
    };

    Ok(((width, height), refresh))
}

impl std::fmt::Display for HardwareProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({}MB) │ {} │ {}MB RAM │ {}x{}@{}Hz │ Tier: {}",
            self.gpu_name,
            self.gpu_vram_mb,
            self.cpu_name,
            self.ram_total_mb,
            self.display_resolution.0,
            self.display_resolution.1,
            self.display_refresh_hz,
            self.gpu_tier.label(),
        )
    }
}
