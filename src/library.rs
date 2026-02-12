use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::models::{Game, GamePlatform};

/// Detect all installed Steam games
pub fn detect_steam_games(extra_paths: &[String]) -> Result<Vec<Game>> {
    let mut games = Vec::new();
    let library_paths = find_steam_libraries(extra_paths)?;

    for lib_path in &library_paths {
        let steamapps = lib_path.join("steamapps");
        if !steamapps.exists() {
            continue;
        }

        // Read all appmanifest files
        let entries = std::fs::read_dir(&steamapps).with_context(|| {
            format!("Failed to read Steam library: {}", steamapps.display())
        })?;

        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name.starts_with("appmanifest_") && name.ends_with(".acf") {
                if let Ok(game) = parse_appmanifest(&path) {
                    games.push(game);
                }
            }
        }
    }

    // Deduplicate by app_id
    games.sort_by_key(|g| g.steam_app_id.unwrap_or(0));
    games.dedup_by_key(|g| g.steam_app_id.unwrap_or(0));
    // Sort by name
    games.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(games)
}

/// Find Steam library folders from registry + libraryfolders.vdf
fn find_steam_libraries(extra_paths: &[String]) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();

    // Try registry first
    if let Ok(steam_path) = get_steam_path_from_registry() {
        paths.push(PathBuf::from(&steam_path));

        // Parse libraryfolders.vdf for additional libraries
        let vdf_path = PathBuf::from(&steam_path)
            .join("steamapps")
            .join("libraryfolders.vdf");
        if vdf_path.exists() {
            if let Ok(lib_paths) = parse_libraryfolders_vdf(&vdf_path) {
                for p in lib_paths {
                    let pb = PathBuf::from(&p);
                    if pb.exists() && !paths.contains(&pb) {
                        paths.push(pb);
                    }
                }
            }
        }
    }

    // Add user-specified paths
    for p in extra_paths {
        let pb = PathBuf::from(p);
        if pb.exists() && !paths.contains(&pb) {
            paths.push(pb);
        }
    }

    Ok(paths)
}

fn get_steam_path_from_registry() -> Result<String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let steam_key = hkcu.open_subkey("SOFTWARE\\Valve\\Steam")?;
    let path: String = steam_key.get_value("SteamPath")?;
    Ok(path.replace('/', "\\"))
}

/// Parse Valve's VDF format (simplified key-value parser)
fn parse_libraryfolders_vdf(path: &Path) -> Result<Vec<String>> {
    let contents = std::fs::read_to_string(path)?;
    let mut paths = Vec::new();

    // Simple line-by-line parser for the "path" keys
    for line in contents.lines() {
        let trimmed = line.trim().trim_matches('\t');
        if let Some(rest) = trimmed.strip_prefix("\"path\"") {
            let val = rest.trim().trim_matches('"').replace("\\\\", "\\");
            if !val.is_empty() {
                paths.push(val);
            }
        }
    }

    Ok(paths)
}

/// Parse a single appmanifest_XXXXX.acf file
fn parse_appmanifest(path: &Path) -> Result<Game> {
    let contents = std::fs::read_to_string(path)?;
    let kv = parse_simple_vdf(&contents);

    let app_id: u32 = kv
        .get("appid")
        .context("Missing appid in manifest")?
        .parse()
        .context("Invalid appid")?;

    let name = kv
        .get("name")
        .cloned()
        .unwrap_or_else(|| format!("Unknown ({})", app_id));

    let install_dir = kv.get("installdir").cloned();
    let install_path = install_dir.map(|dir| {
        path.parent()
            .unwrap_or(Path::new("."))
            .join("common")
            .join(dir)
            .to_string_lossy()
            .to_string()
    });

    // Skip Steamworks redistributable (228980) and other tools
    let state: u32 = kv
        .get("StateFlags")
        .or_else(|| kv.get("stateflags"))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    if app_id == 228980 {
        anyhow::bail!("Skipping Steamworks Common Redistributables");
    }

    Ok(Game {
        name,
        steam_app_id: Some(app_id),
        platform: GamePlatform::Steam,
        install_path,
        installed: state & 4 != 0 || state == 0, // StateFlags 4 = fully installed
        pinned: false,
        capabilities: None,
        recommendations: None,
        drs_profile: None,
    })
}

/// Minimal VDF/ACF key-value parser (flat, no nesting)
fn parse_simple_vdf(contents: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();

    for line in contents.lines() {
        let trimmed = line.trim().trim_matches('\t');
        // Match lines like: "key"		"value"
        let parts: Vec<&str> = trimmed.splitn(2, '"').collect::<Vec<_>>();
        if parts.len() < 2 {
            continue;
        }

        // Re-parse properly: find all quoted strings
        let quoted: Vec<&str> = trimmed
            .split('"')
            .enumerate()
            .filter_map(|(i, s)| if i % 2 == 1 { Some(s) } else { None })
            .collect();

        if quoted.len() == 2 {
            map.insert(quoted[0].to_lowercase(), quoted[1].to_string());
        }
    }

    map
}
