use anyhow::Result;
use serde::Deserialize;

use crate::models::{Game, GamePlatform};

const STEAM_SEARCH_URL: &str = "https://store.steampowered.com/api/storesearch";

pub struct GameSearch {
    http: reqwest::Client,
}

impl GameSearch {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent("nvopt/0.1")
                .build()
                .unwrap_or_default(),
        }
    }

    /// Search Steam Store for games matching query
    pub async fn search(&self, query: &str) -> Result<Vec<Game>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        let resp = self
            .http
            .get(STEAM_SEARCH_URL)
            .query(&[("term", query), ("l", "english"), ("cc", "US")])
            .send()
            .await?;

        let body: SteamSearchResponse = resp.json().await?;

        let games = body
            .items
            .into_iter()
            .filter(|item| item.r#type == "app")
            .map(|item| Game {
                name: item.name,
                steam_app_id: Some(item.id),
                platform: GamePlatform::Steam,
                install_path: None,
                installed: false,
                pinned: false,
                capabilities: None,
                recommendations: None,
                drs_profile: None,
            })
            .collect();

        Ok(games)
    }
}

#[derive(Debug, Deserialize)]
struct SteamSearchResponse {
    #[serde(default)]
    items: Vec<SteamSearchItem>,
}

#[derive(Debug, Deserialize)]
struct SteamSearchItem {
    #[serde(rename = "type")]
    r#type: String,
    name: String,
    id: u32,
}
