use crate::models::*;

/// Generate recommended settings for a game based on hardware and capabilities
pub fn recommend(hw: &HardwareProfile, caps: &GameCapabilities) -> RecommendedSettings {
    let total_pixels = hw.display_resolution.0 as u64 * hw.display_resolution.1 as u64;
    let is_ultrawide = hw.display_resolution.0 as f64 / hw.display_resolution.1 as f64 > 2.5;
    let is_4k_plus = total_pixels >= 3840 * 2160;
    let is_super_ultrawide = total_pixels >= 7680 * 2160;

    let has_dlss = caps.supports_dlss.unwrap_or(false);
    let has_fsr = caps.supports_fsr.unwrap_or(false);
    let has_rt = caps.supports_ray_tracing.unwrap_or(false);
    let has_fg = caps.supports_frame_gen.unwrap_or(false);

    let mut notes = Vec::new();

    // --- Upscaling recommendation ---
    let upscaling = if has_dlss {
        let mode = match (&hw.gpu_tier, is_super_ultrawide, is_4k_plus) {
            (_, true, _) => "Quality",         // Super ultrawide always needs upscaling
            (GpuTier::Ultra, _, false) => "Off", // Ultra tier at non-4K can go native
            (GpuTier::Ultra, _, true) => "Quality",
            (GpuTier::High, _, true) => "Quality",
            (GpuTier::High, _, false) => "Off",
            (GpuTier::Mid, _, true) => "Balanced",
            (GpuTier::Mid, _, false) => "Quality",
            (GpuTier::Entry, _, _) => "Balanced",
            (GpuTier::Low, _, _) => "Performance",
        };
        if mode == "Off" {
            UpscaleRecommendation::None
        } else {
            UpscaleRecommendation::Dlss(mode.to_string())
        }
    } else if has_fsr {
        let mode = if is_super_ultrawide || is_4k_plus {
            "Quality"
        } else {
            match hw.gpu_tier {
                GpuTier::Ultra | GpuTier::High => "Off",
                GpuTier::Mid => "Quality",
                GpuTier::Entry => "Balanced",
                GpuTier::Low => "Performance",
            }
        };
        if mode == "Off" {
            UpscaleRecommendation::None
        } else {
            UpscaleRecommendation::Fsr(mode.to_string())
        }
    } else {
        if is_super_ultrawide || is_4k_plus {
            notes.push("No upscaling support — may need to lower settings for this resolution".into());
        }
        UpscaleRecommendation::None
    };

    // --- Resolution scale ---
    let resolution_scale = match &upscaling {
        UpscaleRecommendation::None => ResolutionScale::Native,
        UpscaleRecommendation::Dlss(mode) => ResolutionScale::Upscaled(format!("DLSS {mode}")),
        UpscaleRecommendation::Fsr(mode) => ResolutionScale::Upscaled(format!("FSR {mode}")),
    };

    // --- Overall preset ---
    let overall_preset = match (&hw.gpu_tier, is_super_ultrawide) {
        (GpuTier::Ultra, false) => SettingLevel::Ultra,
        (GpuTier::Ultra, true) => SettingLevel::High,
        (GpuTier::High, false) => SettingLevel::Ultra,
        (GpuTier::High, true) => SettingLevel::High,
        (GpuTier::Mid, false) => SettingLevel::High,
        (GpuTier::Mid, true) => SettingLevel::Medium,
        (GpuTier::Entry, _) => SettingLevel::Medium,
        (GpuTier::Low, _) => SettingLevel::Low,
    };

    // --- Texture quality (VRAM-dependent) ---
    let texture_quality = if hw.gpu_vram_mb >= 16000 {
        SettingLevel::Ultra
    } else if hw.gpu_vram_mb >= 12000 {
        SettingLevel::High
    } else if hw.gpu_vram_mb >= 8000 {
        SettingLevel::High
    } else {
        SettingLevel::Medium
    };

    // --- Shadow quality ---
    let shadow_quality = match (&hw.gpu_tier, is_super_ultrawide) {
        (GpuTier::Ultra, false) => SettingLevel::Ultra,
        (GpuTier::Ultra, true) => SettingLevel::High,
        (GpuTier::High, _) => SettingLevel::High,
        (GpuTier::Mid, _) => SettingLevel::High,
        (GpuTier::Entry, _) => SettingLevel::Medium,
        (GpuTier::Low, _) => SettingLevel::Low,
    };

    // --- Ray tracing ---
    let ray_tracing = if has_rt {
        match (&hw.gpu_tier, is_super_ultrawide) {
            (GpuTier::Ultra, false) => RtRecommendation::On(SettingLevel::Ultra),
            (GpuTier::Ultra, true) => RtRecommendation::On(SettingLevel::Medium),
            (GpuTier::High, false) => RtRecommendation::On(SettingLevel::High),
            (GpuTier::High, true) => RtRecommendation::On(SettingLevel::Low),
            (GpuTier::Mid, false) => RtRecommendation::On(SettingLevel::Medium),
            (GpuTier::Mid, true) => RtRecommendation::Off,
            _ => RtRecommendation::Off,
        }
    } else {
        RtRecommendation::Off
    };

    // --- Anti-aliasing ---
    let anti_aliasing = match &upscaling {
        UpscaleRecommendation::Dlss(_) | UpscaleRecommendation::Fsr(_) => {
            // Upscaler handles AA
            AaRecommendation::Method("Via upscaler".to_string())
        }
        UpscaleRecommendation::None => AaRecommendation::Method("TAA".to_string()),
    };

    // --- Frame gen ---
    let frame_gen = has_fg && matches!(hw.gpu_tier, GpuTier::Ultra | GpuTier::High);
    if has_fg && frame_gen {
        notes.push("Frame Generation enabled — use NVIDIA Reflex for low latency".into());
    }

    // --- V-Sync ---
    let vsync = false; // Generally off with G-SYNC

    // --- Max FPS ---
    let max_fps = if hw.display_refresh_hz > 60 {
        Some(hw.display_refresh_hz)
    } else {
        None
    };

    // Ultrawide note
    if is_ultrawide {
        if caps.supports_ultrawide == Some(true) {
            notes.push("Native ultrawide supported".into());
        } else if caps.supports_ultrawide == Some(false) {
            notes.push("⚠ No native ultrawide — may need mods or black bars".into());
        }
    }

    RecommendedSettings {
        overall_preset,
        resolution_scale,
        texture_quality,
        shadow_quality,
        ray_tracing,
        anti_aliasing,
        upscaling,
        frame_gen,
        vsync,
        max_fps,
        notes,
    }
}
