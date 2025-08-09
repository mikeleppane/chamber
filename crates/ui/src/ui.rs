use crate::app::{
    AddItemField, App, ChangeKeyField, ImportExportField, ImportExportMode, PasswordGenField, Screen, StatusType,
    UnlockField, ViewMode,
};
use anyhow::Result;
use chamber_vault::ItemKind;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
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
/// # Example
///
/// ```no_run
/// let mut app = App::new();
/// if let Err(e) = run_app(&mut app) {
///     eprintln!("Error: {}", e);
/// }
/// ```
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
fn handle_key(app: &mut App, key: crossterm::event::KeyEvent) -> Result<bool> {
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
                if !key.modifiers.contains(KeyModifiers::CONTROL) {
                    match app.unlock_focus {
                        UnlockField::Master => app.master_input.push(c),
                        UnlockField::Confirm => {
                            if app.master_mode_is_setup {
                                app.master_confirm_input.push(c);
                            }
                        }
                    }
                }
            }
            _ => {}
        },
        Screen::Main => {
            match key.code {
                KeyCode::Char('q') => return Ok(true),
                KeyCode::Char('a') => {
                    app.screen = Screen::AddItem;
                }
                KeyCode::Char('c') => {
                    app.copy_selected()?;
                }
                KeyCode::Char('v') => {
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
                    app.selected = (app.selected + 1).min(app.items.len().saturating_sub(1));
                }
                KeyCode::Up => {
                    app.selected = app.selected.saturating_sub(1);
                }
                KeyCode::Char('r') => {
                    app.refresh_items()?;
                }
                _ => {}
            }
        }
        Screen::AddItem => {
            match key.code {
                KeyCode::Esc => {
                    app.screen = Screen::Main;
                }
                KeyCode::Enter => {
                    app.add_item()?;
                }
                KeyCode::Tab => {
                    app.add_focus = match app.add_focus {
                        AddItemField::Name => AddItemField::Kind,
                        AddItemField::Kind => AddItemField::Value,
                        AddItemField::Value => AddItemField::Name,
                    };
                }
                KeyCode::Left | KeyCode::Right if matches!(app.add_focus, AddItemField::Kind) => {
                    let total_kinds = 7; // Password, EnvVar, Note, ApiKey, SshKey, Certificate, Database
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
                KeyCode::Up if matches!(app.add_focus, AddItemField::Value) => {
                    // Scroll up in value field
                    app.add_value_scroll = app.add_value_scroll.saturating_sub(1);
                }
                KeyCode::Down if matches!(app.add_focus, AddItemField::Value) => {
                    // Scroll down in value field
                    let lines_count = app.add_value.lines().count();
                    if lines_count > 5 {
                        // Only scroll if content is longer than visible area
                        app.add_value_scroll = (app.add_value_scroll + 1).min(lines_count.saturating_sub(5));
                    }
                }
                KeyCode::Char('v')
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && matches!(app.add_focus, AddItemField::Value) =>
                {
                    // Paste from clipboard
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        if let Ok(text) = clipboard.get_text() {
                            app.add_value = text;
                            app.add_value_scroll = 0; // Reset scroll after paste
                        }
                    }
                }
                KeyCode::Char('g') if matches!(app.add_focus, AddItemField::Value) => {
                    // Open password generator from add item screen
                    app.open_password_generator();
                }
                KeyCode::Backspace => match app.add_focus {
                    AddItemField::Name => {
                        app.add_name.pop();
                    }
                    AddItemField::Value => {
                        app.add_value.pop();
                        // Adjust scroll if content becomes shorter
                        let lines_count = app.add_value.lines().count();
                        if app.add_value_scroll >= lines_count && lines_count > 0 {
                            app.add_value_scroll = lines_count.saturating_sub(1);
                        }
                    }
                    AddItemField::Kind => {}
                },
                KeyCode::Char(c) => {
                    if !key.modifiers.contains(KeyModifiers::CONTROL) {
                        match app.add_focus {
                            AddItemField::Name => app.add_name.push(c),
                            AddItemField::Value => app.add_value.push(c),
                            AddItemField::Kind => {}
                        }
                    }
                }
                _ => {}
            }
        }

        Screen::ImportExport => {
            match key.code {
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
                    if !key.modifiers.contains(KeyModifiers::CONTROL) {
                        app.ie_path.push(c);
                    }
                }
                _ => {}
            }
        }

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
                if !key.modifiers.contains(KeyModifiers::CONTROL) {
                    match app.ck_focus {
                        ChangeKeyField::Current => app.ck_current.push(c),
                        ChangeKeyField::New => app.ck_new.push(c),
                        ChangeKeyField::Confirm => app.ck_confirm.push(c),
                    }
                }
            }
            _ => {}
        },
        Screen::GeneratePassword => {
            match key.code {
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
                KeyCode::Enter => {
                    if app.gen_focus == PasswordGenField::Generate {
                        app.generate_password();
                    }
                }
                KeyCode::Char('g') => {
                    app.generate_password();
                }
                KeyCode::Char('c') => {
                    app.copy_generated_password()?;
                }
                KeyCode::Char('u') => {
                    app.use_generated_password();
                }
                KeyCode::Char(' ') if matches!(app.gen_focus, PasswordGenField::Options) => {
                    // Toggle options with spacebar
                    app.gen_config.include_uppercase = !app.gen_config.include_uppercase;
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
                KeyCode::Backspace if matches!(app.gen_focus, PasswordGenField::Length) => {
                    app.gen_length_str.pop();
                }
                KeyCode::Char(c) if matches!(app.gen_focus, PasswordGenField::Length) => {
                    if c.is_ascii_digit() {
                        app.gen_length_str.push(c);
                    }
                }
                _ => {}
            }
        }
        Screen::ViewItem => match key.code {
            KeyCode::Esc => {
                app.view_item = None;
                app.screen = Screen::Main;
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                app.toggle_value_visibility();
            }
            KeyCode::Char('c') => {
                app.copy_selected()?;
            }
            _ => {}
        },
        Screen::EditItem => match key.code {
            KeyCode::Esc => {
                app.edit_item = None;
                app.edit_value.clear();
                app.screen = Screen::Main;
            }
            KeyCode::Enter => {
                app.save_edit()?;
            }
            KeyCode::Backspace => {
                app.edit_value.pop();
            }
            KeyCode::Char(c) => {
                if !key.modifiers.contains(KeyModifiers::CONTROL) {
                    app.edit_value.push(c);
                }
            }
            _ => {}
        },
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
    }
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
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(centered_rect(96, 90, body));

    // Create categorized list with enhanced visual separation
    let mut list_items: Vec<ListItem> = Vec::new();
    let mut current_category = None;

    for (index, item) in app.filtered_items.iter().enumerate() {
        // Add category header if needed
        let item_category = item.kind.as_str();
        if current_category != Some(item_category) {
            current_category = Some(item_category);

            // Add spacing before new category (except for first)
            if !list_items.is_empty() {
                list_items.push(ListItem::new(Line::from("")));
            }

            // Category header
            let (category_name, category_icon, category_color) = match item.kind {
                ItemKind::Password => ("🔐 PASSWORDS", "", c_badge_pwd()),
                ItemKind::EnvVar => ("🌍 ENVIRONMENT VARIABLES", "", c_badge_env()),
                ItemKind::Note => ("📝 NOTES", "", c_badge_note()),
                ItemKind::ApiKey => ("🔑 API KEYS", "", c_badge_pwd()), // Or create c_badge_api()
                ItemKind::SshKey => ("🔐 SSH KEYS", "", c_badge_pwd()),
                ItemKind::Certificate => ("📜 CERTIFICATES", "", c_badge_pwd()),
                ItemKind::Database => ("🗄️ DATABASES", "", c_badge_pwd()),
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
            ItemKind::EnvVar => ("🌍", c_badge_env()),
            ItemKind::Note => ("📄", c_badge_note()),
            ItemKind::ApiKey => ("🔑", c_badge_pwd()),
            ItemKind::Certificate => ("📜", c_badge_pwd()),
            ItemKind::Database => ("🗄️", c_badge_pwd()),
            ItemKind::Password | ItemKind::SshKey => ("🔐", c_badge_pwd()),
        };

        // Enhanced item display with better formatting
        #[allow(clippy::expect_used)]
        let created_date = item
            .created_at
            .format(&time::format_description::parse("[year]-[month]-[day]").expect("Invalid date format"))
            .unwrap_or_else(|_| "unknown".to_string());

        let item_line = Line::from(vec![
            Span::raw("    "), // Indentation for items under category
            Span::styled(format!("{badge} "), Style::default().fg(badge_color)),
            Span::styled(&item.name, Style::default().fg(c_text()).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" ({created_date})"), Style::default().fg(c_text_dim())),
        ]);

        // Check if this item is selected
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

    // If no items, show empty state
    if list_items.is_empty() {
        let empty_message = match app.view_mode {
            ViewMode::All => "No items in vault",
            ViewMode::Passwords => "No passwords stored",
            ViewMode::Environment => "No environment variables stored",
            ViewMode::Notes => "No notes stored",
        };

        list_items.push(ListItem::new(Line::from(vec![Span::styled(
            format!("    📭 {empty_message}"),
            Style::default().fg(c_text_dim()),
        )])));
    }

    let items_title = format!(
        " {} ({}/{}) ",
        app.view_mode.as_str(),
        app.filtered_items.len(),
        app.items.len()
    );

    let list = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(c_border()))
            .style(Style::default().bg(c_bg_panel()))
            .title(Span::styled(
                &items_title,
                Style::default().fg(c_accent()).add_modifier(Modifier::BOLD),
            )),
    );

    // Note: We don't use stateful rendering since we handle selection manually
    f.render_widget(list, chunks[0]);

    // Enhanced help panel with category information
    let (_, passwords, env_vars, notes) = app.get_item_counts();

    let help_lines = vec![
        Line::from(Span::styled(
            "Categories",
            Style::default().fg(c_text_dim()).add_modifier(Modifier::BOLD),
        )),
        Line::default(),
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
        Line::default(),
        Line::from(Span::styled(
            "Actions",
            Style::default().fg(c_text_dim()).add_modifier(Modifier::BOLD),
        )),
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
            Span::raw("Copy value to clipboard"),
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
            Span::styled("q ", Style::default().fg(c_accent()).add_modifier(Modifier::BOLD)),
            Span::raw("Quit"),
        ]),
    ];

    let help = Paragraph::new(help_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(c_border()))
                .style(Style::default().bg(c_bg_panel()))
                .title(Span::styled(
                    " Help ",
                    Style::default().fg(c_accent2()).add_modifier(Modifier::BOLD),
                )),
        )
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(c_text()));
    f.render_widget(help, chunks[1]);
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
                let (total, passwords, env_vars, notes) = app.get_item_counts();
                format!("{total} items ({passwords} passwords, {env_vars} env vars, {notes} notes)",)
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
            add_hint(&mut spans, "↑↓", "Navigate", false);
            add_hint(&mut spans, "Enter", "View", true);
            add_hint(&mut spans, "a", "Add", false);
            add_hint(&mut spans, "e", "Edit", false);
            add_hint(&mut spans, "Del", "Delete", false);
        }
        Screen::AddItem => {
            add_hint(&mut spans, "Tab", "Next Field", false);
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
    }

    // Add a trailing space for padding
    spans.push(Span::raw(" "));
    spans
}

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
            Constraint::Length(3), // Instructions
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

    // Value input with scrolling support
    let value_title = get_value_title_for_kind(selected_kind);
    let value_block = Block::default()
        .title(format!("{value_title} (↑↓ to scroll)"))
        .borders(Borders::ALL)
        .border_style(if matches!(app.add_focus, AddItemField::Value) {
            Style::default().fg(c_accent())
        } else {
            Style::default().fg(c_border())
        });

    // Handle scrollable display
    let lines: Vec<&str> = app.add_value.lines().collect();
    let visible_height = chunks[2].height.saturating_sub(2) as usize; // Account for borders

    let display_lines = if lines.len() > visible_height {
        let start = app.add_value_scroll;
        let end = (start + visible_height).min(lines.len());
        lines[start..end].join("\n")
    } else {
        app.add_value.clone()
    };

    let value_input = Paragraph::new(display_lines)
        .block(value_block)
        .style(Style::default().fg(c_text()))
        .wrap(Wrap { trim: false });
    f.render_widget(value_input, chunks[2]);

    // Instructions
    let instructions = get_instructions_for_kind(selected_kind);
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
    }
}

const fn get_instructions_for_kind(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Password => "Enter the password to store securely.",
        ItemKind::EnvVar => "Enter the environment variable value (e.g., API_URL=https://api.example.com).",
        ItemKind::Note => "Enter any text content, notes, or information.",
        ItemKind::ApiKey => "Enter API key, bearer token, or authentication token.",
        ItemKind::SshKey => "Paste SSH private key in OpenSSH or PEM format.",
        ItemKind::Certificate => "Paste certificate in PEM format (-----BEGIN CERTIFICATE-----).",
        ItemKind::Database => "Enter connection string (e.g., postgresql://user:pass@host:5432/db).",
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
            Span::styled(
                "[Space/Enter]",
                Style::default().fg(c_warn()).add_modifier(Modifier::BOLD),
            ),
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
