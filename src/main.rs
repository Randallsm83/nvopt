mod app;
mod config;
mod export;
mod hw;
mod library;
mod models;
mod pcgw;
mod recommend;
mod drs;
mod search;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use std::io::stdout;

use app::{App, AppMode};

#[tokio::main]
async fn main() -> Result<()> {
    // Load config
    let config = config::load_config()?;

    // Detect hardware
    eprintln!("Detecting hardware...");
    let hw = match hw::detect_hardware() {
        Ok(h) => {
            eprintln!("  GPU: {} ({}MB)", h.gpu_name, h.gpu_vram_mb);
            eprintln!("  CPU: {}", h.cpu_name);
            eprintln!("  Display: {}x{}@{}Hz", h.display_resolution.0, h.display_resolution.1, h.display_refresh_hz);
            eprintln!("  Tier: {}", h.gpu_tier.label());
            Some(h)
        }
        Err(e) => {
            eprintln!("Warning: Hardware detection failed: {e}");
            None
        }
    };

    // Detect games
    eprintln!("Scanning Steam library...");
    let games = library::detect_steam_games(&config.steam_paths).unwrap_or_else(|e| {
        eprintln!("Warning: Steam library scan failed: {e}");
        Vec::new()
    });
    eprintln!("  Found {} games", games.len());

    // Create app
    let mut app = App::new(config, hw, games);

    // Setup terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    let result = run_loop(&mut terminal, &mut app).await;

    // Cleanup
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    result
}

async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match &app.mode {
                    AppMode::Search => match key.code {
                        KeyCode::Enter => {
                            app.execute_search().await;
                        }
                        KeyCode::Esc => {
                            app.exit_search();
                        }
                        KeyCode::Backspace => {
                            app.search_input.pop();
                        }
                        KeyCode::Char(c) => {
                            app.search_input.push(c);
                        }
                        _ => {}
                    },
                    AppMode::Normal => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            app.should_quit = true;
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            app.move_down();
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.move_up();
                        }
                        KeyCode::Tab => {
                            app.next_section();
                        }
                        KeyCode::Char('/') => {
                            app.enter_search();
                        }
                        KeyCode::Enter => {
                            app.fetch_capabilities_for_selected().await;
                        }
                        KeyCode::Char('p') => {
                            app.pin_selected();
                        }
                        KeyCode::Char('e') => {
                            match app.export_selected() {
                                Ok(()) => {
                                    let name = app
                                        .selected_game()
                                        .map(|g| g.name.as_str())
                                        .unwrap_or("game");
                                    app.status_message =
                                        format!("Exported DRS (.nip) profile for {name}");
                                }
                                Err(e) => {
                                    app.status_message = format!("Export error: {e}");
                                }
                            }
                        }
                        KeyCode::Char('o') => {
                            app.open_pcgw();
                        }
                        KeyCode::Char('r') => {
                            // Re-fetch capabilities for selected
                            let idx = app.selected_index;
                            let list = app.active_list_mut();
                            if let Some(g) = list.get_mut(idx) {
                                g.capabilities = None;
                                g.recommendations = None;
                            }
                            app.fetch_capabilities_for_selected().await;
                        }
                        _ => {}
                    },
                }

                if app.should_quit {
                    return Ok(());
                }
            }
        }
    }
}
