use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;

use crate::config::cache_dir;
use crate::models::GameCapabilities;

const PCGW_API: &str = "https://www.pcgamingwiki.com/w/api.php";

pub struct PcgwClient {
    http: reqwest::Client,
    cache_ttl: std::time::Duration,
}

impl PcgwClient {
    pub fn new(cache_ttl_hours: u64) -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent("nvopt/0.1 (game-settings-optimizer)")
                .build()
                .unwrap_or_default(),
            cache_ttl: std::time::Duration::from_secs(cache_ttl_hours * 3600),
        }
    }

    /// Fetch game capabilities by Steam AppID
    pub async fn get_capabilities(&self, steam_app_id: u32) -> Result<Option<GameCapabilities>> {
        // Check cache first
        if let Some(cached) = self.read_cache(steam_app_id) {
            return Ok(Some(cached));
        }

        // Query PCGW Cargo API
        let caps = self.query_pcgw(steam_app_id).await?;

        if let Some(ref c) = caps {
            self.write_cache(steam_app_id, c);
        }

        Ok(caps)
    }

    async fn query_pcgw(&self, steam_app_id: u32) -> Result<Option<GameCapabilities>> {
        let url = format!(
            "{}?action=cargoquery\
            &tables=Infobox_game,Video\
            &fields=Infobox_game._pageName=Page,\
            Infobox_game.Developers,\
            Video.4K_Ultra_HD,\
            Video.HDR,\
            Video.Ultrawidescreen,\
            Video.Ray_tracing,\
            Video.Upscaling,\
            Video.Frame_gen\
            &join_on=Infobox_game._pageID=Video._pageID\
            &where=Infobox_game.Steam_AppID HOLDS \"{}\"\
            &format=json",
            PCGW_API, steam_app_id
        );

        let resp = self.http.get(&url).send().await?;
        let body: CargoResponse = resp.json().await?;

        if body.cargoquery.is_empty() {
            return Ok(None);
        }

        let row = &body.cargoquery[0].title;

        let page_name = row
            .get("Page")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let pcgw_url = if !page_name.is_empty() {
            Some(format!(
                "https://www.pcgamingwiki.com/wiki/{}",
                page_name.replace(' ', "_")
            ))
        } else {
            None
        };

        // Parse the Upscaling field (comma-separated list like "DLSS,FSR")
        let upscaling_str = row
            .get("Upscaling")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();
        let has_dlss = upscaling_str.contains("dlss");
        let has_fsr = upscaling_str.contains("fsr");

        // Parse Frame_gen field (comma-separated)
        let frame_gen_str = row
            .get("Frame gen")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();
        let has_frame_gen = !frame_gen_str.is_empty()
            && frame_gen_str != "false"
            && frame_gen_str != "n/a";

        Ok(Some(GameCapabilities {
            page_name,
            developers: row
                .get("Developers")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            supports_4k: parse_bool_field(row, "4K Ultra HD"),
            supports_hdr: parse_bool_field(row, "HDR"),
            supports_ultrawide: parse_bool_field(row, "Ultrawidescreen"),
            supports_ray_tracing: parse_bool_field(row, "Ray tracing"),
            supports_dlss: Some(has_dlss),
            supports_fsr: Some(has_fsr),
            supports_frame_gen: Some(has_frame_gen),
            graphics_api: None,
            pcgw_url,
        }))
    }

    fn cache_path(&self, steam_app_id: u32) -> PathBuf {
        cache_dir().join(format!("{steam_app_id}.json"))
    }

    fn read_cache(&self, steam_app_id: u32) -> Option<GameCapabilities> {
        let path = self.cache_path(steam_app_id);
        if !path.exists() {
            return None;
        }

        // Check TTL
        if let Ok(metadata) = std::fs::metadata(&path) {
            if let Ok(modified) = metadata.modified() {
                if modified.elapsed().unwrap_or_default() > self.cache_ttl {
                    return None; // Expired
                }
            }
        }

        let contents = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&contents).ok()
    }

    fn write_cache(&self, steam_app_id: u32, caps: &GameCapabilities) {
        let dir = cache_dir();
        let _ = std::fs::create_dir_all(&dir);
        let path = self.cache_path(steam_app_id);
        if let Ok(json) = serde_json::to_string_pretty(caps) {
            let _ = std::fs::write(path, json);
        }
    }
}

fn parse_bool_field(
    row: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Option<bool> {
    row.get(key).and_then(|v| {
        let s = v.as_str().unwrap_or("");
        match s.to_lowercase().as_str() {
            "true" | "yes" | "native" | "hackable" => Some(true),
            "false" | "no" | "n/a" | "" => Some(false),
            _ => Some(!s.is_empty()),
        }
    })
}

#[derive(Debug, Deserialize)]
struct CargoResponse {
    cargoquery: Vec<CargoRow>,
}

#[derive(Debug, Deserialize)]
struct CargoRow {
    title: serde_json::Map<String, serde_json::Value>,
}
