use crate::app::{
    AddItemField, App, ChangeKeyField, ImportExportField, ImportExportMode, ItemCounts, PasswordGenField, Screen,
    StatusType, UnlockField, ViewMode,
};
use chamber_vault::ItemKind;
use color_eyre::Result;
use ratatui::crossterm::event;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::Alignment;
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

// --- Palette ---
const fn c_bg() -> Color {
    Color::Rgb(18, 18, 23)
}
const fn c_bg_panel() -> Color {
    Color::Rgb(24, 26, 33)
}
const fn c_border() -> Color {
    Color::Rgb(60, 66, 80)
}
const fn c_accent() -> Color {
    Color::Rgb(80, 200, 255)
} // cyan-ish
const fn c_accent2() -> Color {
    Color::Rgb(148, 92, 255)
} // purple
const fn c_ok() -> Color {
    Color::Rgb(120, 220, 120)
}
const fn c_warn() -> Color {
    Color::Rgb(255, 210, 90)
}
const fn c_err() -> Color {
    Color::Rgb(255, 120, 120)
}
const fn c_text() -> Color {
    Color::Rgb(220, 224, 232)
}
const fn c_text_dim() -> Color {
    Color::Rgb(140, 145, 160)
}
const fn c_badge_pwd() -> Color {
    Color::Rgb(255, 140, 140)
}
const fn c_badge_env() -> Color {
    Color::Rgb(120, 220, 120)
}
const fn c_badge_note() -> Color {
    Color::Rgb(120, 180, 255)
}

const fn c_badge_api() -> Color {
    Color::Rgb(255, 165, 0) // Orange
}

const fn c_badge_ssh() -> Color {
    Color::Rgb(0, 255, 255) // Cyan
}

const fn c_badge_cert() -> Color {
    Color::Rgb(255, 20, 147) // Deep Pink
}

const fn c_badge_db() -> Color {
    Color::Rgb(50, 205, 50) // Lime Green
}

const fn c_badge_creditcard() -> Color {
    Color::Rgb(255, 215, 0) // Gold for credit cards
}

const fn c_badge_securenote() -> Color {
    Color::Rgb(138, 43, 226) // Purple for secure notes
}

const fn c_badge_identity() -> Color {
    Color::Rgb(0, 191, 255) // Deep sky blue for identity
}

const fn c_badge_server() -> Color {
    Color::Rgb(220, 20, 60) // Crimson for servers
}

const fn c_badge_wifi() -> Color {
    Color::Rgb(34, 139, 34) // Forest green for WiFi
}

const fn c_badge_license() -> Color {
    Color::Rgb(255, 140, 0) // Dark orange for licenses
}

const fn c_badge_bankaccount() -> Color {
    Color::Rgb(0, 100, 0) // Dark green for bank accounts
}

const fn c_badge_document() -> Color {
    Color::Rgb(105, 105, 105) // Dim gray for documents
}

const fn c_badge_recovery() -> Color {
    Color::Rgb(255, 20, 147) // Deep pink for recovery codes
}

const fn c_badge_oauth() -> Color {
    Color::Rgb(30, 144, 255) // Dodger blue for OAuth
}

fn truncate_text(text: &str, max_width: usize) -> String {
    if text.chars().count() <= max_width {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_width.saturating_sub(3)).collect();
        format!("{truncated}...")
    }
}

/// Runs the application in a terminal-based user interface with an event loop.
///
/// This function initializes a terminal using the Crossterm backend, clears its
/// contents, and enters an event loop. In this loop, it draws the application's
/// current state to the terminal and waits for user input events. When a key
/// event occurs, it processes the event and checks if the application should
/// exit. The loop exits when the `handle_key` function signals to terminate.
///
/// # Arguments
///
/// * `app` - A mutable reference to the application's state, which is used
///   and updated throughout the event loop.
///
/// # Returns
///
/// * `Result<()>` - Returns `Ok(())` if the application runs and exits successfully.
///   Returns an error if any issues are encountered during initialization, drawing,
///   or event handling.
///
/// # Errors
///
/// This function can return an error in the following situations:
/// * If the terminal backend cannot be initialized.
/// * If the terminal fails to clear its contents or render the UI.
/// * If there is an error reading input events or processing them.
///
/// # Notes
///
/// * The terminal clearing and rendering are achieved using the `Terminal` and
///   `CrosstermBackend` from the `tui` (Terminal User Interface) library.
/// * The event loop uses a polling mechanism to periodically check for input
///   using `event::poll`, blocking for 250 milliseconds before timing out.
/// * Pressing specific keys, as determined by the `handle_key` function, can
///   terminate the event loop.
pub fn run_app(app: &mut App) -> Result<()> {
    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    loop {
        terminal.draw(|f| draw(f, app))?;
        if event::poll(std::time::Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && handle_key(app, key)? {
                    break;
                }
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
#[allow(clippy::cognitive_complexity)]
fn handle_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    if app.search_mode {
        match key.code {
            KeyCode::Esc => {
                app.search_mode = false;
                return Ok(false);
            }
            KeyCode::Enter => {
                app.search_mode = false;
                app.update_filtered_items();
                return Ok(false);
            }
            KeyCode::Backspace => {
                app.search_query.pop();
                app.update_filtered_items();
                return Ok(false);
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
                app.update_filtered_items();
                return Ok(false);
            }
            _ => return Ok(false),
        }
    }

    // Handle global Ctrl combinations FIRST, before screen-specific logic
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('v' | 'V') => {
                // Handle paste based on current screen and focus
                match app.screen {
                    Screen::Unlock => {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            if let Ok(text) = clipboard.get_text() {
                                match app.unlock_focus {
                                    UnlockField::Master => app.master_input.push_str(&text),
                                    UnlockField::Confirm => {
                                        if app.master_mode_is_setup {
                                            app.master_confirm_input.push_str(&text);
                                        }
                                    }
                                }
                            }
                        }
                        return Ok(false); // Prevent further processing
                    }
                    Screen::AddItem => {
                        match app.add_focus {
                            AddItemField::Value => {
                                if let Err(e) = app.paste_to_add_value() {
                                    app.set_status(format!("Paste failed: {e}"), StatusType::Error);
                                }
                            }
                            AddItemField::Name => {
                                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                    if let Ok(text) = clipboard.get_text() {
                                        app.add_name.push_str(&text);
                                        app.set_status(
                                            format!("Pasted {} characters to name field", text.len()),
                                            StatusType::Success,
                                        );
                                    }
                                }
                            }
                            AddItemField::Kind => {}
                        }
                        return Ok(false); // Prevent further processing
                    }
                    Screen::EditItem => {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            if let Ok(text) = clipboard.get_text() {
                                app.edit_value.push_str(&text);
                                app.set_status(
                                    format!("Pasted {} characters to edit field", text.len()),
                                    StatusType::Success,
                                );
                            }
                        }
                        return Ok(false); // Prevent further processing
                    }
                    Screen::ChangeMaster => {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            if let Ok(text) = clipboard.get_text() {
                                match app.ck_focus {
                                    ChangeKeyField::Current => app.ck_current.push_str(&text),
                                    ChangeKeyField::New => app.ck_new.push_str(&text),
                                    ChangeKeyField::Confirm => app.ck_confirm.push_str(&text),
                                }
                            }
                        }
                        return Ok(false); // Prevent further processing
                    }
                    Screen::ImportExport => {
                        if matches!(app.ie_focus, ImportExportField::Path) {
                            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                if let Ok(text) = clipboard.get_text() {
                                    app.ie_path.push_str(&text);
                                    app.set_status(
                                        format!("Pasted {} characters to path field", text.len()),
                                        StatusType::Success,
                                    );
                                }
                            }
                        }
                        return Ok(false); // Prevent further processing
                    }
                    _ => {}
                }
                return Ok(false); // Always prevent further processing for Ctrl+V
            }
            KeyCode::Char('c' | 'C') => {
                // Handle copy operations
                if matches!(app.screen, Screen::Main) {
                    app.copy_selected()?;
                }
                return Ok(false); // Prevent further processing
            }
            KeyCode::Enter => {
                // Handle Ctrl+Enter for saving based on current screen and focus
                match app.screen {
                    Screen::AddItem if matches!(app.add_focus, AddItemField::Value) => {
                        // Save the item when Ctrl+Enter is pressed in Value field
                        return app.add_item().map(|()| false);
                    }
                    _ => {
                        // For other screens/fields, let the normal handling occur
                    }
                }
                return Ok(false);
            }

            _ => {
                // For other Ctrl combinations, don't process them as regular characters
                return Ok(false);
            }
        }
    }

    // Now handle screen-specific keys (after Ctrl combinations are processed)
    match app.screen {
        Screen::Unlock => match key.code {
            KeyCode::Esc => return Ok(true),
            KeyCode::Enter => {
                app.unlock()?;
            }
            KeyCode::Tab => {
                if app.master_mode_is_setup {
                    app.unlock_focus = match app.unlock_focus {
                        UnlockField::Master => UnlockField::Confirm,
                        UnlockField::Confirm => UnlockField::Master,
                    };
                }
            }
            KeyCode::Backspace => match app.unlock_focus {
                UnlockField::Master => {
                    app.master_input.pop();
                }
                UnlockField::Confirm => {
                    if app.master_mode_is_setup {
                        app.master_confirm_input.pop();
                    }
                }
            },
            KeyCode::Char(c) => {
                // Only process regular characters (Ctrl combinations handled above)
                match app.unlock_focus {
                    UnlockField::Master => app.master_input.push(c),
                    UnlockField::Confirm => {
                        if app.master_mode_is_setup {
                            app.master_confirm_input.push(c);
                        }
                    }
                }
            }
            _ => {}
        },

        Screen::Main => match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Char('a') => {
                app.screen = Screen::AddItem;
            }
            KeyCode::Char('c') => {
                // Only handle 'c' for copy if Ctrl is NOT pressed (Ctrl+C handled above)
                if !key.modifiers.contains(KeyModifiers::CONTROL) {
                    app.copy_selected()?;
                }
            }
            KeyCode::Char('v') => {
                // Only handle 'v' for view if Ctrl is NOT pressed (Ctrl+V handled above)
                app.view_selected();
            }
            KeyCode::Char('e') => {
                app.edit_selected();
            }
            KeyCode::Char('k') => {
                app.ck_current.clear();
                app.ck_new.clear();
                app.ck_confirm.clear();
                app.ck_focus = ChangeKeyField::Current;
                app.error = None;
                app.screen = Screen::ChangeMaster;
            }
            KeyCode::Char('g') => {
                app.open_password_generator();
            }
            KeyCode::Char('x') => {
                app.open_import_export(ImportExportMode::Export);
            }
            KeyCode::Char('i') => {
                app.open_import_export(ImportExportMode::Import);
            }
            KeyCode::Char('d') => {
                app.delete_selected()?;
            }
            KeyCode::Down => {
                if app.filtered_items.is_empty() {
                    return Ok(false);
                }

                if app.selected < app.filtered_items.len().saturating_sub(1) {
                    app.selected += 1;
                } else {
                    app.selected = 0;
                }

                let viewport_height = 10;
                if app.selected >= app.scroll_offset + viewport_height {
                    app.scroll_offset = app.selected.saturating_sub(viewport_height - 1);
                } else if app.selected == 0 {
                    app.scroll_offset = 0;
                }
            }
            KeyCode::Up => {
                if app.filtered_items.is_empty() {
                    return Ok(false);
                }

                if app.selected > 0 {
                    app.selected -= 1;
                } else {
                    app.selected = app.filtered_items.len().saturating_sub(1);
                }
                if app.selected < app.scroll_offset {
                    app.scroll_offset = app.selected;
                }
            }
            KeyCode::Char('r') => {
                app.refresh_items()?;
            }
            KeyCode::F(2) => {
                app.open_vault_selector();
            }
            KeyCode::Char('/' | 's') => {
                app.search_mode = true;
            }
            KeyCode::Esc => {
                if !app.search_query.is_empty() {
                    app.search_query.clear();
                    app.update_filtered_items();
                }
            }
            _ => {}
        },

        Screen::AddItem => match key.code {
            KeyCode::Esc => {
                app.screen = Screen::Main;
            }
            KeyCode::Tab => {
                app.add_value = app.add_value_textarea.lines().join("\n");
                app.add_focus = match app.add_focus {
                    AddItemField::Name => AddItemField::Kind,
                    AddItemField::Kind => AddItemField::Value,
                    AddItemField::Value => AddItemField::Name,
                };
            }
            KeyCode::Left | KeyCode::Right if matches!(app.add_focus, AddItemField::Kind) => {
                let total_kinds = 17;
                if key.code == KeyCode::Right {
                    app.add_kind_idx = (app.add_kind_idx + 1) % total_kinds;
                } else {
                    app.add_kind_idx = if app.add_kind_idx == 0 {
                        total_kinds - 1
                    } else {
                        app.add_kind_idx - 1
                    };
                }
            }
            KeyCode::Char(c) => {
                // All regular character input (Ctrl combinations handled above)
                match app.add_focus {
                    AddItemField::Name => app.add_name.push(c),
                    AddItemField::Value => {
                        app.add_value_textarea.input(key);
                    }
                    AddItemField::Kind => {}
                }
            }
            _ => {
                match app.add_focus {
                    AddItemField::Name => {
                        // Handle name field input
                        match key.code {
                            KeyCode::Enter => {
                                // Save item when Enter is pressed in name field
                                app.add_value = app.add_value_textarea.lines().join("\n");
                                return app.add_item().map(|()| false);
                            }
                            KeyCode::Backspace => {
                                app.add_name.pop();
                            }
                            KeyCode::Char(c) => {
                                app.add_name.push(c);
                            }
                            _ => {}
                        }
                    }
                    AddItemField::Kind => {
                        // Kind field doesn't need text input, just navigation
                        if matches!(key.code, KeyCode::Enter) {
                            app.add_value = app.add_value_textarea.lines().join("\n");
                            return app.add_item().map(|()| false);
                        }
                    }
                    AddItemField::Value => {
                        // Let textarea handle ALL input for Value field
                        match key.code {
                            // Only intercept Ctrl+Enter for saving
                            KeyCode::Enter if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                app.add_value = app.add_value_textarea.lines().join("\n");
                                return app.add_item().map(|()| false);
                            }
                            // Let textarea handle everything else, including regular Enter
                            _ => {
                                app.add_value_textarea.input(key);
                            }
                        }
                    }
                }
            }
        },

        Screen::ViewItem => match key.code {
            KeyCode::Esc => {
                app.screen = Screen::Main;
            }
            KeyCode::Char('t') | KeyCode::Enter => {
                app.toggle_value_visibility();
            }
            KeyCode::Char('c') => {
                if let Some(item) = &app.view_item {
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        let _ = clipboard.set_text(&item.value);
                        app.set_status(format!("Copied '{}' to clipboard", item.name), StatusType::Success);
                    }
                }
            }
            _ => {}
        },

        Screen::EditItem => match key.code {
            KeyCode::Esc => {
                app.screen = Screen::Main;
            }
            KeyCode::Enter => {
                app.save_edit()?;
            }
            KeyCode::Backspace => {
                app.edit_value.pop();
            }
            KeyCode::Char(c) => {
                // All regular character input (Ctrl combinations handled above)
                app.edit_value.push(c);
            }
            _ => {}
        },

        Screen::ChangeMaster => match key.code {
            KeyCode::Esc => {
                app.screen = Screen::Main;
            }
            KeyCode::Enter => {
                app.change_master()?;
            }
            KeyCode::Tab => {
                app.ck_focus = match app.ck_focus {
                    ChangeKeyField::Current => ChangeKeyField::New,
                    ChangeKeyField::New => ChangeKeyField::Confirm,
                    ChangeKeyField::Confirm => ChangeKeyField::Current,
                };
            }
            KeyCode::Backspace => match app.ck_focus {
                ChangeKeyField::Current => {
                    app.ck_current.pop();
                }
                ChangeKeyField::New => {
                    app.ck_new.pop();
                }
                ChangeKeyField::Confirm => {
                    app.ck_confirm.pop();
                }
            },
            KeyCode::Char(c) => {
                // All regular character input (Ctrl combinations handled above)
                match app.ck_focus {
                    ChangeKeyField::Current => app.ck_current.push(c),
                    ChangeKeyField::New => app.ck_new.push(c),
                    ChangeKeyField::Confirm => app.ck_confirm.push(c),
                }
            }
            _ => {}
        },

        Screen::GeneratePassword => match key.code {
            KeyCode::Esc => {
                app.screen = Screen::Main;
            }
            KeyCode::Tab => {
                app.gen_focus = match app.gen_focus {
                    PasswordGenField::Length => PasswordGenField::Options,
                    PasswordGenField::Options => PasswordGenField::Generate,
                    PasswordGenField::Generate => PasswordGenField::Length,
                };
            }
            KeyCode::Char('g') | KeyCode::Enter => {
                app.generate_password();
            }
            KeyCode::Char('c') => {
                if let Err(e) = app.copy_generated_password() {
                    app.set_status(format!("Copy failed: {e}"), StatusType::Error);
                }
            }
            KeyCode::Char('u') => {
                app.use_generated_password();
            }
            KeyCode::Char(c) if matches!(app.gen_focus, PasswordGenField::Length) => {
                if c.is_ascii_digit() {
                    app.gen_length_str.push(c);
                }
            }
            KeyCode::Backspace if matches!(app.gen_focus, PasswordGenField::Length) => {
                app.gen_length_str.pop();
            }
            KeyCode::Char('1') if matches!(app.gen_focus, PasswordGenField::Options) => {
                app.gen_config.include_uppercase = !app.gen_config.include_uppercase;
            }
            KeyCode::Char('2') if matches!(app.gen_focus, PasswordGenField::Options) => {
                app.gen_config.include_lowercase = !app.gen_config.include_lowercase;
            }
            KeyCode::Char('3') if matches!(app.gen_focus, PasswordGenField::Options) => {
                app.gen_config.include_digits = !app.gen_config.include_digits;
            }
            KeyCode::Char('4') if matches!(app.gen_focus, PasswordGenField::Options) => {
                app.gen_config.include_symbols = !app.gen_config.include_symbols;
            }
            KeyCode::Char('5') if matches!(app.gen_focus, PasswordGenField::Options) => {
                app.gen_config.exclude_ambiguous = !app.gen_config.exclude_ambiguous;
            }
            _ => {}
        },

        Screen::ImportExport => match key.code {
            KeyCode::Esc => {
                app.screen = Screen::Main;
            }
            KeyCode::Tab => {
                app.ie_focus = match app.ie_focus {
                    ImportExportField::Path => ImportExportField::Format,
                    ImportExportField::Format => ImportExportField::Action,
                    ImportExportField::Action => ImportExportField::Path,
                };
            }
            KeyCode::Enter => {
                if matches!(app.ie_focus, ImportExportField::Action) {
                    app.execute_import_export()?;
                }
            }
            KeyCode::Left | KeyCode::Right if matches!(app.ie_focus, ImportExportField::Format) => {
                if key.code == KeyCode::Right {
                    app.ie_format_idx = (app.ie_format_idx + 1) % app.ie_formats.len();
                } else {
                    app.ie_format_idx = if app.ie_format_idx == 0 {
                        app.ie_formats.len() - 1
                    } else {
                        app.ie_format_idx - 1
                    };
                }
            }
            KeyCode::Backspace if matches!(app.ie_focus, ImportExportField::Path) => {
                app.ie_path.pop();
            }
            KeyCode::Char(c) if matches!(app.ie_focus, ImportExportField::Path) => {
                // All regular character input (Ctrl combinations handled above)
                app.ie_path.push(c);
            }
            _ => {}
        },
        Screen::VaultSelector => {
            if let Some(action) = app.vault_selector.handle_input(key) {
                app.handle_vault_action(action)?;
            }
            return Ok(false);
        }
    }

    Ok(false)
}

fn draw(f: &mut Frame, app: &mut App) {
    let size = f.area();
    let bg_block = Block::default().style(Style::default().bg(c_bg()));
    f.render_widget(bg_block, size);

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Min(0),    // Body
            Constraint::Length(1), // Status bar
        ])
        .split(size);

    draw_header(f, root[0]);
    draw_status_bar(f, app, root[2]);

    match app.screen {
        Screen::Unlock => draw_unlock(f, app, root[1]),
        Screen::Main => draw_main(f, app, root[1]),
        Screen::AddItem => {
            draw_main(f, app, root[1]);
            draw_add_item(f, app);
        }
        Screen::ViewItem => {
            draw_main(f, app, root[1]);
            draw_view_item(f, app);
        }
        Screen::EditItem => {
            draw_main(f, app, root[1]);
            draw_edit_item(f, app);
        }
        Screen::ChangeMaster => {
            draw_main(f, app, root[1]);
            draw_change_master(f, app);
        }
        Screen::GeneratePassword => {
            draw_main(f, app, root[1]);
            draw_generate_password(f, app);
        }
        Screen::ImportExport => {
            draw_main(f, app, root[1]);
            draw_import_export(f, app);
        }
        Screen::VaultSelector => draw_vault_selector(f, app, root[1]),
    }
}

fn draw_vault_selector(f: &mut Frame, app: &mut App, area: Rect) {
    app.vault_selector.render(f, area);
}

fn draw_header(f: &mut Frame, area: Rect) {
    let title = Line::from(vec![
        Span::styled(
            "  ◈ chamber ",
            Style::default().fg(c_accent()).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled("secure vault", Style::default().fg(c_text_dim())),
    ]);

    let bar = Block::default()
        .borders(Borders::BOTTOM)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(c_border()))
        .style(Style::default().bg(c_bg_panel()));
    f.render_widget(bar, area);

    let inner = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    let title_para = Paragraph::new(title).style(Style::default().fg(c_text()));
    f.render_widget(title_para, inner[0]);

    let version_info = Paragraph::new(Line::from(vec![
        Span::styled("v", Style::default().fg(c_text_dim())),
        Span::styled(env!("CARGO_PKG_VERSION"), Style::default().fg(c_accent())),
        Span::styled(" © 2025", Style::default().fg(c_text_dim())),
    ]))
    .style(Style::default().fg(c_text()))
    .alignment(Alignment::Right);
    f.render_widget(version_info, inner[1]);
}

fn draw_unlock(f: &mut Frame, app: &App, body: Rect) {
    let area = centered_rect(60, 40, body);
    let title = if app.master_mode_is_setup {
        " Create Master Key "
    } else {
        " Unlock "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Span::styled(
            title,
            Style::default().fg(c_accent()).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(c_bg_panel()).fg(c_text()))
        .border_style(Style::default().fg(c_border()));

    let highlight = Style::default().fg(c_accent()).add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(c_text_dim());

    let mk_label = if app.master_mode_is_setup {
        "New master key"
    } else {
        "Master key"
    };
    let mk_value = field_box(
        &mask(&app.master_input),
        matches!(app.unlock_focus, UnlockField::Master),
    );

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(mk_label, Style::default().fg(c_text_dim()))),
        Line::from(mk_value),
    ];

    if app.master_mode_is_setup {
        let cf_value = field_box(
            &mask(&app.master_confirm_input),
            matches!(app.unlock_focus, UnlockField::Confirm),
        );
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            "Confirm master key",
            Style::default().fg(c_text_dim()),
        )));
        lines.push(Line::from(cf_value));
        lines.push(Line::default());
        lines.push(Line::from(vec![
            Span::styled("[Tab]", highlight),
            Span::styled(" switch  ", dim),
            Span::styled("[Enter]", highlight),
            Span::styled(" initialize & unlock  ", dim),
            Span::styled("[Esc]", highlight),
            Span::styled(" quit", dim),
        ]));
    } else {
        lines.push(Line::default());
        lines.push(Line::from(vec![
            Span::styled("[Enter]", highlight),
            Span::styled(" unlock  ", dim),
            Span::styled("[Esc]", highlight),
            Span::styled(" quit", dim),
        ]));
    }

    if let Some(err) = &app.error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            err,
            Style::default().fg(c_err()).add_modifier(Modifier::BOLD),
        )));
    }

    let p = Paragraph::new(Text::from(lines)).block(block).wrap(Wrap { trim: true });
    f.render_widget(Clear, area);
    f.render_widget(p, area);
}

#[allow(clippy::too_many_lines)]
fn draw_main(f: &mut Frame, app: &App, body: Rect) {
    // Create three-column layout: Items | Categories | Help
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60), // Items section
            Constraint::Percentage(20), // Categories section
            Constraint::Percentage(20), // Help section
        ])
        .split(centered_rect(96, 90, body));

    let items_area = main_layout[0];
    let categories_area = main_layout[1];
    let help_area = main_layout[2];

    // === ITEMS SECTION ===
    draw_items_section(f, app, items_area);

    // === CATEGORIES SECTION ===
    draw_categories_section(f, app, categories_area);

    // === HELP SECTION ===
    draw_help_section(f, help_area);
}

#[allow(clippy::too_many_lines)]
fn draw_items_section(f: &mut Frame, app: &App, area: Rect) {
    // Create layout for search bar and items list
    let (search_area, items_area) = if app.search_mode || !app.search_query.is_empty() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(area);
        (Some(chunks[0]), chunks[1])
    } else {
        (None, area) // Use full area if no search
    };

    // Draw search bar if needed
    if let Some(search_rect) = search_area {
        let search_text = if app.search_mode {
            format!("Search: {}_", app.search_query) // Show cursor when in search mode
        } else {
            format!("Search: {} (Press '/' or 's' to edit, Esc to clear)", app.search_query)
        };

        let search_style = if app.search_mode {
            Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(c_text_dim())
        };

        let search_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(if app.search_mode { c_accent() } else { c_border() }))
            .style(Style::default().bg(c_bg_panel()));

        let search_paragraph = Paragraph::new(search_text).style(search_style).block(search_block);

        f.render_widget(search_paragraph, search_rect);
    }

    // Calculate viewport dimensions for items area
    let viewport_height = items_area.height.saturating_sub(2) as usize; // account for borders
    let content_length = app.filtered_items.len();

    // Calculate what items to show based on scroll offset
    let visible_start = app.scroll_offset.min(content_length.saturating_sub(1));
    let visible_end = (visible_start + viewport_height).min(content_length);

    // Create list items only for visible range
    let mut list_items: Vec<ListItem> = Vec::new();
    let mut current_category = None;

    for (index, item) in app.filtered_items.iter().enumerate() {
        // Skip items outside visible range
        if index < visible_start || index >= visible_end {
            continue;
        }

        // Add category header if needed
        let item_category = item.kind.as_str();
        if current_category != Some(item_category) {
            current_category = Some(item_category);

            // Add spacing before new category (except for first visible item)
            if !list_items.is_empty() {
                list_items.push(ListItem::new(Line::from("")));
            }

            // Category header
            let (category_name, category_icon, category_color) = match item.kind {
                ItemKind::Password => (" PASSWORDS", "🔐", c_badge_pwd()),
                ItemKind::EnvVar => (" ENVIRONMENT VARIABLES", "🌍", c_badge_env()),
                ItemKind::Note => (" NOTES", "📝", c_badge_note()),
                ItemKind::ApiKey => (" API KEYS", "🔑", c_badge_api()),
                ItemKind::SshKey => (" SSH KEYS", "🔒", c_badge_ssh()),
                ItemKind::Certificate => (" CERTIFICATES", "📜", c_badge_cert()),
                ItemKind::Database => (" DATABASES", "🗄️", c_badge_db()),
                ItemKind::CreditCard => (" CREDIT CARDS", "💳", c_badge_creditcard()),
                ItemKind::SecureNote => (" SECURE NOTES", "🔒", c_badge_securenote()),
                ItemKind::Identity => (" IDENTITIES", "🆔", c_badge_identity()),
                ItemKind::Server => (" SERVERS", "🖥️", c_badge_server()),
                ItemKind::WifiPassword => (" WIFI", "📶", c_badge_wifi()),
                ItemKind::License => (" LICENSES", "📄", c_badge_license()),
                ItemKind::BankAccount => (" BANK ACCOUNTS", "🏦", c_badge_bankaccount()),
                ItemKind::Document => (" DOCUMENTS", "📋", c_badge_document()),
                ItemKind::Recovery => (" RECOVERY", "🔄", c_badge_recovery()),
                ItemKind::OAuth => (" OAUTH TOKENS", "🎫", c_badge_oauth()),
            };

            let header_line = Line::from(vec![
                Span::styled(
                    format!("  {category_icon} "),
                    Style::default().fg(category_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    category_name,
                    Style::default().fg(category_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" ".repeat(50), Style::default().fg(c_border())),
            ]);

            list_items.push(ListItem::new(header_line).style(Style::default().bg(Color::Rgb(30, 32, 38))));
        }

        // Regular item
        let (badge, badge_color) = match item.kind {
            ItemKind::Password => ("🔐", c_badge_pwd()),
            ItemKind::EnvVar => ("🌍", c_badge_env()),
            ItemKind::Note => ("📝", c_badge_note()),
            ItemKind::ApiKey => ("🔑", c_badge_api()),
            ItemKind::SshKey => ("🔒", c_badge_ssh()),
            ItemKind::Certificate => ("📜", c_badge_cert()),
            ItemKind::Database => ("🗄️", c_badge_db()),
            ItemKind::CreditCard => ("💳", c_badge_creditcard()),
            ItemKind::SecureNote => ("🔒", c_badge_securenote()),
            ItemKind::Identity => ("🆔", c_badge_identity()),
            ItemKind::Server => ("🖥️", c_badge_server()),
            ItemKind::WifiPassword => ("📶", c_badge_wifi()),
            ItemKind::License => ("📄", c_badge_license()),
            ItemKind::BankAccount => ("🏦", c_badge_bankaccount()),
            ItemKind::Document => ("📋", c_badge_document()),
            ItemKind::Recovery => ("🔄", c_badge_recovery()),
            ItemKind::OAuth => ("🎫", c_badge_oauth()),
        };

        let created_date = match time::format_description::parse("[year]-[month]-[day]") {
            Ok(format) => item
                .created_at
                .format(&format)
                .unwrap_or_else(|_| "unknown".to_string()),
            Err(_) => "unknown".to_string(),
        };

        let content_width = items_area.width.saturating_sub(8) as usize;
        let max_name_width = content_width.saturating_sub(20); // Conservative estimate for all fixed elements

        let truncated_name = if max_name_width < 1 {
            "…".to_string() // Show ellipsis if no room
        } else {
            truncate_text(&item.name, max_name_width.max(1))
        };

        // Highlight search matches in the item name
        let item_name_spans = if !app.search_query.is_empty() && !app.search_mode {
            highlight_search_matches(&truncated_name, &app.search_query, c_text(), c_accent())
        } else {
            vec![Span::styled(
                truncated_name,
                Style::default().fg(c_text()).add_modifier(Modifier::BOLD),
            )]
        };

        let mut item_line_spans = vec![
            Span::raw("    "), // Indentation for items under category
            Span::styled(format!("{badge} "), Style::default().fg(badge_color)),
        ];
        item_line_spans.extend(item_name_spans);
        item_line_spans.push(Span::styled(
            format!(" ({created_date})"),
            Style::default().fg(c_text_dim()),
        ));

        let item_line = Line::from(item_line_spans);

        // Highlight selected item
        let item_style = if app.selected == index {
            Style::default()
                .bg(Color::Rgb(40, 46, 60))
                .fg(c_accent())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        list_items.push(ListItem::new(item_line).style(item_style));
    }

    // Show empty state or search results info
    if list_items.is_empty() && app.filtered_items.is_empty() {
        let empty_message = if app.search_query.is_empty() {
            match app.view_mode {
                ViewMode::All => "No items in vault".to_string(),
                ViewMode::Passwords => "No passwords stored".to_string(),
                ViewMode::Environment => "No environment variables stored".to_string(),
                ViewMode::Notes => "No notes stored".to_string(),
            }
        } else {
            format!("No items match search: '{}'", app.search_query)
        };

        list_items.push(ListItem::new(Line::from(vec![Span::styled(
            format!("     {empty_message}"),
            Style::default().fg(c_text_dim()),
        )])));
    }

    let items_title = if app.search_query.is_empty() {
        format!(
            " {} ({}/{}) ",
            app.view_mode.as_str(),
            app.filtered_items.len(),
            app.items.len()
        )
    } else {
        format!(
            " {} ({}/{}) - Search: '{}' ",
            app.view_mode.as_str(),
            app.filtered_items.len(),
            app.items.len(),
            app.search_query
        )
    };

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(c_border()))
        .style(Style::default().bg(c_bg_panel()))
        .title(Span::styled(
            &items_title,
            Style::default().fg(c_accent()).add_modifier(Modifier::BOLD),
        ));

    let list = List::new(list_items).block(list_block.clone());
    f.render_widget(list, items_area);

    // Create scrollbar
    let mut scrollbar_state =
        ratatui::widgets::ScrollbarState::new(content_length.max(1).saturating_sub(1)).position(app.scroll_offset);

    let scrollbar = ratatui::widgets::Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"))
        .thumb_style(Style::default().fg(c_accent()).add_modifier(Modifier::BOLD))
        .track_style(Style::default().fg(c_text_dim()));

    let inner_area = list_block.inner(items_area);
    let scrollbar_area = Rect {
        x: inner_area.x + inner_area.width.saturating_sub(1),
        y: inner_area.y,
        width: 1,
        height: inner_area.height,
    };
    f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
}

// Helper function to highlight search matches in text
fn highlight_search_matches(
    text: &str,
    query: &str,
    normal_color: Color,
    highlight_color: Color,
) -> Vec<Span<'static>> {
    if query.is_empty() {
        return vec![Span::styled(
            text.to_string(),
            Style::default().fg(normal_color).add_modifier(Modifier::BOLD),
        )];
    }

    let mut spans = Vec::new();
    let text_lower = text.to_lowercase();
    let query_lower = query.to_lowercase();
    let mut last_end = 0;

    // Find all matches
    for match_start in text_lower.match_indices(&query_lower).map(|(i, _)| i) {
        // Add text before match
        if match_start > last_end {
            spans.push(Span::styled(
                text[last_end..match_start].to_string(),
                Style::default().fg(normal_color).add_modifier(Modifier::BOLD),
            ));
        }

        // Add highlighted match
        let match_end = match_start + query.len();
        spans.push(Span::styled(
            text[match_start..match_end].to_string(),
            Style::default()
                .fg(highlight_color)
                .bg(Color::Rgb(60, 60, 0))
                .add_modifier(Modifier::BOLD),
        ));

        last_end = match_end;
    }

    // Add remaining text
    if last_end < text.len() {
        spans.push(Span::styled(
            text[last_end..].to_string(),
            Style::default().fg(normal_color).add_modifier(Modifier::BOLD),
        ));
    }

    spans
}

#[allow(clippy::too_many_lines)]
fn draw_categories_section(f: &mut Frame, app: &App, area: Rect) {
    let ItemCounts {
        total: _,
        passwords,
        env_vars,
        notes,
        api_keys,
        ssh_keys,
        certificates,
        databases,
        credit_cards,
        secure_notes,
        identities,
        servers,
        wifi_passwords,
        licenses,
        bank_accounts,
        documents,
        recovery_codes,
        oauth_tokens,
    } = app.get_item_counts();

    let categories_content = vec![
        Line::from(vec![
            Span::styled("🔐 ", Style::default().fg(c_badge_pwd())),
            Span::styled(format!("Passwords ({passwords})"), Style::default().fg(c_text())),
        ]),
        Line::from(vec![
            Span::styled("🌍 ", Style::default().fg(c_badge_env())),
            Span::styled(format!("Environment ({env_vars})"), Style::default().fg(c_text())),
        ]),
        Line::from(vec![
            Span::styled("📝 ", Style::default().fg(c_badge_note())),
            Span::styled(format!("Notes ({notes})"), Style::default().fg(c_text())),
        ]),
        Line::from(vec![
            Span::styled("🔑 ", Style::default().fg(c_badge_api())),
            Span::styled(format!("API Keys ({api_keys})"), Style::default().fg(c_text())),
        ]),
        Line::from(vec![
            Span::styled("🔒 ", Style::default().fg(c_badge_ssh())),
            Span::styled(format!("SSH Keys ({ssh_keys})"), Style::default().fg(c_text())),
        ]),
        Line::from(vec![
            Span::styled("📜 ", Style::default().fg(c_badge_cert())),
            Span::styled(format!("Certificates ({certificates})"), Style::default().fg(c_text())),
        ]),
        Line::from(vec![
            Span::styled("🗄️ ", Style::default().fg(c_badge_db())),
            Span::styled(format!("Databases ({databases})"), Style::default().fg(c_text())),
        ]),
        Line::from(vec![
            Span::styled("💳 ", Style::default().fg(c_badge_creditcard())),
            Span::styled(format!("Credit Cards ({credit_cards})"), Style::default().fg(c_text())),
        ]),
        Line::from(vec![
            Span::styled("🔒 ", Style::default().fg(c_badge_securenote())),
            Span::styled(format!("Secure Notes ({secure_notes})"), Style::default().fg(c_text())),
        ]),
        Line::from(vec![
            Span::styled("🆔 ", Style::default().fg(c_badge_identity())),
            Span::styled(format!("Identities ({identities})"), Style::default().fg(c_text())),
        ]),
        Line::from(vec![
            Span::styled("🖥️ ", Style::default().fg(c_badge_server())),
            Span::styled(format!("Servers ({servers})"), Style::default().fg(c_text())),
        ]),
        Line::from(vec![
            Span::styled("📶 ", Style::default().fg(c_badge_wifi())),
            Span::styled(format!("WiFi ({wifi_passwords})"), Style::default().fg(c_text())),
        ]),
        Line::from(vec![
            Span::styled("📄 ", Style::default().fg(c_badge_license())),
            Span::styled(format!("Licenses ({licenses})"), Style::default().fg(c_text())),
        ]),
        Line::from(vec![
            Span::styled("🏦 ", Style::default().fg(c_badge_bankaccount())),
            Span::styled(
                format!("Bank Accounts ({bank_accounts})"),
                Style::default().fg(c_text()),
            ),
        ]),
        Line::from(vec![
            Span::styled("📋 ", Style::default().fg(c_badge_document())),
            Span::styled(format!("Documents ({documents})"), Style::default().fg(c_text())),
        ]),
        Line::from(vec![
            Span::styled("🔄 ", Style::default().fg(c_badge_recovery())),
            Span::styled(format!("Recovery ({recovery_codes})"), Style::default().fg(c_text())),
        ]),
        Line::from(vec![
            Span::styled("🎫 ", Style::default().fg(c_badge_oauth())),
            Span::styled(format!("OAuth ({oauth_tokens})"), Style::default().fg(c_text())),
        ]),
    ];

    let categories_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(c_border()))
        .style(Style::default().bg(c_bg_panel()))
        .title(Span::styled(
            " Categories ",
            Style::default().fg(c_accent()).add_modifier(Modifier::BOLD),
        ));

    let categories_paragraph = Paragraph::new(categories_content)
        .block(categories_block)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(c_text()));

    f.render_widget(categories_paragraph, area);
}

fn draw_help_section(f: &mut Frame, area: Rect) {
    let help_content = vec![
        Line::from(vec![
            Span::styled("a ", Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)),
            Span::raw("Add item"),
        ]),
        Line::from(vec![
            Span::styled("e ", Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)),
            Span::raw("Edit item"),
        ]),
        Line::from(vec![
            Span::styled("c ", Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)),
            Span::raw("Copy value"),
        ]),
        Line::from(vec![
            Span::styled("s or / ", Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)),
            Span::raw("Start search"),
        ]),
        Line::from(vec![
            Span::styled("g ", Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)),
            Span::raw("Generate password"),
        ]),
        Line::from(vec![
            Span::styled("x ", Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)),
            Span::raw("Export items"),
        ]),
        Line::from(vec![
            Span::styled("i ", Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)),
            Span::raw("Import items"),
        ]),
        Line::from(vec![
            Span::styled("d ", Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)),
            Span::raw("Delete selected"),
        ]),
        Line::from(vec![
            Span::styled("v ", Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)),
            Span::raw("View item"),
        ]),
        Line::from(vec![
            Span::styled("k ", Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)),
            Span::raw("Change master key"),
        ]),
        Line::from(vec![
            Span::styled("Ctrl+v ", Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)),
            Span::raw("Paste clipboard"),
        ]),
        Line::from(vec![
            Span::styled("F2 ", Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)),
            Span::raw("Vault registry"),
        ]),
        Line::from(vec![
            Span::styled("q ", Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)),
            Span::raw("Quit"),
        ]),
    ];

    let help_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(c_border()))
        .style(Style::default().bg(c_bg_panel()))
        .title(Span::styled(
            " Help ",
            Style::default().fg(c_accent2()).add_modifier(Modifier::BOLD),
        ));

    let help_paragraph = Paragraph::new(help_content)
        .block(help_block)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(c_text()));

    f.render_widget(help_paragraph, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    // Create the status bar background
    let status_block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(c_border()))
        .style(Style::default().bg(c_bg_panel()));

    f.render_widget(status_block, area);

    // Split the area into left (status message) and right (key hints)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    // Left side: Status message or default context
    let (message, message_style) = get_status_message_and_style(app);
    let status_paragraph = Paragraph::new(Line::from(vec![
        Span::raw(" "), // Small padding
        Span::styled(message, message_style),
    ]))
    .style(Style::default().bg(c_bg_panel()))
    .wrap(Wrap { trim: true });

    f.render_widget(status_paragraph, chunks[0]);

    // Right side: Key hints
    let key_hints = get_key_hints_for_screen(app);
    let hints_paragraph = Paragraph::new(Line::from(key_hints))
        .style(Style::default().bg(c_bg_panel()).fg(c_text_dim()))
        .alignment(Alignment::Right)
        .wrap(Wrap { trim: true });

    f.render_widget(hints_paragraph, chunks[1]);
}

fn get_status_message_and_style(app: &App) -> (String, Style) {
    if let Some(message) = &app.status_message {
        let style = match app.status_type {
            StatusType::Success => Style::default().fg(c_ok()).add_modifier(Modifier::BOLD),
            StatusType::Warning => Style::default().fg(c_warn()).add_modifier(Modifier::BOLD),
            StatusType::Error => Style::default().fg(c_err()).add_modifier(Modifier::BOLD),
            StatusType::Info => Style::default().fg(c_accent()),
        };
        (message.clone(), style)
    } else {
        // Default context message based on the current screen
        let context_message = match app.screen {
            Screen::Unlock => "Enter your master key to unlock the vault".to_string(),
            Screen::Main => {
                format!(" {} items in vault", app.items.len())
            }
            Screen::AddItem => "Fill in the item details and press Enter to save".to_string(),
            Screen::ViewItem => "Viewing item details".to_string(),
            Screen::EditItem => "Edit the item value and press Enter to save".to_string(),
            Screen::ChangeMaster => "Change your master key".to_string(),
            Screen::GeneratePassword => "Configure and generate a new password".to_string(),
            Screen::ImportExport => match app.ie_mode {
                crate::app::ImportExportMode::Export => "Export items to file".to_string(),
                crate::app::ImportExportMode::Import => "Import items from file".to_string(),
            },
            Screen::VaultSelector => "Select a vault to open".to_string(),
        };
        (context_message, Style::default().fg(c_text_dim()))
    }
}

fn get_key_hints_for_screen(app: &App) -> Vec<Span<'static>> {
    let mut spans = Vec::new();

    let add_hint = |spans: &mut Vec<Span<'static>>, key: &'static str, action: &'static str, emphasized: bool| {
        if !spans.is_empty() {
            spans.push(Span::raw(" "));
        }
        let key_style = if emphasized {
            Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(c_text()).add_modifier(Modifier::BOLD)
        };
        spans.push(Span::styled(format!("[{key}]"), key_style));
        spans.push(Span::styled(format!(" {action}"), Style::default().fg(c_text_dim())));
    };

    match app.screen {
        Screen::Unlock => {
            add_hint(&mut spans, "Tab", "Switch", false);
            add_hint(&mut spans, "Enter", "Unlock", true);
        }
        Screen::Main => {
            if app.search_mode {
                // When in search mode, show search-specific hints
                add_hint(&mut spans, "Type", "Search", true);
                add_hint(&mut spans, "Enter", "Confirm", false);
                add_hint(&mut spans, "Esc", "Exit Search", false);
            } else {
                // Normal main screen navigation
                add_hint(&mut spans, "↑↓", "Navigate", false);

                if !app.filtered_items.is_empty() {
                    add_hint(&mut spans, "Enter", "View", true);
                    add_hint(&mut spans, "e", "Edit", false);
                    add_hint(&mut spans, "c", "Copy", false);
                    add_hint(&mut spans, "Del", "Delete", false);
                }

                add_hint(&mut spans, "a", "Add", false);
                add_hint(&mut spans, "/", "Search", false);

                // Show different Esc behavior based on search state
                if app.search_query.is_empty() {
                    add_hint(&mut spans, "q", "Quit", false);
                } else {
                    add_hint(&mut spans, "Esc", "Clear Search", false);
                }

                add_hint(&mut spans, "g", "Generate", false);
                add_hint(&mut spans, "i", "Import", false);
                add_hint(&mut spans, "o", "Export", false);
                add_hint(&mut spans, "v", "Vaults", false);
            }
        }
        Screen::AddItem => {
            add_hint(&mut spans, "Tab", "Next Field", false);
            add_hint(&mut spans, "Ctrl+V", "Paste", false);
            add_hint(&mut spans, "Enter", "Save", true);
            add_hint(&mut spans, "Esc", "Cancel", false);
        }
        Screen::ViewItem => {
            add_hint(&mut spans, "v", "Toggle Value", true);
            add_hint(&mut spans, "e", "Edit", false);
            add_hint(&mut spans, "c", "Copy", false);
            add_hint(&mut spans, "Esc", "Back", false);
        }
        Screen::EditItem => {
            add_hint(&mut spans, "Enter", "Save", true);
            add_hint(&mut spans, "Esc", "Cancel", false);
        }
        Screen::ChangeMaster => {
            add_hint(&mut spans, "Tab", "Next Field", false);
            add_hint(&mut spans, "Enter", "Change", true);
            add_hint(&mut spans, "Esc", "Cancel", false);
        }
        Screen::GeneratePassword => {
            add_hint(&mut spans, "Space", "Generate", true);
            add_hint(&mut spans, "c", "Copy", false);
            add_hint(&mut spans, "u", "Use", false);
            add_hint(&mut spans, "Esc", "Back", false);
        }
        Screen::ImportExport => {
            add_hint(&mut spans, "Tab", "Next Field", false);
            add_hint(&mut spans, "Enter", "Execute", true);
            add_hint(&mut spans, "Esc", "Cancel", false);
        }
        Screen::VaultSelector => {
            add_hint(&mut spans, "↑↓", "Navigate", false);
            add_hint(&mut spans, "Tab", "Next Field", false);
            add_hint(&mut spans, "Enter", "Select", true);
            add_hint(&mut spans, "Esc", "Close", false);
        }
    }

    // Add a trailing space for padding
    spans.push(Span::raw(" "));
    spans
}

#[allow(clippy::too_many_lines)]
fn draw_add_item(f: &mut Frame, app: &App) {
    // Create a centered modal area
    let modal_area = centered_rect(70, 80, f.area());

    // Clear the area to avoid visual artifacts
    f.render_widget(Clear, modal_area);

    let block = Block::default()
        .title("Add Item")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(c_border()))
        .style(Style::default().bg(c_bg_panel()));

    let inner = block.inner(modal_area);
    f.render_widget(block, modal_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Name input
            Constraint::Length(3), // Kind selection
            Constraint::Min(5),    // Value input (expanded for multi-line)
            Constraint::Length(5), // Instructions
        ])
        .split(inner);

    // Name input
    let name_block = Block::default().title("Name").borders(Borders::ALL).border_style(
        if matches!(app.add_focus, AddItemField::Name) {
            Style::default().fg(c_accent())
        } else {
            Style::default().fg(c_border())
        },
    );
    let name_input = Paragraph::new(app.add_name.as_str())
        .block(name_block)
        .style(Style::default().fg(c_text()));
    f.render_widget(name_input, chunks[0]);

    // Kind selection
    let selected_kind = ItemKind::all()[app.add_kind_idx.min(ItemKind::all().len() - 1)];
    let kind_text = format!("◄ {} ►", selected_kind.display_name());
    let kind_block = Block::default().title("Type").borders(Borders::ALL).border_style(
        if matches!(app.add_focus, AddItemField::Kind) {
            Style::default().fg(c_accent())
        } else {
            Style::default().fg(c_border())
        },
    );
    let kind_widget = Paragraph::new(kind_text)
        .block(kind_block)
        .style(Style::default().fg(if matches!(app.add_focus, AddItemField::Kind) {
            c_accent()
        } else {
            c_text()
        }))
        .alignment(Alignment::Center);
    f.render_widget(kind_widget, chunks[1]);

    let value_title = get_value_title_for_kind(selected_kind);

    // Create a mutable textarea for rendering
    let mut textarea = app.add_value_textarea.clone();

    // Set the block with proper styling
    textarea.set_block(
        Block::default()
            .title(format!("{value_title} (Enter for new line, Ctrl+Enter to save)"))
            .borders(Borders::ALL)
            .border_style(if matches!(app.add_focus, AddItemField::Value) {
                Style::default().fg(c_accent())
            } else {
                Style::default().fg(c_border())
            }),
    );

    // Set text and cursor styles
    textarea.set_style(Style::default().fg(c_text()));

    // Set cursor style when focused
    if matches!(app.add_focus, AddItemField::Value) {
        textarea.set_cursor_line_style(Style::default().bg(c_bg()));
        textarea.set_cursor_style(Style::default().bg(c_accent()));
    }

    // Enable line numbers with styling
    textarea.set_line_number_style(Style::default().fg(c_text_dim()));

    f.render_widget(&textarea, chunks[2]);

    // Instructions
    let instructions = match app.add_focus {
        AddItemField::Name => {
            vec![Line::from(vec![
                Span::styled("Enter ", Style::default().fg(c_text_dim())),
                Span::styled("Tab", Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)),
                Span::styled(" to continue", Style::default().fg(c_text_dim())),
            ])]
        }
        AddItemField::Kind => {
            vec![Line::from(vec![
                Span::styled("Use ", Style::default().fg(c_text_dim())),
                Span::styled("←/→", Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)),
                Span::styled(" to select, ", Style::default().fg(c_text_dim())),
                Span::styled("Tab", Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)),
                Span::styled(" to continue", Style::default().fg(c_text_dim())),
            ])]
        }
        AddItemField::Value => {
            vec![
                Line::from(vec![
                    Span::styled("Enter the ", Style::default().fg(c_text_dim())),
                    Span::styled(
                        get_value_title_for_kind(selected_kind).to_lowercase(),
                        Style::default().fg(c_accent()),
                    ),
                    Span::styled(" value", Style::default().fg(c_text_dim())),
                ]),
                Line::from(vec![
                    Span::styled("Press ", Style::default().fg(c_text_dim())),
                    Span::styled("Enter", Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)),
                    Span::styled(" for new line, ", Style::default().fg(c_text_dim())),
                    Span::styled(
                        "Ctrl+Enter",
                        Style::default().fg(c_accent()).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" to save", Style::default().fg(c_text_dim())),
                ]),
                Line::from(vec![
                    Span::styled("Press ", Style::default().fg(c_text_dim())),
                    Span::styled("Ctrl+v", Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)),
                    Span::styled(" to paste content from clipboard, ", Style::default().fg(c_text_dim())),
                ]),
            ]
        }
    };

    let instr_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(c_border()));
    let instr_text = Paragraph::new(instructions)
        .block(instr_block)
        .style(Style::default().fg(c_text_dim()))
        .wrap(Wrap { trim: false });
    f.render_widget(instr_text, chunks[3]);
}

const fn get_value_title_for_kind(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Password => "Password",
        ItemKind::EnvVar => "Environment Variable Value",
        ItemKind::Note => "Note Content",
        ItemKind::ApiKey => "API Key / Token",
        ItemKind::SshKey => "SSH Private Key",
        ItemKind::Certificate => "Certificate (PEM format)",
        ItemKind::Database => "Connection String",
        ItemKind::CreditCard => "Card Details",
        ItemKind::SecureNote => "Secure Note Content",
        ItemKind::Identity => "Identity Information",
        ItemKind::Server => "Server Credentials",
        ItemKind::WifiPassword => "WiFi Password",
        ItemKind::License => "License Key",
        ItemKind::BankAccount => "Account Details",
        ItemKind::Document => "Document Content",
        ItemKind::Recovery => "Recovery Codes",
        ItemKind::OAuth => "OAuth Token",
    }
}

#[allow(clippy::too_many_lines)]
fn draw_import_export(f: &mut Frame, app: &App) {
    let area = centered_rect(80, 70, f.area());
    f.render_widget(Clear, area);

    let title = match app.ie_mode {
        ImportExportMode::Export => " Export Items ",
        ImportExportMode::Import => " Import Items ",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(c_border()))
        .style(Style::default().bg(c_bg_panel()).fg(c_text()))
        .title(Span::styled(
            title,
            Style::default().fg(c_accent2()).add_modifier(Modifier::BOLD),
        ));
    f.render_widget(block, area);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Description
            Constraint::Length(5), // Path field with hints
            Constraint::Length(3), // Format field
            Constraint::Length(3), // Action button
            Constraint::Length(3), // Actions
            Constraint::Length(2), // Error
        ])
        .split(pad(area, 2, 2));

    // Description
    let desc_text = match app.ie_mode {
        ImportExportMode::Export => format!("Export {} items to file", app.items.len()),
        ImportExportMode::Import => "Import items from file".to_string(),
    };
    let desc = Paragraph::new(vec![
        Line::from(vec![Span::styled(&desc_text, Style::default().fg(c_text_dim()))]),
        Line::from(vec![Span::styled(
            "Tip: Use / for paths on all systems, ~ for home directory",
            Style::default().fg(c_text_dim()),
        )]),
    ]);
    f.render_widget(desc, inner[0]);

    let focused = |field: ImportExportField| app.ie_focus == field;

    // Path field with better hints
    let path_hint = match app.ie_mode {
        ImportExportMode::Export => "e.g., ~/Documents/backup.json or C:/backup.json",
        ImportExportMode::Import => "e.g., ~/Documents/data.csv or /path/to/import.json",
    };

    let path_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if focused(ImportExportField::Path) {
            Style::default().fg(c_accent())
        } else {
            Style::default().fg(c_border())
        })
        .style(Style::default().bg(Color::Rgb(30, 32, 40)))
        .title(Span::styled(" File Path ", Style::default().fg(c_text_dim())));

    let path_content = if app.ie_path.is_empty() && !focused(ImportExportField::Path) {
        Paragraph::new(vec![Line::from(Span::styled(
            path_hint,
            Style::default().fg(c_text_dim()),
        ))])
    } else {
        Paragraph::new(vec![
            Line::from(&*app.ie_path),
            Line::from(Span::styled(path_hint, Style::default().fg(c_text_dim()))),
        ])
    };

    let path_display = path_content.block(path_block).style(Style::default().fg(c_text()));
    f.render_widget(path_display, inner[1]);

    // Format field
    let format_display = format!("< {} >", app.ie_formats[app.ie_format_idx]);
    let format_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if focused(ImportExportField::Format) {
            Style::default().fg(c_accent())
        } else {
            Style::default().fg(c_border())
        })
        .style(Style::default().bg(Color::Rgb(30, 32, 40)))
        .title(Span::styled(" Format ", Style::default().fg(c_text_dim())));

    let format_content = Paragraph::new(Line::from(vec![
        Span::styled(
            &format_display,
            if focused(ImportExportField::Format) {
                Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(c_text())
            },
        ),
        if focused(ImportExportField::Format) {
            Span::styled("  [←/→ to change]", Style::default().fg(c_text_dim()))
        } else {
            Span::raw("")
        },
    ]))
    .block(format_block);
    f.render_widget(format_content, inner[2]);

    // Action button
    let action_text = match app.ie_mode {
        ImportExportMode::Export => "Export",
        ImportExportMode::Import => "Import",
    };
    let action_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if focused(ImportExportField::Action) {
            Style::default().fg(c_ok())
        } else {
            Style::default().fg(c_border())
        })
        .style(Style::default().bg(if focused(ImportExportField::Action) {
            Color::Rgb(20, 40, 20)
        } else {
            Color::Rgb(30, 32, 40)
        }));

    let action_content = Paragraph::new(Line::from(vec![Span::styled(
        format!("  {action_text}  "),
        Style::default().fg(c_ok()).add_modifier(Modifier::BOLD),
    )]))
    .block(action_block);
    f.render_widget(action_content, inner[3]);

    // Actions with file path help
    let actions = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("[Tab]", Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)),
            Span::styled(" switch   ", Style::default().fg(c_text_dim())),
            Span::styled("[Enter]", Style::default().fg(c_ok()).add_modifier(Modifier::BOLD)),
            Span::styled(" execute   ", Style::default().fg(c_text_dim())),
            Span::styled("[Esc]", Style::default().fg(c_err()).add_modifier(Modifier::BOLD)),
            Span::styled(" cancel", Style::default().fg(c_text_dim())),
        ]),
        Line::from(vec![
            Span::styled("Examples: ", Style::default().fg(c_text_dim())),
            Span::styled("./backup.json", Style::default().fg(c_accent())),
            Span::styled(", ", Style::default().fg(c_text_dim())),
            Span::styled("~/Documents/vault.csv", Style::default().fg(c_accent())),
        ]),
    ]);
    f.render_widget(actions, inner[4]);

    // Error/status display
    if let Some(err) = &app.error {
        let color = if err.contains("Exported") || err.contains("Imported") {
            c_ok()
        } else {
            c_err()
        };
        let err_p = Paragraph::new(Span::styled(
            err.clone(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
        f.render_widget(err_p, inner[5]);
    }
}

#[allow(clippy::too_many_lines)]
fn draw_generate_password(f: &mut Frame, app: &App) {
    let area = centered_rect(75, 80, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(c_border()))
        .style(Style::default().bg(c_bg_panel()).fg(c_text()))
        .title(Span::styled(
            " Password Generator ",
            Style::default().fg(c_accent2()).add_modifier(Modifier::BOLD),
        ));
    f.render_widget(block, area);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Title
            Constraint::Length(3), // Length field
            Constraint::Length(8), // Options
            Constraint::Length(4), // Generated password
            Constraint::Length(2), // Actions
            Constraint::Length(2), // Error
        ])
        .split(pad(area, 2, 2));

    // Title
    let title = Paragraph::new(Line::from(vec![Span::styled(
        "Configure and generate secure passwords",
        Style::default().fg(c_text_dim()),
    )]));
    f.render_widget(title, inner[0]);

    // Length field
    let length_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if matches!(app.gen_focus, PasswordGenField::Length) {
            Style::default().fg(c_accent())
        } else {
            Style::default().fg(c_border())
        })
        .style(Style::default().bg(Color::Rgb(30, 32, 40)))
        .title(Span::styled(" Length ", Style::default().fg(c_text_dim())));

    let length_content = Paragraph::new(&*app.gen_length_str)
        .block(length_block)
        .style(Style::default().fg(c_text()));
    f.render_widget(length_content, inner[1]);

    // Options
    let options_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if matches!(app.gen_focus, PasswordGenField::Options) {
            Style::default().fg(c_accent())
        } else {
            Style::default().fg(c_border())
        })
        .style(Style::default().bg(Color::Rgb(30, 32, 40)))
        .title(Span::styled(" Options ", Style::default().fg(c_text_dim())));

    let check_mark = |enabled: bool| if enabled { "☑" } else { "☐" };
    let options_lines = vec![
        Line::from(vec![
            Span::styled("[1] ", Style::default().fg(c_accent())),
            Span::styled(
                check_mark(app.gen_config.include_uppercase),
                Style::default().fg(c_text()),
            ),
            Span::styled(" Uppercase letters (A-Z)", Style::default().fg(c_text())),
        ]),
        Line::from(vec![
            Span::styled("[2] ", Style::default().fg(c_accent())),
            Span::styled(
                check_mark(app.gen_config.include_lowercase),
                Style::default().fg(c_text()),
            ),
            Span::styled(" Lowercase letters (a-z)", Style::default().fg(c_text())),
        ]),
        Line::from(vec![
            Span::styled("[3] ", Style::default().fg(c_accent())),
            Span::styled(check_mark(app.gen_config.include_digits), Style::default().fg(c_text())),
            Span::styled(" Digits (0-9)", Style::default().fg(c_text())),
        ]),
        Line::from(vec![
            Span::styled("[4] ", Style::default().fg(c_accent())),
            Span::styled(
                check_mark(app.gen_config.include_symbols),
                Style::default().fg(c_text()),
            ),
            Span::styled(" Symbols (!@#$%^&*...)", Style::default().fg(c_text())),
        ]),
        Line::from(vec![
            Span::styled("[5] ", Style::default().fg(c_accent())),
            Span::styled(
                check_mark(app.gen_config.exclude_ambiguous),
                Style::default().fg(c_text()),
            ),
            Span::styled(" Exclude ambiguous (0,O,1,l,I)", Style::default().fg(c_text())),
        ]),
    ];

    let options_content = Paragraph::new(options_lines).block(options_block);
    f.render_widget(options_content, inner[2]);

    // Generated password display
    let pwd_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(c_border()))
        .style(Style::default().bg(Color::Rgb(20, 25, 35)))
        .title(Span::styled(" Generated Password ", Style::default().fg(c_warn())));

    let pwd_content = if let Some(password) = &app.generated_password {
        Paragraph::new(password.clone())
            .style(Style::default().fg(c_accent()).add_modifier(Modifier::BOLD))
            .wrap(Wrap { trim: true })
    } else {
        Paragraph::new("Press 'g' or Enter to generate").style(Style::default().fg(c_text_dim()))
    };
    let pwd_display = pwd_content.block(pwd_block);
    f.render_widget(pwd_display, inner[3]);

    // Actions
    let actions = Paragraph::new(Line::from(vec![
        Span::styled("[Tab]", Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)),
        Span::styled(" switch   ", Style::default().fg(c_text_dim())),
        Span::styled("[g/Enter]", Style::default().fg(c_ok()).add_modifier(Modifier::BOLD)),
        Span::styled(" generate   ", Style::default().fg(c_text_dim())),
        Span::styled("[c]", Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)),
        Span::styled(" copy   ", Style::default().fg(c_text_dim())),
        Span::styled("[u]", Style::default().fg(c_warn()).add_modifier(Modifier::BOLD)),
        Span::styled(" use   ", Style::default().fg(c_text_dim())),
        Span::styled("[Esc]", Style::default().fg(c_err()).add_modifier(Modifier::BOLD)),
        Span::styled(" back", Style::default().fg(c_text_dim())),
    ]));
    f.render_widget(actions, inner[4]);

    // Error display
    if let Some(err) = &app.error {
        let err_p = Paragraph::new(Span::styled(
            err.clone(),
            Style::default()
                .fg(if err.contains("copied") { c_ok() } else { c_err() })
                .add_modifier(Modifier::BOLD),
        ));
        f.render_widget(err_p, inner[5]);
    }
}

fn draw_change_master(f: &mut Frame, app: &App) {
    let area = centered_rect(65, 70, f.area());
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(c_border()))
        .style(Style::default().bg(c_bg_panel()).fg(c_text()))
        .title(Span::styled(
            " Change Master Key ",
            Style::default().fg(c_accent2()).add_modifier(Modifier::BOLD),
        ));
    f.render_widget(block, area);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Length(2),
        ])
        .split(pad(area, 2, 2));

    let subtitle = Paragraph::new(Line::from(vec![
        Span::styled("Policy: ", Style::default().fg(c_text_dim())),
        Span::styled("≥8 chars, upper, lower, digit", Style::default().fg(c_accent())),
    ]));
    f.render_widget(subtitle, inner[0]);

    let focused = |foc: ChangeKeyField| app.ck_focus == foc;

    let current_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if focused(ChangeKeyField::Current) {
            Style::default().fg(c_accent())
        } else {
            Style::default().fg(c_border())
        })
        .style(Style::default().bg(Color::Rgb(30, 32, 40)))
        .title(Span::styled(" Current master key ", Style::default().fg(c_text_dim())));

    let current_content = Paragraph::new(mask(&app.ck_current))
        .block(current_block)
        .style(Style::default().fg(c_text()));
    f.render_widget(current_content, inner[1]);

    let new_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if focused(ChangeKeyField::New) {
            Style::default().fg(c_accent())
        } else {
            Style::default().fg(c_border())
        })
        .style(Style::default().bg(Color::Rgb(30, 32, 40)))
        .title(Span::styled(" New master key ", Style::default().fg(c_text_dim())));

    let new_content = Paragraph::new(mask(&app.ck_new))
        .block(new_block)
        .style(Style::default().fg(c_text()));
    f.render_widget(new_content, inner[2]);

    let confirm_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if focused(ChangeKeyField::Confirm) {
            Style::default().fg(c_accent())
        } else {
            Style::default().fg(c_border())
        })
        .style(Style::default().bg(Color::Rgb(30, 32, 40)))
        .title(Span::styled(
            " Confirm new master key ",
            Style::default().fg(c_text_dim()),
        ));

    let confirm_content = Paragraph::new(mask(&app.ck_confirm))
        .block(confirm_block)
        .style(Style::default().fg(c_text()));
    f.render_widget(confirm_content, inner[3]);

    let hints = Paragraph::new(Line::from(vec![
        Span::styled("[Tab]", Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)),
        Span::styled(" switch  ", Style::default().fg(c_text_dim())),
        Span::styled("[Enter]", Style::default().fg(c_ok()).add_modifier(Modifier::BOLD)),
        Span::styled(" apply  ", Style::default().fg(c_text_dim())),
        Span::styled("[Esc]", Style::default().fg(c_err()).add_modifier(Modifier::BOLD)),
        Span::styled(" cancel", Style::default().fg(c_text_dim())),
    ]));
    f.render_widget(hints, inner[4]);

    if let Some(err) = &app.error {
        let err_p = Paragraph::new(Span::styled(
            err.clone(),
            Style::default().fg(c_err()).add_modifier(Modifier::BOLD),
        ));
        f.render_widget(err_p, inner[5]);
    }
}

#[allow(clippy::too_many_lines)]
fn draw_view_item(f: &mut Frame, app: &App) {
    if let Some(item) = &app.view_item {
        let area = centered_rect(70, 60, f.area());
        f.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(c_border()))
            .style(Style::default().bg(c_bg_panel()).fg(c_text()))
            .title(Span::styled(
                " View Item ",
                Style::default().fg(c_accent2()).add_modifier(Modifier::BOLD),
            ));
        f.render_widget(block, area);

        let inner = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(6),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(2),
            ])
            .split(pad(area, 2, 2));

        let name_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(c_border()))
            .style(Style::default().bg(Color::Rgb(30, 32, 40)))
            .title(Span::styled(" Name ", Style::default().fg(c_text_dim())));
        let name_content = Paragraph::new(&*item.name)
            .block(name_block)
            .style(Style::default().fg(c_text()));
        f.render_widget(name_content, inner[0]);

        let (badge, color) = match item.kind.as_str() {
            "password" => ("Password", c_badge_pwd()),
            "env" => ("Environment Variable", c_badge_env()),
            _ => ("Note", c_badge_note()),
        };
        let kind_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(c_border()))
            .style(Style::default().bg(Color::Rgb(30, 32, 40)))
            .title(Span::styled(" Kind ", Style::default().fg(c_text_dim())));
        let kind_content = Paragraph::new(Line::from(vec![Span::styled(
            format!(" {badge} "),
            Style::default().bg(color).fg(Color::Black).add_modifier(Modifier::BOLD),
        )]))
        .block(kind_block);
        f.render_widget(kind_content, inner[1]);

        let value_display = if app.view_show_value {
            &item.value
        } else {
            "••••••••••••••••••••••••••••••••••••"
        };

        let value_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(c_border()))
            .style(Style::default().bg(Color::Rgb(30, 32, 40)))
            .title(Span::styled(
                if app.view_show_value {
                    " Value (visible) "
                } else {
                    " Value (hidden) "
                },
                Style::default().fg(if app.view_show_value { c_warn() } else { c_text_dim() }),
            ));
        let value_content = Paragraph::new(value_display)
            .block(value_block)
            .style(Style::default().fg(if app.view_show_value { c_text() } else { c_text_dim() }))
            .wrap(Wrap { trim: true });
        f.render_widget(value_content, inner[2]);

        let created_str = item
            .created_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "Unknown".to_string());
        let created_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(c_border()))
            .style(Style::default().bg(Color::Rgb(30, 32, 40)))
            .title(Span::styled(" Created ", Style::default().fg(c_text_dim())));
        let created_content = Paragraph::new(created_str)
            .block(created_block)
            .style(Style::default().fg(c_text()));
        f.render_widget(created_content, inner[3]);

        let updated_str = item
            .updated_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "Unknown".to_string());
        let updated_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(c_border()))
            .style(Style::default().bg(Color::Rgb(30, 32, 40)))
            .title(Span::styled(" Updated ", Style::default().fg(c_text_dim())));
        let updated_content = Paragraph::new(updated_str)
            .block(updated_block)
            .style(Style::default().fg(c_text()));
        f.render_widget(updated_content, inner[4]);

        let actions = Paragraph::new(Line::from(vec![
            Span::styled("[t/Enter]", Style::default().fg(c_warn()).add_modifier(Modifier::BOLD)),
            Span::styled(" Toggle visibility   ", Style::default().fg(c_text_dim())),
            Span::styled("[c]", Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)),
            Span::styled(" Copy   ", Style::default().fg(c_text_dim())),
            Span::styled("[Esc]", Style::default().fg(c_err()).add_modifier(Modifier::BOLD)),
            Span::styled(" Close", Style::default().fg(c_text_dim())),
        ]));
        f.render_widget(actions, inner[5]);
    }
}

fn draw_edit_item(f: &mut Frame, app: &App) {
    if let Some(item) = &app.edit_item {
        let area = centered_rect(70, 50, f.area());
        f.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(c_border()))
            .style(Style::default().bg(c_bg_panel()).fg(c_text()))
            .title(Span::styled(
                " Edit Item ",
                Style::default().fg(c_accent2()).add_modifier(Modifier::BOLD),
            ));
        f.render_widget(block, area);

        let inner = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(6),
                Constraint::Length(2),
            ])
            .split(pad(area, 2, 2));

        let subtitle = Paragraph::new(Line::from(vec![
            Span::styled("Editing: ", Style::default().fg(c_text_dim())),
            Span::styled(&item.name, Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)),
        ]));
        f.render_widget(subtitle, inner[0]);

        let name_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(c_border()))
            .style(Style::default().bg(Color::Rgb(40, 42, 50)))
            .title(Span::styled(" Name (read-only) ", Style::default().fg(c_text_dim())));
        let name_content = Paragraph::new(&*item.name)
            .block(name_block)
            .style(Style::default().fg(c_text_dim()));
        f.render_widget(name_content, inner[1]);

        let (badge, color) = match item.kind.as_str() {
            "password" => ("Password", c_badge_pwd()),
            "env" => ("Environment Variable", c_badge_env()),
            _ => ("Note", c_badge_note()),
        };
        let kind_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(c_border()))
            .style(Style::default().bg(Color::Rgb(40, 42, 50)))
            .title(Span::styled(" Kind (read-only) ", Style::default().fg(c_text_dim())));
        let kind_content = Paragraph::new(Line::from(vec![Span::styled(
            format!(" {badge} "),
            Style::default().bg(color).fg(Color::Black).add_modifier(Modifier::BOLD),
        )]))
        .block(kind_block);
        f.render_widget(kind_content, inner[2]);

        let value_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(c_accent()))
            .style(Style::default().bg(Color::Rgb(30, 32, 40)))
            .title(Span::styled(" New Value ", Style::default().fg(c_accent())));
        let value_content = Paragraph::new(&*app.edit_value)
            .block(value_block)
            .style(Style::default().fg(c_text()))
            .wrap(Wrap { trim: true });
        f.render_widget(value_content, inner[3]);

        let actions = Paragraph::new(Line::from(vec![
            Span::styled("[Enter]", Style::default().fg(c_ok()).add_modifier(Modifier::BOLD)),
            Span::styled(" Save changes   ", Style::default().fg(c_text_dim())),
            Span::styled("[Esc]", Style::default().fg(c_err()).add_modifier(Modifier::BOLD)),
            Span::styled(" Cancel   ", Style::default().fg(c_text_dim())),
            Span::styled("Type to edit value", Style::default().fg(c_text_dim())),
        ]));
        f.render_widget(actions, inner[4]);

        if let Some(err) = &app.error {
            let error_area = Rect {
                x: area.x + 2,
                y: area.y + area.height - 3,
                width: area.width - 4,
                height: 1,
            };
            let err_p = Paragraph::new(Span::styled(
                err.clone(),
                Style::default().fg(c_err()).add_modifier(Modifier::BOLD),
            ));
            f.render_widget(err_p, error_area);
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    let h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(v[1]);
    h[1]
}

const fn pad(r: Rect, x: u16, y: u16) -> Rect {
    Rect {
        x: r.x.saturating_add(x),
        y: r.y.saturating_add(y),
        width: r.width.saturating_sub(x.saturating_mul(2)),
        height: r.height.saturating_sub(y.saturating_mul(2)),
    }
}

fn field_box(content: &str, focused: bool) -> String {
    if focused {
        format!("[{content}]")
    } else {
        format!(" {content} ")
    }
}

fn mask(s: &str) -> String {
    if s.is_empty() {
        String::new()
    } else {
        "•".repeat(s.chars().count())
    }
}
