//! DRS (Driver) settings: NVAPI IDs and helpers to build concrete profiles

use crate::models::{DrsProfile, DrsSetting, GameCapabilities, HardwareProfile};

// NVAPI Setting IDs (from NvApiDriverSettings.h)
#[allow(dead_code)]
pub mod id {
    pub const VSYNCMODE_ID: u32 = 0x00A879CF;
    pub const VSYNCVRRCONTROL_ID: u32 = 0x10A879CE; // Variable Refresh Rate (G-SYNC/FreeSync)
    pub const PREFERRED_PSTATE_ID: u32 = 0x1057EB71; // Power management mode
    pub const QUALITY_ENHANCEMENTS_ID: u32 = 0x00CE2691; // Texture filtering - Quality
    pub const OGL_TRIPLE_BUFFER_ID: u32 = 0x20FDD1F9; // Triple buffering
    pub const OGL_THREAD_CONTROL_ID: u32 = 0x20C1221E; // Threaded optimization (OpenGL)
    pub const OGL_CPL_GDI_COMPATIBILITY_ID: u32 = 0x2072C5A3; // OpenGL GDI compatibility
    pub const OGL_CPL_PREFER_DXPRESENT_ID: u32 = 0x20D690F8; // Vulkan/OpenGL present method
    pub const PS_SHADERDISKCACHE_ID: u32 = 0x00198FFF; // Shader cache
    pub const CUDA_EXCLUDED_GPUS_ID: u32 = 0x10354FF8; // CUDA GPUs (bitmask of excluded devices)
    // Frame Rate Limiter family exists but encoding varies by driver; we omit here deliberately
}

// Selected enum values
#[allow(dead_code)]
pub mod val {
    // VSYNC
    pub const VSYNCMODE_PASSIVE: u32 = 0x6092_5292; // App-controlled
    pub const VSYNCMODE_FORCEOFF: u32 = 0x0841_6747;
    pub const VSYNCMODE_FORCEON: u32 = 0x4781_4940;

    // VRR (G-SYNC/FreeSync Compatible)
    pub const VSYNCVRR_DISABLE: u32 = 0x0000_0000;
    pub const VSYNCVRR_ENABLE: u32 = 0x0000_0001;

    // Power management
    pub const PSTATE_ADAPTIVE: u32 = 0x0000_0000;
    pub const PSTATE_PREFER_MAX: u32 = 0x0000_0001;
    pub const PSTATE_OPTIMAL_POWER: u32 = 0x0000_0005;

    // Quality Enhancements (Texture filtering quality)
    pub const QUAL_HIGH_QUALITY: u32 = 0xFFFF_FFF6; // High Quality
    pub const QUAL_QUALITY: u32 = 0x0000_0000;      // Quality (default)
    pub const QUAL_PERFORMANCE: u32 = 0x0000_000A;  // Performance
    pub const QUAL_HIGH_PERF: u32 = 0x0000_0014;    // High Performance

    // OpenGL-only toggles
    pub const OGL_TRIPLE_BUFFER_DISABLED: u32 = 0x0;
    pub const OGL_TRIPLE_BUFFER_ENABLED: u32 = 0x1;

    pub const OGL_THREAD_ENABLE: u32 = 0x1;
    pub const OGL_THREAD_DISABLE: u32 = 0x2;

    // OpenGL GDI compatibility
    pub const OGL_GDI_PREFER_DISABLED: u32 = 0x0;
    pub const OGL_GDI_PREFER_ENABLED: u32 = 0x1;
    pub const OGL_GDI_AUTO: u32 = 0x2;

    // Vulkan/OpenGL present method
    pub const OGL_DXPRESENT_PREFER_DISABLED: u32 = 0x0;
    pub const OGL_DXPRESENT_PREFER_ENABLED: u32 = 0x1;
    pub const OGL_DXPRESENT_AUTO: u32 = 0x2;

    // Shader Cache
    pub const SHADER_CACHE_OFF: u32 = 0x0;
    pub const SHADER_CACHE_ON: u32 = 0x1;
}

pub fn build_default_profile(hw: &HardwareProfile, caps: &GameCapabilities) -> DrsProfile {
    let mut settings: Vec<DrsSetting> = Vec::new();

    // 1) Variable Refresh Rate ON if available (assume G-SYNC compatible monitor in modern systems)
    settings.push(DrsSetting {
        id: id::VSYNCVRRCONTROL_ID,
        name: "Variable Refresh Rate (VRR/G-SYNC)".into(),
        value: val::VSYNCVRR_ENABLE,
        value_name: Some("Enable".into()),
    });

    // 2) V-Sync mode: app-controlled (let G-SYNC handle)
    settings.push(DrsSetting {
        id: id::VSYNCMODE_ID,
        name: "Vertical Sync".into(),
        value: val::VSYNCMODE_PASSIVE,
        value_name: Some("Use the 3D application setting".into()),
    });

    // 3) Power management: Prefer maximum performance for games
    settings.push(DrsSetting {
        id: id::PREFERRED_PSTATE_ID,
        name: "Power management mode".into(),
        value: val::PSTATE_PREFER_MAX,
        value_name: Some("Prefer maximum performance".into()),
    });

    // 4) Texture filtering quality: Quality (or High Quality on ultra tier)
    let qual = match hw.gpu_tier {
        crate::models::GpuTier::Ultra => val::QUAL_HIGH_QUALITY,
        crate::models::GpuTier::High => val::QUAL_QUALITY,
        crate::models::GpuTier::Mid => val::QUAL_QUALITY,
        _ => val::QUAL_PERFORMANCE,
    };
    let qual_label = match qual {
        val::QUAL_HIGH_QUALITY => "High quality",
        val::QUAL_QUALITY => "Quality",
        val::QUAL_PERFORMANCE => "Performance",
        _ => "High performance",
    };
    settings.push(DrsSetting {
        id: id::QUALITY_ENHANCEMENTS_ID,
        name: "Texture filtering - Quality".into(),
        value: qual,
        value_name: Some(qual_label.into()),
    });

    // 5) Shader Disk Cache: On
    settings.push(DrsSetting {
        id: id::PS_SHADERDISKCACHE_ID,
        name: "Shader Cache".into(),
        value: val::SHADER_CACHE_ON,
        value_name: Some("On".into()),
    });

    // 6) Vulkan/OpenGL present method: Auto
    settings.push(DrsSetting {
        id: id::OGL_CPL_PREFER_DXPRESENT_ID,
        name: "Vulkan/OpenGL present method".into(),
        value: val::OGL_DXPRESENT_AUTO,
        value_name: Some("Auto".into()),
    });

    // 7) OpenGL GDI compatibility: Auto
    settings.push(DrsSetting {
        id: id::OGL_CPL_GDI_COMPATIBILITY_ID,
        name: "OpenGL GDI compatibility".into(),
        value: val::OGL_GDI_AUTO,
        value_name: Some("Auto".into()),
    });

    // 8) Threaded optimization (OpenGL): Enable
    settings.push(DrsSetting {
        id: id::OGL_THREAD_CONTROL_ID,
        name: "OpenGL threaded optimization".into(),
        value: val::OGL_THREAD_ENABLE,
        value_name: Some("On".into()),
    });

    // 9) Triple buffering (OpenGL): Off
    settings.push(DrsSetting {
        id: id::OGL_TRIPLE_BUFFER_ID,
        name: "OpenGL triple buffering".into(),
        value: val::OGL_TRIPLE_BUFFER_DISABLED,
        value_name: Some("Off".into()),
    });

    // 10) CUDA GPUs: do not exclude any
    settings.push(DrsSetting {
        id: id::CUDA_EXCLUDED_GPUS_ID,
        name: "CUDA GPUs (excluded mask)".into(),
        value: 0x0000_0000,
        value_name: Some("None excluded (use all)".into()),
    });

    let mut notes: Vec<String> = Vec::new();

    // Add guidance for frame limiter (not set via DRS due to encoding variability)
    if hw.display_refresh_hz >= 120 {
        notes.push(format!(
            "Tip: Cap FPS to {} ({}-3) for best G-SYNC latency.",
            hw.display_refresh_hz.saturating_sub(3),
            hw.display_refresh_hz
        ));
    }

    // If the game is known to support frame generation, suggest enabling Reflex in-game
    if caps.supports_frame_gen.unwrap_or(false) {
        notes.push("Enable NVIDIA Reflex (On + Boost) in-game when using Frame Generation.".into());
    }

    DrsProfile { settings, notes }
}