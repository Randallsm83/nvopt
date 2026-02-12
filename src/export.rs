use anyhow::Result;
use std::path::Path;

use crate::models::{DrsProfile, Game};

/// Export NVIDIA Profile Inspector .nip format (XML) for given games.
/// Prefers real DRS profiles; falls back to legacy recommendations only if DRS is missing.
pub fn export_nip(games: &[&Game], output_path: &Path) -> Result<()> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"utf-16\"?>\n");
    xml.push_str("<ArrayOfProfile>\n");

    for game in games {
        if let Some(ref drs) = game.drs_profile {
            xml.push_str(&format_profile(game.name.as_str(), drs));
        } else if let Some(_) = game.recommendations {
            // Legacy path (kept to avoid breaking export hotkey); create a minimal placeholder profile
            let placeholder = DrsProfile { settings: vec![], notes: vec![] };
            xml.push_str(&format_profile(game.name.as_str(), &placeholder));
        }
    }

    xml.push_str("</ArrayOfProfile>\n");

    std::fs::write(output_path, &xml)?;
    Ok(())
}

fn format_profile(game_name: &str, drs: &DrsProfile) -> String {
    let mut profile = String::new();
    profile.push_str("  <Profile>\n");
    profile.push_str(&format!(
        "    <ProfileName>{}</ProfileName>\n",
        escape_xml(game_name)
    ));
    profile.push_str("    <Executeables />\n");
    profile.push_str("    <Settings>\n");

    for s in &drs.settings {
        profile.push_str(&format_setting(
            &format!("0x{:08X}", s.id),
            &s.name,
            &format!("0x{:08X}", s.value),
        ));
    }

    profile.push_str("    </Settings>\n");
    profile.push_str("  </Profile>\n");
    profile
}

fn format_setting(id: &str, name: &str, value: &str) -> String {
    format!(
        "      <ProfileSetting>\n\
         \x20       <SettingNameInfo>{name}</SettingNameInfo>\n\
         \x20       <SettingID>{id}</SettingID>\n\
         \x20       <SettingValue>{value}</SettingValue>\n\
         \x20     </ProfileSetting>\n"
    )
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
