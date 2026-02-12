use anyhow::Result;

use crate::config::{self, AppConfig};
use crate::models::*;
use crate::pcgw::PcgwClient;
use crate::search::GameSearch;

pub enum AppMode {
    Normal,
    Search,
}

pub enum ListSection {
    Installed,
    Watchlist,
    SearchResults,
}

pub struct App {
    pub _config: AppConfig,
    pub hw: Option<HardwareProfile>,
    pub installed_games: Vec<Game>,
    pub watchlist_games: Vec<Game>,
    pub search_results: Vec<Game>,
    pub selected_index: usize,
    pub active_section: ListSection,
    pub mode: AppMode,
    pub search_input: String,
    pub status_message: String,
    pub pcgw: PcgwClient,
    pub searcher: GameSearch,
    pub should_quit: bool,
}

impl App {
    pub fn new(config: AppConfig, hw: Option<HardwareProfile>, games: Vec<Game>) -> Self {
        let pcgw = PcgwClient::new(config.cache_ttl_hours);
        let searcher = GameSearch::new();

        // Load watchlist
        let watchlist_entries = config::load_watchlist().unwrap_or_default();
        let watchlist_games: Vec<Game> = watchlist_entries
            .into_iter()
            .map(|e| Game {
                name: e.name,
                steam_app_id: e.steam_app_id,
                platform: GamePlatform::Steam,
                install_path: None,
                installed: false,
                pinned: true,
                capabilities: None,
                recommendations: None,
                drs_profile: None,
            })
            .collect();

        let status_message = if let Some(ref hw) = hw {
            hw.to_string()
        } else {
            "Hardware detection failed".to_string()
        };

        Self {
            _config: config,
            hw,
            installed_games: games,
            watchlist_games,
            search_results: Vec::new(),
            selected_index: 0,
            active_section: ListSection::Installed,
            mode: AppMode::Normal,
            search_input: String::new(),
            status_message,
            pcgw,
            searcher,
            should_quit: false,
        }
    }

    pub fn active_list(&self) -> &[Game] {
        match self.active_section {
            ListSection::Installed => &self.installed_games,
            ListSection::Watchlist => &self.watchlist_games,
            ListSection::SearchResults => &self.search_results,
        }
    }

    pub fn active_list_mut(&mut self) -> &mut Vec<Game> {
        match self.active_section {
            ListSection::Installed => &mut self.installed_games,
            ListSection::Watchlist => &mut self.watchlist_games,
            ListSection::SearchResults => &mut self.search_results,
        }
    }

    pub fn selected_game(&self) -> Option<&Game> {
        self.active_list().get(self.selected_index)
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let len = self.active_list().len();
        if len > 0 && self.selected_index < len - 1 {
            self.selected_index += 1;
        }
    }

    pub fn next_section(&mut self) {
        self.active_section = match self.active_section {
            ListSection::Installed => ListSection::Watchlist,
            ListSection::Watchlist => ListSection::SearchResults,
            ListSection::SearchResults => ListSection::Installed,
        };
        self.selected_index = 0;
    }

    pub fn enter_search(&mut self) {
        self.mode = AppMode::Search;
        self.search_input.clear();
    }

    pub fn exit_search(&mut self) {
        self.mode = AppMode::Normal;
    }

    pub async fn execute_search(&mut self) {
        let query = self.search_input.clone();
        self.status_message = format!("Searching for '{query}'...");

        match self.searcher.search(&query).await {
            Ok(results) => {
                let count = results.len();
                self.search_results = results;
                self.active_section = ListSection::SearchResults;
                self.selected_index = 0;
                self.status_message = format!("Found {count} results for '{query}'");
            }
            Err(e) => {
                self.status_message = format!("Search error: {e}");
            }
        }

        self.mode = AppMode::Normal;
    }

    pub async fn fetch_capabilities_for_selected(&mut self) {
        let game = match self.active_list().get(self.selected_index) {
            Some(g) => g.clone(),
            None => return,
        };

        if game.capabilities.is_some() {
            return; // Already fetched
        }

        let app_id = match game.steam_app_id {
            Some(id) => id,
            None => return,
        };

        self.status_message = format!("Fetching info for {}...", game.name);

        match self.pcgw.get_capabilities(app_id).await {
            Ok(Some(caps)) => {
                // Build new DRS profile (driver-side settings)
                let (drs_profile, legacy_rec) = if let Some(ref hw) = self.hw {
                    let drs = crate::drs::build_default_profile(hw, &caps);
                    // Keep legacy recommendations for now (not shown by default)
                    let legacy = Some(crate::recommend::recommend(hw, &caps));
                    (Some(drs), legacy)
                } else {
                    (None, None)
                };

                let idx = self.selected_index;
                let list = self.active_list_mut();
                if let Some(g) = list.get_mut(idx) {
                    g.capabilities = Some(caps);
                    g.drs_profile = drs_profile;
                    g.recommendations = legacy_rec; // retained but UI will favor DRS
                }
                self.status_message = format!("Loaded info for {}", game.name);
            }
            Ok(None) => {
                self.status_message = format!("No PCGW data found for {}", game.name);
            }
            Err(e) => {
                self.status_message = format!("Error fetching {}: {e}", game.name);
            }
        }
    }

    pub fn pin_selected(&mut self) {
        if !matches!(self.active_section, ListSection::SearchResults) {
            return;
        }
        if let Some(game) = self.search_results.get(self.selected_index) {
            let entry = WatchlistEntry {
                name: game.name.clone(),
                steam_app_id: game.steam_app_id,
            };

            // Check not already in watchlist
            let already = self
                .watchlist_games
                .iter()
                .any(|g| g.steam_app_id == game.steam_app_id);

            if !already {
                self.watchlist_games.push(Game {
                    name: entry.name.clone(),
                    steam_app_id: entry.steam_app_id,
                    platform: GamePlatform::Steam,
                    install_path: None,
                    installed: false,
                    pinned: true,
                    capabilities: game.capabilities.clone(),
                    recommendations: game.recommendations.clone(),
                    drs_profile: game.drs_profile.clone(),
                });

                // Save watchlist
                let entries: Vec<WatchlistEntry> = self
                    .watchlist_games
                    .iter()
                    .map(|g| WatchlistEntry {
                        name: g.name.clone(),
                        steam_app_id: g.steam_app_id,
                    })
                    .collect();
                let _ = config::save_watchlist(&entries);

                self.status_message = format!("Pinned '{}' to watchlist", entry.name);
            } else {
                self.status_message = format!("'{}' already in watchlist", game.name);
            }
        }
    }

    pub fn export_selected(&self) -> Result<()> {
        let game = match self.selected_game() {
            Some(g) if g.drs_profile.is_some() || g.recommendations.is_some() => g,
            _ => anyhow::bail!("No profile to export"),
        };

        let filename = format!(
            "{}.nip",
            game.name
                .chars()
                .map(|c| if c.is_alphanumeric() || c == ' ' { c } else { '_' })
                .collect::<String>()
        );

        let path = config::config_dir().join("exports");
        std::fs::create_dir_all(&path)?;
        let full_path = path.join(&filename);

        crate::export::export_nip(&[game], &full_path)?;
        Ok(())
    }

    pub fn open_pcgw(&self) {
        if let Some(game) = self.selected_game() {
            if let Some(ref caps) = game.capabilities {
                if let Some(ref url) = caps.pcgw_url {
                    let _ = open::that(url);
                }
            }
        }
    }
}
