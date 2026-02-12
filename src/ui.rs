use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, AppMode, ListSection};
use crate::models::Game;

const ACCENT: Color = Color::Rgb(0xB3, 0xA1, 0xE6); // spaceduck purple
const HIGHLIGHT: Color = Color::Rgb(0x5C, 0xCC, 0x96); // spaceduck green
const DIM: Color = Color::Rgb(0x68, 0x68, 0x80);
const WARN: Color = Color::Rgb(0xE3, 0x94, 0x00); // spaceduck orange

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title
            Constraint::Min(10),   // Main content
            Constraint::Length(3), // Status/search bar
        ])
        .split(f.area());

    draw_title(f, chunks[0]);
    draw_main(f, app, chunks[1]);
    draw_status(f, app, chunks[2]);
}

fn draw_title(f: &mut Frame, area: Rect) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(" nvopt ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("│ GPU Game Settings Optimizer", Style::default().fg(DIM)),
    ]));
    f.render_widget(title, area);
}

fn draw_main(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    draw_game_list(f, app, chunks[0]);
    draw_detail_panel(f, app, chunks[1]);
}

fn draw_game_list(f: &mut Frame, app: &App, area: Rect) {
    let mut items: Vec<ListItem> = Vec::new();

    // Section: Installed
    let installed_active = matches!(app.active_section, ListSection::Installed);
    let header_style = if installed_active {
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(DIM)
    };
    items.push(ListItem::new(Line::from(Span::styled(
        format!("─ Installed ({}) ─", app.installed_games.len()),
        header_style,
    ))));
    for (i, game) in app.installed_games.iter().enumerate() {
        let selected = installed_active && i == app.selected_index;
        items.push(make_game_item(game, selected, true));
    }

    // Section: Watchlist
    if !app.watchlist_games.is_empty() {
        let wl_active = matches!(app.active_section, ListSection::Watchlist);
        let header_style = if wl_active {
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(DIM)
        };
        items.push(ListItem::new(Line::from(Span::styled(
            format!("─ Watchlist ({}) ─", app.watchlist_games.len()),
            header_style,
        ))));
        for (i, game) in app.watchlist_games.iter().enumerate() {
            let selected = wl_active && i == app.selected_index;
            items.push(make_game_item(game, selected, false));
        }
    }

    // Section: Search Results
    if !app.search_results.is_empty() {
        let sr_active = matches!(app.active_section, ListSection::SearchResults);
        let header_style = if sr_active {
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(DIM)
        };
        items.push(ListItem::new(Line::from(Span::styled(
            format!("─ Search ({}) ─", app.search_results.len()),
            header_style,
        ))));
        for (i, game) in app.search_results.iter().enumerate() {
            let selected = sr_active && i == app.selected_index;
            items.push(make_game_item(game, selected, false));
        }
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(DIM)),
    );
    f.render_widget(list, area);
}

fn make_game_item(game: &Game, selected: bool, show_icon: bool) -> ListItem<'static> {
    let prefix = if selected { "▸ " } else { "  " };
    let icon = if show_icon {
        format!("{} ", game.platform.icon())
    } else {
        String::new()
    };
    let has_data = if game.capabilities.is_some() { "●" } else { "○" };

    let style = if selected {
        Style::default().fg(HIGHLIGHT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let data_style = if game.capabilities.is_some() {
        Style::default().fg(HIGHLIGHT)
    } else {
        Style::default().fg(DIM)
    };

    ListItem::new(Line::from(vec![
        Span::styled(prefix.to_string(), style),
        Span::styled(icon, style),
        Span::styled(game.name.clone(), style),
        Span::raw(" "),
        Span::styled(has_data.to_string(), data_style),
    ]))
}

fn draw_detail_panel(f: &mut Frame, app: &App, area: Rect) {
    let game = match app.selected_game() {
        Some(g) => g,
        None => {
            let empty = Paragraph::new("Select a game and press Enter to fetch recommendations")
                .style(Style::default().fg(DIM))
                .block(Block::default().borders(Borders::NONE));
            f.render_widget(empty, area);
            return;
        }
    };

    let mut lines: Vec<Line<'static>> = Vec::new();

    // Game header
    lines.push(Line::from(Span::styled(
        game.name.clone(),
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
    )));

    if let Some(caps) = game.capabilities.clone() {
        // Developer
        if let Some(ref dev) = caps.developers {
            lines.push(Line::from(vec![
                Span::styled("Developer: ", Style::default().fg(DIM)),
                Span::raw(dev.clone()),
            ]));
        }

        lines.push(Line::from(""));

        // Capabilities
        lines.push(Line::from(Span::styled(
            "─ Capabilities ─",
            Style::default().fg(ACCENT),
        )));
        add_cap_line(&mut lines, "4K", caps.supports_4k);
        add_cap_line(&mut lines, "HDR", caps.supports_hdr);
        add_cap_line(&mut lines, "Ultrawide", caps.supports_ultrawide);
        add_cap_line(&mut lines, "DLSS", caps.supports_dlss);
        add_cap_line(&mut lines, "FSR", caps.supports_fsr);
        add_cap_line(&mut lines, "Ray Tracing", caps.supports_ray_tracing);
        add_cap_line(&mut lines, "Frame Gen", caps.supports_frame_gen);

        // DRS (driver) settings
        if let Some(drs) = game.drs_profile.clone() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "─ DRS (Driver) Settings ─",
                Style::default().fg(HIGHLIGHT),
            )));
            for s in drs.settings.iter() {
                let v = s
                    .value_name
                    .as_ref()
                    .map(|x| x.as_str())
                    .unwrap_or("");
                let shown = if v.is_empty() {
                    format!("0x{:08X}", s.value)
                } else {
                    format!("{} (0x{:08X})", v, s.value)
                };
                add_setting_line(&mut lines, &s.name, &shown);
            }
            if !drs.notes.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "─ Notes ─",
                    Style::default().fg(WARN),
                )));
                for note in &drs.notes {
                    lines.push(Line::from(Span::styled(
                        format!("  {note}"),
                        Style::default().fg(WARN),
                    )));
                }
            }
        } else if let Some(rec) = game.recommendations.clone() {
            // Legacy fallback until all panels are ported
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "─ Recommended Settings ─",
                Style::default().fg(HIGHLIGHT),
            )));

            add_setting_line(&mut lines, "Preset", &rec.overall_preset.to_string());
            add_setting_line(&mut lines, "Resolution", &rec.resolution_scale.to_string());
            add_setting_line(&mut lines, "Upscaling", &rec.upscaling.to_string());
            add_setting_line(&mut lines, "Textures", &rec.texture_quality.to_string());
            add_setting_line(&mut lines, "Shadows", &rec.shadow_quality.to_string());
            add_setting_line(&mut lines, "Ray Tracing", &rec.ray_tracing.to_string());
            add_setting_line(&mut lines, "Anti-Aliasing", &rec.anti_aliasing.to_string());
            add_setting_line(
                &mut lines,
                "Frame Gen",
                if rec.frame_gen { "On" } else { "Off" },
            );
            add_setting_line(
                &mut lines,
                "V-Sync",
                if rec.vsync { "On" } else { "Off (G-SYNC)" },
            );
            if let Some(fps) = rec.max_fps {
                add_setting_line(&mut lines, "Max FPS", &format!("{fps} Hz"));
            }

            // Notes
            if !rec.notes.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "─ Notes ─",
                    Style::default().fg(WARN),
                )));
                for note in &rec.notes {
                    lines.push(Line::from(Span::styled(
                        format!("  {note}"),
                        Style::default().fg(WARN),
                    )));
                }
            }
        }
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press Enter to fetch game info & recommendations",
            Style::default().fg(DIM),
        )));
    }

    // Keybinds help
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "─ Keys ─",
        Style::default().fg(DIM),
    )));
    lines.push(Line::from(Span::styled(
        "  Enter: fetch │ /: search │ Tab: section │ p: pin │ e: export │ o: PCGW │ q: quit",
        Style::default().fg(DIM),
    )));

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, area);
}

fn add_cap_line(lines: &mut Vec<Line<'static>>, label: &str, value: Option<bool>) {
    let (icon, color) = match value {
        Some(true) => ("✓", HIGHLIGHT),
        Some(false) => ("✗", Color::Red),
        None => ("?", DIM),
    };
    lines.push(Line::from(vec![
        Span::styled(format!("  {icon} "), Style::default().fg(color)),
        Span::raw(label.to_string()),
    ]));
}

fn add_setting_line(lines: &mut Vec<Line<'static>>, label: &str, value: &str) {
    lines.push(Line::from(vec![
        Span::styled(
            format!("  {label}: "),
            Style::default().fg(DIM),
        ),
        Span::styled(
            value.to_string(),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ),
    ]));
}

fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    match app.mode {
        AppMode::Search => {
            let input = Paragraph::new(Line::from(vec![
                Span::styled(" Search: ", Style::default().fg(ACCENT)),
                Span::raw(&app.search_input),
                Span::styled("█", Style::default().fg(HIGHLIGHT)),
            ]))
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(ACCENT)),
            );
            f.render_widget(input, area);
        }
        AppMode::Normal => {
            let status = Paragraph::new(Line::from(Span::styled(
                format!(" {}", &app.status_message),
                Style::default().fg(DIM),
            )))
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(DIM)),
            );
            f.render_widget(status, area);
        }
    }
}
