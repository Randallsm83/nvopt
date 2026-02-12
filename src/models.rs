use serde::{Deserialize, Serialize};

/// Detected hardware profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub gpu_name: String,
    pub gpu_vram_mb: u32,
    pub driver_version: String,
    pub cpu_name: String,
    pub ram_total_mb: u64,
    pub display_resolution: (u32, u32),
    pub display_refresh_hz: u32,
    pub gpu_tier: GpuTier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuTier {
    /// RTX 5090, 5080
    Ultra,
    /// RTX 5070 Ti, 4090, 4080
    High,
    /// RTX 5070, 4070 Ti, 4070
    Mid,
    /// RTX 4060 Ti, 4060, 3070
    Entry,
    /// Anything older/lower
    Low,
}

impl GpuTier {
    pub fn from_gpu_name(name: &str, vram_mb: u32) -> Self {
        let name_upper = name.to_uppercase();
        if name_upper.contains("5090") || name_upper.contains("5080") {
            GpuTier::Ultra
        } else if name_upper.contains("5070 TI")
            || name_upper.contains("4090")
            || name_upper.contains("4080")
        {
            GpuTier::High
        } else if name_upper.contains("5070")
            || name_upper.contains("4070")
            || name_upper.contains("3080")
        {
            GpuTier::Mid
        } else if name_upper.contains("4060")
            || name_upper.contains("3070")
            || name_upper.contains("3060")
        {
            GpuTier::Entry
        } else if vram_mb >= 12000 {
            GpuTier::Mid
        } else if vram_mb >= 8000 {
            GpuTier::Entry
        } else {
            GpuTier::Low
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            GpuTier::Ultra => "Ultra",
            GpuTier::High => "High",
            GpuTier::Mid => "Mid",
            GpuTier::Entry => "Entry",
            GpuTier::Low => "Low",
        }
    }
}

/// A game from any source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub name: String,
    pub steam_app_id: Option<u32>,
    pub platform: GamePlatform,
    pub install_path: Option<String>,
    pub installed: bool,
    pub pinned: bool,
    pub capabilities: Option<GameCapabilities>,
    // Old generic in-game recommendations (kept for compatibility until full pivot completes)
    pub recommendations: Option<RecommendedSettings>,
    // New: Real DRS/driver-side profile to export/apply via NV Inspector / NVAPI
    pub drs_profile: Option<DrsProfile>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GamePlatform {
    Steam,
    Epic,
    Gog,
    Other,
}

impl GamePlatform {
    pub fn icon(&self) -> &'static str {
        match self {
            GamePlatform::Steam => "󰓓",
            GamePlatform::Epic => "",
            GamePlatform::Gog => "󰊗",
            GamePlatform::Other => "",
        }
    }
}

/// Capabilities fetched from PCGamingWiki
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameCapabilities {
    pub page_name: String,
    pub developers: Option<String>,
    pub supports_4k: Option<bool>,
    pub supports_hdr: Option<bool>,
    pub supports_ultrawide: Option<bool>,
    pub supports_dlss: Option<bool>,
    pub supports_fsr: Option<bool>,
    pub supports_ray_tracing: Option<bool>,
    pub supports_frame_gen: Option<bool>,
    pub graphics_api: Option<String>,
    pub pcgw_url: Option<String>,
}

/// DRS (driver) setting entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrsSetting {
    pub id: u32,
    pub name: String,
    pub value: u32,
    pub value_name: Option<String>,
}

/// A DRS profile is a list of concrete NVAPI settings to apply
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrsProfile {
    pub settings: Vec<DrsSetting>,
    #[serde(default)]
    pub notes: Vec<String>,
}

/// Recommended in-game settings (legacy)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendedSettings {
    pub overall_preset: SettingLevel,
    pub resolution_scale: ResolutionScale,
    pub texture_quality: SettingLevel,
    pub shadow_quality: SettingLevel,
    pub ray_tracing: RtRecommendation,
    pub anti_aliasing: AaRecommendation,
    pub upscaling: UpscaleRecommendation,
    pub frame_gen: bool,
    pub vsync: bool,
    pub max_fps: Option<u32>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettingLevel {
    Ultra,
    High,
    Medium,
    Low,
    Off,
}

impl std::fmt::Display for SettingLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettingLevel::Ultra => write!(f, "Ultra"),
            SettingLevel::High => write!(f, "High"),
            SettingLevel::Medium => write!(f, "Medium"),
            SettingLevel::Low => write!(f, "Low"),
            SettingLevel::Off => write!(f, "Off"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResolutionScale {
    Native,
    /// e.g. "DLSS Quality" or "FSR Quality"
    Upscaled(String),
}

impl std::fmt::Display for ResolutionScale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolutionScale::Native => write!(f, "Native"),
            ResolutionScale::Upscaled(method) => write!(f, "{method}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RtRecommendation {
    Off,
    On(SettingLevel),
}

impl std::fmt::Display for RtRecommendation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RtRecommendation::Off => write!(f, "Off"),
            RtRecommendation::On(level) => write!(f, "{level}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AaRecommendation {
    Off,
    Method(String),
}

impl std::fmt::Display for AaRecommendation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AaRecommendation::Off => write!(f, "Off"),
            AaRecommendation::Method(m) => write!(f, "{m}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpscaleRecommendation {
    None,
    Dlss(String),
    Fsr(String),
}

impl std::fmt::Display for UpscaleRecommendation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpscaleRecommendation::None => write!(f, "None"),
            UpscaleRecommendation::Dlss(mode) => write!(f, "DLSS {mode}"),
            UpscaleRecommendation::Fsr(mode) => write!(f, "FSR {mode}"),
        }
    }
}

/// Watchlist entry for non-installed games
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchlistEntry {
    pub name: String,
    pub steam_app_id: Option<u32>,
}
