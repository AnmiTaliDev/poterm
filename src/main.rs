// Poterm - Modern TUI editor for .po translation files
// Copyright (c) 2025 AnmiTaliDev <anmitali198@gmail.com>
// Licensed under the Apache License, Version 2.0

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, stdout};
use std::path::PathBuf;

mod gettext;
mod ui;

use gettext::PoFile;
use ui::App;

#[derive(Parser)]
#[command(
    name = "poterm",
    version = env!("CARGO_PKG_VERSION"),
    author = "AnmiTaliDev <anmitali198@gmail.com>",
    about = "Modern TUI editor for .po translation files"
)]
struct Cli {
    /// Path to the .po file to edit
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,

    /// Create new .po file if it doesn't exist
    #[arg(short, long)]
    create: bool,

    /// Create .po file from .pot template
    #[arg(long, value_name = "POT_FILE")]
    from_pot: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup terminal
    enable_raw_mode().context("Failed to enable raw mode")?;
    stdout().execute(EnterAlternateScreen).context("Failed to enter alternate screen")?;
    
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend).context("Failed to create terminal")?;

    let result = run_app(&mut terminal, cli);

    // Cleanup terminal
    disable_raw_mode().context("Failed to disable raw mode")?;
    stdout().execute(LeaveAlternateScreen).context("Failed to leave alternate screen")?;

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, cli: Cli) -> Result<()> {
    let po_file = match (cli.file, cli.from_pot) {
        (Some(path), Some(pot_path)) => {
            // Create .po from .pot template
            PoFile::from_pot_template(&pot_path, &path)
                .context("Failed to create .po file from .pot template")?
        }
        (Some(path), None) => {
            if path.exists() {
                PoFile::from_file(&path).context("Failed to load .po file")?
            } else if cli.create {
                PoFile::new(path)
            } else {
                anyhow::bail!("File does not exist. Use --create to create a new file or --from-pot to create from template.");
            }
        }
        (None, Some(_pot_path)) => {
            anyhow::bail!("Please specify output .po file path when using --from-pot");
        }
        (None, None) => PoFile::default(),
    };

    let mut app = App::new(po_file);

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if handle_key_event(&mut app, key)? {
                break;
            }
        }
    }

    // Save file if modified
    if app.is_modified() {
        app.save().context("Failed to save file")?;
    }

    Ok(())
}

fn handle_key_event(app: &mut App, key: KeyEvent) -> Result<bool> {
    // Debug: print key events to help diagnose issues
    // eprintln!("Key: {:?} {:?}", key.modifiers, key.code);
    
    match (key.modifiers, key.code) {
        // Quit
        (KeyModifiers::CONTROL, KeyCode::Char('q')) => return Ok(true),
        
        // Save
        (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
            app.save()?;
        }
        
        // Save current entry (Ctrl+Shift+P)
        (KeyModifiers::CONTROL | KeyModifiers::SHIFT, KeyCode::Char('p')) => {
            app.save_current_entry()?;
        }
        
        // Navigation
        (KeyModifiers::NONE, KeyCode::Up) | (KeyModifiers::NONE, KeyCode::Char('k')) => {
            if app.is_metadata_mode() {
                app.metadata_previous();
            } else {
                app.previous_entry();
            }
        }
        (KeyModifiers::NONE, KeyCode::Down) | (KeyModifiers::NONE, KeyCode::Char('j')) => {
            if app.is_metadata_mode() {
                app.metadata_next();
            } else {
                app.next_entry();
            }
        }
        (KeyModifiers::NONE, KeyCode::PageUp) => {
            app.page_up();
        }
        (KeyModifiers::NONE, KeyCode::PageDown) => {
            app.page_down();
        }
        (KeyModifiers::NONE, KeyCode::Home) => {
            app.go_to_first();
        }
        (KeyModifiers::NONE, KeyCode::End) => {
            app.go_to_last();
        }
        
        // Edit mode
        (KeyModifiers::NONE, KeyCode::Enter) | (KeyModifiers::NONE, KeyCode::Char('i')) => {
            if app.is_metadata_mode() {
                app.start_editing_selected_metadata();
            } else {
                app.start_editing();
            }
        }
        (KeyModifiers::NONE, KeyCode::Esc) => {
            if app.help_visible {
                app.toggle_help();
            } else {
                app.stop_editing();
            }
        }
        
        // Tab switching
        (KeyModifiers::NONE, KeyCode::Tab) => {
            app.next_field();
        }
        (KeyModifiers::SHIFT, KeyCode::BackTab) => {
            app.previous_field();
        }
        
        // Search
        (KeyModifiers::CONTROL, KeyCode::Char('f')) => {
            app.start_search();
        }
        (KeyModifiers::NONE, KeyCode::F(3)) => {
            app.find_next();
        }
        (KeyModifiers::SHIFT, KeyCode::F(3)) => {
            app.find_previous();
        }
        
        // Toggle fuzzy/untranslated filter
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            app.toggle_untranslated_filter();
        }
        (KeyModifiers::CONTROL, KeyCode::Char('z')) => {
            app.toggle_fuzzy_filter();
        }
        
        // Help
        (KeyModifiers::NONE, KeyCode::F(1)) => {
            app.toggle_help();
        }

        // F9 for metadata mode
        (KeyModifiers::NONE, KeyCode::F(9)) => {
            app.toggle_metadata_mode();
        }

        // Toggle fuzzy status
        (KeyModifiers::NONE, KeyCode::F(2)) => {
            app.toggle_current_entry_fuzzy();
        }

        // Mark entry as done (remove fuzzy flag)
        (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
            app.mark_current_entry_done();
        }

        // Alternative fuzzy toggle with Ctrl+T (T for Toggle)
        (KeyModifiers::CONTROL, KeyCode::Char('t')) => {
            app.toggle_current_entry_fuzzy();
        }
        
        // Handle text input when editing
        _ => {
            if app.is_editing() {
                app.handle_input(key);
            }
        }
    }
    
    Ok(false)
}