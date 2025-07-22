// Poterm - Modern TUI editor for .po translation files
// Copyright (c) 2025 AnmiTaliDev <anmitali198@gmail.com>
// Licensed under the Apache License, Version 2.0

use crate::gettext::{PoEntry, PoFile};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap,
    },
    Frame,
};
use std::cmp::min;
use unicode_width::UnicodeWidthStr;

// UI Constants
const ENTRY_LIST_WIDTH_PERCENT: u16 = 40;
const ENTRY_DETAILS_WIDTH_PERCENT: u16 = 60;
const PAGE_SIZE: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditField {
    Msgid,
    Msgstr,
    Comments,
    Metadata,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterMode {
    All,
    Untranslated,
    Fuzzy,
}

pub struct App {
    po_file: PoFile,
    current_entry: usize,
    list_state: ListState,
    editing: bool,
    edit_field: EditField,
    edit_text: String,
    edit_cursor: usize,
    search_mode: bool,
    search_query: String,
    search_cursor: usize,
    filter_mode: FilterMode,
    filtered_indices: Vec<usize>,
    pub help_visible: bool,
    metadata_mode: bool,
    metadata_key: String,
    metadata_keys: Vec<String>,
    metadata_selected: usize,
}

impl App {
    // Helper function to convert character index to byte index
    fn char_to_byte_index(text: &str, char_idx: usize) -> usize {
        text.char_indices().nth(char_idx).map(|(i, _)| i).unwrap_or(text.len())
    }
    
    // Helper function to convert byte index to character index
    fn byte_to_char_index(text: &str, byte_idx: usize) -> usize {
        text.char_indices().take_while(|(i, _)| *i < byte_idx).count()
    }
    
    // Optimized helper to remove character at specific index
    fn remove_char_at(text: &mut String, char_idx: usize) {
        if let Some((start_byte, ch)) = text.char_indices().nth(char_idx) {
            let char_len = ch.len_utf8();
            text.drain(start_byte..start_byte + char_len);
        }
    }
    
    // Helper to insert character at specific position
    fn insert_char_at(text: &mut String, char_idx: usize, ch: char) {
        let byte_pos = Self::char_to_byte_index(text, char_idx);
        text.insert(byte_pos, ch);
    }

    pub fn new(po_file: PoFile) -> Self {
        let mut app = Self {
            po_file,
            current_entry: 0,
            list_state: ListState::default(),
            editing: false,
            edit_field: EditField::Msgstr,
            edit_text: String::new(),
            edit_cursor: 0,
            search_mode: false,
            search_query: String::new(),
            search_cursor: 0,
            filter_mode: FilterMode::All,
            filtered_indices: Vec::new(),
            help_visible: false,
            metadata_mode: false,
            metadata_key: String::new(),
            metadata_keys: vec![
                "Project-Id-Version".to_string(),
                "Language".to_string(),
                "Language-Team".to_string(),
                "Last-Translator".to_string(),
                "Report-Msgid-Bugs-To".to_string(),
                "POT-Creation-Date".to_string(),
                "PO-Revision-Date".to_string(),
                "MIME-Version".to_string(),
                "Content-Type".to_string(),
                "Content-Transfer-Encoding".to_string(),
                "Plural-Forms".to_string(),
            ],
            metadata_selected: 0,
        };
        
        app.update_filtered_indices();
        app.update_list_state();
        app
    }

    fn update_filtered_indices(&mut self) {
        self.filtered_indices.clear();
        
        for (i, entry) in self.po_file.entries.iter().enumerate() {
            let matches_filter = match self.filter_mode {
                FilterMode::All => true,
                FilterMode::Untranslated => !entry.is_translated,
                FilterMode::Fuzzy => entry.is_fuzzy,
            };
            
            let matches_search = if self.search_query.is_empty() {
                true
            } else {
                entry.msgid.to_lowercase().contains(&self.search_query.to_lowercase()) ||
                entry.msgstr.to_lowercase().contains(&self.search_query.to_lowercase())
            };
            
            if matches_filter && matches_search {
                self.filtered_indices.push(i);
            }
        }
        
        // Adjust current_entry if needed
        if self.current_entry >= self.filtered_indices.len() && !self.filtered_indices.is_empty() {
            self.current_entry = self.filtered_indices.len() - 1;
        }
    }

    fn update_list_state(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.list_state.select(Some(self.current_entry));
        } else {
            self.list_state.select(None);
        }
    }

    pub fn next_entry(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.current_entry = min(self.current_entry + 1, self.filtered_indices.len() - 1);
            self.update_list_state();
        }
    }

    pub fn previous_entry(&mut self) {
        if self.current_entry > 0 {
            self.current_entry -= 1;
            self.update_list_state();
        }
    }

    pub fn page_up(&mut self) {
        if self.current_entry >= PAGE_SIZE {
            self.current_entry -= PAGE_SIZE;
        } else {
            self.current_entry = 0;
        }
        self.update_list_state();
    }

    pub fn page_down(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.current_entry = min(self.current_entry + PAGE_SIZE, self.filtered_indices.len() - 1);
            self.update_list_state();
        }
    }

    pub fn go_to_first(&mut self) {
        self.current_entry = 0;
        self.update_list_state();
    }

    pub fn go_to_last(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.current_entry = self.filtered_indices.len() - 1;
            self.update_list_state();
        }
    }

    pub fn start_editing(&mut self) {
        if !self.filtered_indices.is_empty() && !self.search_mode {
            let actual_index = self.filtered_indices[self.current_entry];
            if let Some(entry) = self.po_file.entries.get(actual_index) {
                self.editing = true;
                self.edit_text = match self.edit_field {
                    EditField::Msgid => entry.msgid.clone(),
                    EditField::Msgstr => entry.msgstr.clone(),
                    EditField::Comments => entry.comments.join("\n"),
                    EditField::Metadata => String::new(), // Handled in metadata mode
                };
                self.edit_cursor = self.edit_text.len();
            }
        }
    }

    pub fn stop_editing(&mut self) {
        if self.editing {
            self.apply_edit();
            self.editing = false;
        } else if self.search_mode {
            self.search_mode = false;
        }
    }

    fn apply_edit(&mut self) {
        if self.edit_field == EditField::Metadata {
            self.apply_metadata_edit();
        } else if let Some(&actual_index) = self.filtered_indices.get(self.current_entry) {
            if let Some(entry) = self.po_file.entries.get_mut(actual_index) {
                match self.edit_field {
                    EditField::Msgid => {
                        entry.msgid = self.edit_text.clone();
                    }
                    EditField::Msgstr => {
                        entry.set_msgstr(self.edit_text.clone());
                    }
                    EditField::Comments => {
                        entry.comments = self.edit_text.lines().map(|s| s.to_string()).collect();
                    }
                    EditField::Metadata => {
                        // Handled above
                    }
                }
                self.po_file.mark_modified();
            }
        }
    }

    pub fn next_field(&mut self) {
        if !self.editing && !self.metadata_mode {
            self.edit_field = match self.edit_field {
                EditField::Msgid => EditField::Msgstr,
                EditField::Msgstr => EditField::Comments,
                EditField::Comments => EditField::Msgid,
                EditField::Metadata => EditField::Metadata, // Stay in metadata mode
            };
        }
    }

    pub fn previous_field(&mut self) {
        if !self.editing && !self.metadata_mode {
            self.edit_field = match self.edit_field {
                EditField::Msgid => EditField::Comments,
                EditField::Msgstr => EditField::Msgid,
                EditField::Comments => EditField::Msgstr,
                EditField::Metadata => EditField::Metadata, // Stay in metadata mode
            };
        }
    }

    pub fn start_search(&mut self) {
        self.search_mode = true;
        self.search_cursor = self.search_query.len();
    }

    pub fn find_next(&mut self) {
        if !self.search_query.is_empty() {
            self.update_filtered_indices();
            self.next_entry();
            self.update_list_state();
        }
    }

    pub fn find_previous(&mut self) {
        if !self.search_query.is_empty() {
            self.update_filtered_indices();
            self.previous_entry();
            self.update_list_state();
        }
    }

    pub fn toggle_untranslated_filter(&mut self) {
        self.filter_mode = match self.filter_mode {
            FilterMode::Untranslated => FilterMode::All,
            _ => FilterMode::Untranslated,
        };
        self.update_filtered_indices();
        self.update_list_state();
    }

    pub fn toggle_fuzzy_filter(&mut self) {
        self.filter_mode = match self.filter_mode {
            FilterMode::Fuzzy => FilterMode::All,
            _ => FilterMode::Fuzzy,
        };
        self.update_filtered_indices();
        self.update_list_state();
    }

    pub fn handle_input(&mut self, key: KeyEvent) {
        if self.search_mode {
            self.handle_search_input(key);
        } else if self.editing {
            self.handle_edit_input(key);
        }
    }

    fn handle_search_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) => {
                Self::insert_char_at(&mut self.search_query, self.search_cursor, c);
                self.search_cursor += 1;
                self.update_filtered_indices();
                self.current_entry = 0;
                self.update_list_state();
            }
            KeyCode::Backspace => {
                if self.search_cursor > 0 {
                    self.search_cursor -= 1;
                    Self::remove_char_at(&mut self.search_query, self.search_cursor);
                    self.update_filtered_indices();
                    self.current_entry = 0;
                    self.update_list_state();
                }
            }
            KeyCode::Left => {
                if self.search_cursor > 0 {
                    self.search_cursor -= 1;
                }
            }
            KeyCode::Right => {
                let char_len = self.search_query.chars().count();
                if self.search_cursor < char_len {
                    self.search_cursor += 1;
                }
            }
            KeyCode::Enter => {
                self.search_mode = false;
            }
            _ => {}
        }
    }

    fn handle_edit_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) => {
                Self::insert_char_at(&mut self.edit_text, self.edit_cursor, c);
                self.edit_cursor += 1;
            }
            KeyCode::Backspace => {
                if self.edit_cursor > 0 {
                    self.edit_cursor -= 1;
                    Self::remove_char_at(&mut self.edit_text, self.edit_cursor);
                }
            }
            KeyCode::Delete => {
                let char_len = self.edit_text.chars().count();
                if self.edit_cursor < char_len {
                    Self::remove_char_at(&mut self.edit_text, self.edit_cursor);
                }
            }
            KeyCode::Left => {
                if self.edit_cursor > 0 {
                    self.edit_cursor -= 1;
                }
            }
            KeyCode::Right => {
                let char_len = self.edit_text.chars().count();
                if self.edit_cursor < char_len {
                    self.edit_cursor += 1;
                }
            }
            KeyCode::Home => {
                self.edit_cursor = 0;
            }
            KeyCode::End => {
                self.edit_cursor = self.edit_text.chars().count();
            }
            KeyCode::Enter => {
                if self.edit_field == EditField::Comments {
                    Self::insert_char_at(&mut self.edit_text, self.edit_cursor, '\n');
                    self.edit_cursor += 1;
                } else {
                    self.apply_edit();
                    self.editing = false;
                }
            }
            _ => {}
        }
    }

    pub fn is_editing(&self) -> bool {
        self.editing || self.search_mode
    }

    pub fn is_metadata_mode(&self) -> bool {
        self.metadata_mode
    }

    pub fn is_modified(&self) -> bool {
        self.po_file.is_modified()
    }

    pub fn save(&mut self) -> Result<()> {
        self.po_file.save()
    }
    
    pub fn save_current_entry(&mut self) -> Result<()> {
        self.apply_edit();
        self.po_file.save()
    }

    pub fn toggle_help(&mut self) {
        self.help_visible = !self.help_visible;
    }

    pub fn toggle_metadata_mode(&mut self) {
        if self.editing {
            return;
        }
        
        self.metadata_mode = !self.metadata_mode;
        if self.metadata_mode {
            self.edit_field = EditField::Metadata;
        } else {
            self.edit_field = EditField::Msgstr;
        }
    }

    pub fn start_metadata_editing(&mut self, key: String) {
        if !self.metadata_mode {
            return;
        }
        
        self.metadata_key = key.clone();
        self.edit_text = self.po_file.get_header()
            .get(&key)
            .cloned()
            .unwrap_or_default();
        self.edit_cursor = self.edit_text.chars().count();
        self.editing = true;
    }

    pub fn start_editing_selected_metadata(&mut self) {
        if self.metadata_mode && !self.metadata_keys.is_empty() && !self.editing {
            let key = self.metadata_keys[self.metadata_selected].clone();
            self.start_metadata_editing(key);
        }
    }

    pub fn metadata_next(&mut self) {
        if self.metadata_mode && !self.editing {
            if self.metadata_selected + 1 < self.metadata_keys.len() {
                self.metadata_selected += 1;
            }
        }
    }

    pub fn metadata_previous(&mut self) {
        if self.metadata_mode && !self.editing {
            if self.metadata_selected > 0 {
                self.metadata_selected -= 1;
            }
        }
    }

    fn apply_metadata_edit(&mut self) {
        if self.metadata_mode && !self.metadata_key.is_empty() {
            self.po_file.set_header_field(self.metadata_key.clone(), self.edit_text.clone());
            self.po_file.update_revision_date();
        }
    }

    pub fn toggle_current_entry_fuzzy(&mut self) {
        if !self.filtered_indices.is_empty() && !self.editing && !self.search_mode {
            let actual_index = self.filtered_indices[self.current_entry];
            if let Some(entry) = self.po_file.entries.get_mut(actual_index) {
                // Don't toggle fuzzy status for empty entries (no translation)
                if entry.msgstr.is_empty() {
                    return;
                }
                
                entry.toggle_fuzzy();
                self.po_file.mark_modified();
                self.po_file.update_revision_date();
            }
        }
    }

    pub fn mark_current_entry_done(&mut self) {
        if !self.filtered_indices.is_empty() && !self.editing && !self.search_mode {
            let actual_index = self.filtered_indices[self.current_entry];
            if let Some(entry) = self.po_file.entries.get_mut(actual_index) {
                // Only mark as done if there's a translation
                if !entry.msgstr.is_empty() {
                    entry.flags.retain(|flag| flag != "fuzzy");
                    entry.update_status();
                    self.po_file.mark_modified();
                    self.po_file.update_revision_date();
                }
            }
        }
    }

    fn get_current_entry(&self) -> Option<&PoEntry> {
        if let Some(&actual_index) = self.filtered_indices.get(self.current_entry) {
            self.po_file.entries.get(actual_index)
        } else {
            None
        }
    }
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Footer
        ])
        .split(f.area());

    // Draw header
    draw_header(f, chunks[0], app);

    // Draw main content based on mode
    if app.metadata_mode {
        draw_metadata_panel(f, chunks[1], app);
    } else {
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(ENTRY_LIST_WIDTH_PERCENT), 
                Constraint::Percentage(ENTRY_DETAILS_WIDTH_PERCENT)
            ])
            .split(chunks[1]);

        draw_entry_list(f, main_chunks[0], app);
        draw_entry_details(f, main_chunks[1], app);
    }

    // Draw footer
    draw_footer(f, chunks[2], app);

    // Draw search overlay
    if app.search_mode {
        draw_search_overlay(f, app);
    }

    // Draw help overlay
    if app.help_visible {
        draw_help_overlay(f);
    }
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let (total, translated, fuzzy) = app.po_file.get_stats();
    let untranslated = total - translated - fuzzy;
    
    let progress = if total > 0 {
        (translated as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    let title = if let Some(ref path) = app.po_file.path {
        format!(
            "Poterm - {} {}",
            path.file_name().unwrap_or_default().to_string_lossy(),
            if app.po_file.is_modified() { "*" } else { "" }
        )
    } else {
        "Poterm - New File".to_string()
    };

    let stats = format!(
        "Total: {} | Translated: {} ({:.1}%) | Fuzzy: {} | Untranslated: {}",
        total, translated, progress, fuzzy, untranslated
    );

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(stats)
        .block(block)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));

    f.render_widget(paragraph, area);
}

fn draw_entry_list(f: &mut Frame, area: Rect, app: &mut App) {
    let items: Vec<ListItem> = app
        .filtered_indices
        .iter()
        .enumerate()
        .map(|(_i, &actual_index)| {
            let entry = &app.po_file.entries[actual_index];
            let status_char = if entry.is_fuzzy {
                "~"
            } else if entry.is_translated {
                "✓"
            } else {
                "○"
            };

            let color = if entry.is_fuzzy {
                Color::Yellow
            } else if entry.is_translated {
                Color::Green
            } else {
                Color::Red
            };

            let msgid_preview = if entry.msgid.len() > 35 {
                format!("{}...", &entry.msgid[..32])
            } else {
                entry.msgid.clone()
            };

            let line = Line::from(vec![
                Span::styled(format!("{} ", status_char), Style::default().fg(color)),
                Span::raw(format!("{:3} ", actual_index + 1)),
                Span::raw(msgid_preview),
            ]);

            ListItem::new(line)
        })
        .collect();

    let filter_text = match app.filter_mode {
        FilterMode::All => "All",
        FilterMode::Untranslated => "Untranslated",
        FilterMode::Fuzzy => "Fuzzy",
    };

    let title = format!("Entries [{}]", filter_text);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("► ");

    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn draw_entry_details(f: &mut Frame, area: Rect, app: &App) {
    if let Some(entry) = app.get_current_entry() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),  // Msgid
                Constraint::Length(5),  // Msgstr
                Constraint::Min(3),     // Comments
                Constraint::Length(3),  // References and flags
            ])
            .split(area);

        // Draw msgid
        draw_text_field(
            f,
            chunks[0],
            "Original Text (msgid)",
            &entry.msgid,
            app.edit_field == EditField::Msgid,
            app.editing && app.edit_field == EditField::Msgid,
            &app.edit_text,
            app.edit_cursor,
        );

        // Draw msgstr
        draw_text_field(
            f,
            chunks[1],
            "Translation (msgstr)",
            &entry.msgstr,
            app.edit_field == EditField::Msgstr,
            app.editing && app.edit_field == EditField::Msgstr,
            &app.edit_text,
            app.edit_cursor,
        );

        // Draw comments
        let comments_text = entry.comments.join("\n");
        draw_text_field(
            f,
            chunks[2],
            "Comments",
            &comments_text,
            app.edit_field == EditField::Comments,
            app.editing && app.edit_field == EditField::Comments,
            &app.edit_text,
            app.edit_cursor,
        );

        // Draw references and flags
        let mut info_lines = Vec::new();
        if !entry.references.is_empty() {
            info_lines.push(Line::from(vec![
                Span::styled("References: ", Style::default().fg(Color::Cyan)),
                Span::raw(entry.references.join(", ")),
            ]));
        }
        if !entry.flags.is_empty() {
            info_lines.push(Line::from(vec![
                Span::styled("Flags: ", Style::default().fg(Color::Yellow)),
                Span::raw(entry.flags.join(", ")),
            ]));
        }

        let block = Block::default()
            .title("Information")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta));

        let paragraph = Paragraph::new(info_lines)
            .block(block)
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, chunks[3]);
    } else {
        let block = Block::default()
            .title("Entry Details")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray));

        let paragraph = Paragraph::new("No entry selected")
            .block(block)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));

        f.render_widget(paragraph, area);
    }
}

fn draw_text_field(
    f: &mut Frame,
    area: Rect,
    title: &str,
    text: &str,
    is_selected: bool,
    is_editing: bool,
    edit_text: &str,
    cursor_pos: usize,
) {
    let border_color = if is_editing {
        Color::Green
    } else if is_selected {
        Color::Yellow
    } else {
        Color::White
    };

    let display_text = if is_editing { edit_text } else { text };

    let block = Block::default()
        .title(format!("{}{}", title, if is_editing { " (editing)" } else { "" }))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner_area = block.inner(area);
    
    let paragraph = Paragraph::new(display_text)
        .block(block)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White));

    f.render_widget(paragraph, area);

    // Draw cursor if editing
    if is_editing {
        // Convert character index to byte index for slicing
        let byte_pos = if cursor_pos <= display_text.chars().count() {
            display_text.char_indices().nth(cursor_pos).map(|(i, _)| i).unwrap_or(display_text.len())
        } else {
            display_text.len()
        };
        
        let text_width = display_text[..byte_pos].width();
        let cursor_x = inner_area.x + (text_width as u16) % inner_area.width;
        let cursor_y = inner_area.y + (text_width as u16) / inner_area.width;
        
        if cursor_x < inner_area.x + inner_area.width && cursor_y < inner_area.y + inner_area.height {
            f.render_widget(
                Block::default().style(Style::default().bg(Color::White)),
                Rect {
                    x: cursor_x,
                    y: cursor_y,
                    width: 1,
                    height: 1,
                },
            );
        }
    }
}

fn draw_metadata_panel(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),  // Metadata keys list
            Constraint::Percentage(60),  // Value editor
        ])
        .split(area);
    
    // Draw metadata keys list
    let keys_items: Vec<ListItem> = app.metadata_keys
        .iter()
        .enumerate()
        .map(|(i, key)| {
            let current_value = app.po_file.get_header()
                .get(key)
                .cloned()
                .unwrap_or_default();
            
            let display_value = if current_value.len() > 30 {
                format!("{}...", &current_value[..27])
            } else {
                current_value
            };
            
            let prefix = if i == app.metadata_selected { "► " } else { "  " };
            ListItem::new(format!("{}{}: {}", prefix, key, display_value))
        })
        .collect();
    
    let keys_list = List::new(keys_items)
        .block(
            Block::default()
                .title("Metadata Fields")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
        )
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black));
    
    f.render_widget(keys_list, chunks[0]);
    
    // Draw value editor
    if app.metadata_selected < app.metadata_keys.len() {
        let selected_key = &app.metadata_keys[app.metadata_selected];
        let current_value = app.po_file.get_header()
            .get(selected_key)
            .cloned()
            .unwrap_or_default();
        
        let title = if app.editing && app.metadata_key == *selected_key {
            format!("{} (editing)", selected_key)
        } else {
            selected_key.clone()
        };
        
        let display_text = if app.editing && app.metadata_key == *selected_key {
            &app.edit_text
        } else {
            &current_value
        };
        
        let border_color = if app.editing && app.metadata_key == *selected_key {
            Color::Green
        } else {
            Color::Blue
        };
        
        let paragraph = Paragraph::new(display_text.as_str())
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
            )
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(Color::White));
        
        f.render_widget(paragraph, chunks[1]);
        
        // Draw cursor if editing
        if app.editing && app.metadata_key == *selected_key {
            let inner_area = Block::default().borders(Borders::ALL).inner(chunks[1]);
            
            // Convert character index to byte index for slicing
            let byte_pos = if app.edit_cursor <= display_text.chars().count() {
                display_text.char_indices().nth(app.edit_cursor).map(|(i, _)| i).unwrap_or(display_text.len())
            } else {
                display_text.len()
            };
            
            let text_width = display_text[..byte_pos].width();
            let cursor_x = inner_area.x + (text_width as u16) % inner_area.width;
            let cursor_y = inner_area.y + (text_width as u16) / inner_area.width;
            
            if cursor_x < inner_area.x + inner_area.width && cursor_y < inner_area.y + inner_area.height {
                f.render_widget(
                    Block::default().style(Style::default().bg(Color::White)),
                    Rect {
                        x: cursor_x,
                        y: cursor_y,
                        width: 1,
                        height: 1,
                    },
                );
            }
        }
    }
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let help_text = if app.search_mode {
        "Search mode: Type to search, Enter to finish, Esc to cancel"
    } else if app.editing {
        "Edit mode: Type to edit, Enter to save, Esc to cancel"
    } else if app.metadata_mode {
        "Metadata mode: ↑/↓/j/k: Navigate fields | Enter/i: Edit selected | Esc: Cancel | F9: Exit | Ctrl+S: Save | F1: Help"
    } else {
        "Ctrl+Q: Quit | Ctrl+S: Save | Enter: Edit | F2/Ctrl+T: Toggle fuzzy | Ctrl+D: Mark done | F9: Metadata | F1: Help"
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray));

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));

    f.render_widget(paragraph, area);
}

fn draw_search_overlay(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 3, f.area());
    
    f.render_widget(Clear, area);
    
    let block = Block::default()
        .title("Search")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let search_text = format!("{}{}", app.search_query, 
        if app.search_cursor == app.search_query.len() { "█" } else { "" });

    let paragraph = Paragraph::new(search_text)
        .block(block)
        .style(Style::default().fg(Color::White));

    f.render_widget(paragraph, area);
}

fn draw_help_overlay(f: &mut Frame) {
    let area = centered_rect(80, 25, f.area());
    
    f.render_widget(Clear, area);
    
    let help_text = vec![
        Line::from("Navigation:"),
        Line::from("  j/↓        - Next entry"),
        Line::from("  k/↑        - Previous entry"),
        Line::from("  PageUp     - Page up"),
        Line::from("  PageDown   - Page down"),
        Line::from("  Home       - First entry"),
        Line::from("  End        - Last entry"),
        Line::from(""),
        Line::from("Editing:"),
        Line::from("  i/Enter    - Start editing"),
        Line::from("  Esc        - Stop editing"),
        Line::from("  Tab        - Next field"),
        Line::from("  Shift+Tab  - Previous field"),
        Line::from(""),
        Line::from("Translation Status:"),
        Line::from("  F2/Ctrl+T  - Toggle fuzzy status"),
        Line::from("  Ctrl+D     - Mark entry as done"),
        Line::from(""),
        Line::from("Metadata Editing:"),
        Line::from("  F9         - Enter/exit metadata mode"),
        Line::from("  ↑/↓        - Navigate fields (in metadata mode)"),
        Line::from("  Enter      - Edit selected field"),
        Line::from(""),
        Line::from("Search & Filter:"),
        Line::from("  Ctrl+F     - Search"),
        Line::from("  F3         - Find next"),
        Line::from("  Shift+F3   - Find previous"),
        Line::from("  Ctrl+U     - Toggle untranslated filter"),
        Line::from("  Ctrl+Z     - Toggle fuzzy filter"),
        Line::from(""),
        Line::from("File Operations:"),
        Line::from("  Ctrl+S     - Save file"),
        Line::from("  Ctrl+Shift+P - Save current entry"),
        Line::from("  Ctrl+Q     - Quit"),
        Line::from(""),
        Line::from("Press Esc to close this help"),
    ];

    let block = Block::default()
        .title("Help")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .style(Style::default().fg(Color::White));

    f.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height - height) / 2),
            Constraint::Length(height),
            Constraint::Length((r.height - height) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gettext::{PoFile, PoEntry};

    #[test]
    fn test_char_to_byte_index() {
        let text = "Hello мир world";  // Contains Cyrillic characters
        
        // Test ASCII characters
        assert_eq!(App::char_to_byte_index(text, 0), 0);  // 'H'
        assert_eq!(App::char_to_byte_index(text, 5), 5);  // Space after "Hello"
        
        // Test Unicode boundaries
        assert_eq!(App::char_to_byte_index(text, 6), 6);  // 'м' starts at byte 6
        assert_eq!(App::char_to_byte_index(text, 7), 8);  // 'и' starts at byte 8 (м is 2 bytes)
        assert_eq!(App::char_to_byte_index(text, 8), 10); // 'р' starts at byte 10 (и is 2 bytes)
        
        // Test beyond string length
        assert_eq!(App::char_to_byte_index(text, 100), text.len());
    }

    #[test]
    fn test_remove_char_at() {
        let mut text = String::from("Hello мир world");
        
        // Remove ASCII character
        App::remove_char_at(&mut text, 0);
        assert_eq!(text, "ello мир world");
        
        // Remove Unicode character
        let mut text = String::from("Hello мир world");
        App::remove_char_at(&mut text, 6); // Remove 'м'
        assert_eq!(text, "Hello ир world");
        
        // Remove beyond string length (should do nothing)
        let mut text = String::from("test");
        let original_len = text.len();
        App::remove_char_at(&mut text, 100);
        assert_eq!(text.len(), original_len);
    }

    #[test]
    fn test_toggle_metadata_mode() {
        let po_file = PoFile::default();
        let mut app = App::new(po_file);
        
        // Initially should not be in metadata mode
        assert!(!app.is_metadata_mode());
        assert_eq!(app.edit_field, EditField::Msgstr);
        
        // Toggle to metadata mode
        app.toggle_metadata_mode();
        assert!(app.is_metadata_mode());
        assert_eq!(app.edit_field, EditField::Metadata);
        
        // Toggle back
        app.toggle_metadata_mode();
        assert!(!app.is_metadata_mode());
        assert_eq!(app.edit_field, EditField::Msgstr);
        
        // Should not toggle when editing
        app.editing = true;
        app.toggle_metadata_mode();
        assert!(!app.is_metadata_mode());  // Should remain false
    }

    #[test]
    fn test_insert_char_at() {
        let mut text = String::from("Hello world");
        
        // Insert ASCII character
        App::insert_char_at(&mut text, 5, ' ');
        assert_eq!(text, "Hello  world");
        
        // Insert Unicode character
        let mut text = String::from("Hello world");
        App::insert_char_at(&mut text, 5, 'ё');
        assert_eq!(text, "Helloё world");
        
        // Insert at beginning
        let mut text = String::from("test");
        App::insert_char_at(&mut text, 0, 'X');
        assert_eq!(text, "Xtest");
        
        // Insert at end
        let mut text = String::from("test");
        App::insert_char_at(&mut text, 4, '!');
        assert_eq!(text, "test!");
    }

    #[test]
    fn test_page_navigation() {
        let mut po_file = PoFile::default();
        // Add test entries
        for i in 0..25 {
            let mut entry = PoEntry::new();
            entry.msgid = format!("test {}", i);
            po_file.entries.push(entry);
        }
        
        let mut app = App::new(po_file);
        
        // Test page down
        app.page_down();
        assert_eq!(app.current_entry, PAGE_SIZE);
        
        // Test page up
        app.page_up();
        assert_eq!(app.current_entry, 0);
        
        // Test page down at end
        app.current_entry = 20;
        app.page_down();
        assert_eq!(app.current_entry, 24); // Should go to last entry
    }

    #[test]
    fn test_edit_field_cycling() {
        let po_file = PoFile::default();
        let mut app = App::new(po_file);
        
        assert_eq!(app.edit_field, EditField::Msgstr);
        
        app.next_field();
        assert_eq!(app.edit_field, EditField::Comments);
        
        app.next_field();
        assert_eq!(app.edit_field, EditField::Msgid);
        
        app.next_field();
        assert_eq!(app.edit_field, EditField::Msgstr);
        
        app.previous_field();
        assert_eq!(app.edit_field, EditField::Msgid);
    }
    
    #[test]
    fn test_metadata_mode() {
        let po_file = PoFile::default();
        let mut app = App::new(po_file);
        
        assert!(!app.metadata_mode);
        
        app.toggle_metadata_mode();
        assert!(app.metadata_mode);
        assert_eq!(app.edit_field, EditField::Metadata);
    }

    #[test]
    fn test_toggle_fuzzy_functionality() {
        let mut po_file = PoFile::default();
        
        // Add a translated entry
        let mut entry = PoEntry::new();
        entry.msgid = "Hello".to_string();
        entry.set_msgstr("Привет".to_string());
        po_file.entries.push(entry);
        
        // Add a fuzzy entry
        let mut fuzzy_entry = PoEntry::new();
        fuzzy_entry.msgid = "World".to_string();
        fuzzy_entry.msgstr = "Мир".to_string();
        fuzzy_entry.flags.push("fuzzy".to_string());
        fuzzy_entry.update_status();
        po_file.entries.push(fuzzy_entry);
        
        let mut app = App::new(po_file);
        
        // Test toggle fuzzy on translated entry (index 0)
        assert!(!app.po_file.entries[0].is_fuzzy);
        app.toggle_current_entry_fuzzy();
        assert!(app.po_file.entries[0].is_fuzzy);
        
        // Toggle back
        app.toggle_current_entry_fuzzy();
        assert!(!app.po_file.entries[0].is_fuzzy);
        
        // Move to fuzzy entry (index 1)
        app.next_entry();
        assert!(app.po_file.entries[1].is_fuzzy);
        
        // Toggle fuzzy off (mark as done)
        app.toggle_current_entry_fuzzy();
        assert!(!app.po_file.entries[1].is_fuzzy);
        assert!(app.po_file.entries[1].is_translated);
    }

    #[test]
    fn test_mark_entry_done() {
        let mut po_file = PoFile::default();
        
        // Add a fuzzy entry
        let mut entry = PoEntry::new();
        entry.msgid = "Test".to_string();
        entry.msgstr = "Тест".to_string();
        entry.flags.push("fuzzy".to_string());
        entry.update_status();
        po_file.entries.push(entry);
        
        let mut app = App::new(po_file);
        
        assert!(app.po_file.entries[0].is_fuzzy);
        assert!(!app.po_file.entries[0].is_translated);
        
        // Mark as done
        app.mark_current_entry_done();
        
        assert!(!app.po_file.entries[0].is_fuzzy);
        assert!(app.po_file.entries[0].is_translated);
        assert!(!app.po_file.entries[0].flags.contains(&"fuzzy".to_string()));
    }

    #[test]
    fn test_fuzzy_toggle_edge_cases() {
        let mut po_file = PoFile::default();
        
        // Add an untranslated entry (empty msgstr)
        let mut entry = PoEntry::new();
        entry.msgid = "Empty".to_string();
        entry.msgstr = "".to_string();
        po_file.entries.push(entry);
        
        let mut app = App::new(po_file);
        
        // Should not toggle fuzzy on empty translation
        assert!(!app.po_file.entries[0].is_fuzzy);
        app.toggle_current_entry_fuzzy();
        assert!(!app.po_file.entries[0].is_fuzzy);
        
        // Should not mark as done if no translation
        app.mark_current_entry_done();
        assert!(!app.po_file.entries[0].is_translated);
    }
}