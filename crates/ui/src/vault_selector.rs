use chamber_vault::{VaultInfo, VaultManager};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    prelude::{Alignment, Color, Constraint, Frame, Layout, Modifier, Rect, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

#[derive(Debug, Clone, PartialEq)]
pub enum VaultSelectorMode {
    Select, // Normal selection mode
    Create, // Creating a new vault
    Edit,   // Editing vault info
    Delete, // Confirming deletion
    Import, // Importing vault
}

pub struct VaultSelector {
    pub vaults: Vec<VaultInfo>,
    pub state: ListState,
    pub show_selector: bool,
    pub mode: VaultSelectorMode,
    pub input_buffer: String,
    pub input_field: InputField,
    pub confirmation_message: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputField {
    None,
    Name,
    Description,
    Category,
    Path,
}

impl Default for VaultSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl VaultSelector {
    pub fn new() -> Self {
        Self {
            vaults: Vec::new(),
            state: ListState::default(),
            show_selector: false,
            mode: VaultSelectorMode::Select,
            input_buffer: String::new(),
            input_field: InputField::None,
            confirmation_message: String::new(),
            error_message: None,
        }
    }

    pub fn load_vaults(&mut self, manager: &VaultManager) {
        self.vaults = manager.list_vaults().into_iter().cloned().collect();
        if !self.vaults.is_empty() && self.state.selected().is_none() {
            self.state.select(Some(0));
        }
    }

    pub fn show(&mut self) {
        self.show_selector = true;
        self.mode = VaultSelectorMode::Select;
        self.clear_input();
    }

    pub fn hide(&mut self) {
        self.show_selector = false;
        self.clear_input();
    }

    pub fn start_create(&mut self) {
        self.mode = VaultSelectorMode::Create;
        self.input_field = InputField::Name;
        self.input_buffer.clear();
        self.error_message = None;
    }

    pub fn start_edit(&mut self) {
        // Get the vault name first, then modify self
        let vault_name = if let Some(vault) = self.selected_vault() {
            vault.name.clone() // Clone the name to avoid borrowing issues
        } else {
            return; // Early return if no vault is selected
        };

        // Now we can safely modify self
        self.mode = VaultSelectorMode::Edit;
        self.input_field = InputField::Name;
        self.input_buffer = vault_name; // Use the cloned name
        self.error_message = None;
    }

    pub fn start_delete(&mut self) {
        // Get the vault name first, then modify self
        let vault_name = if let Some(vault) = self.selected_vault() {
            vault.name.clone() // Clone the name to avoid borrowing issues
        } else {
            return; // Early return if no vault is selected
        };

        // Now we can safely modify self
        self.mode = VaultSelectorMode::Delete;
        self.confirmation_message = format!(
            "Are you sure you want to delete vault '{vault_name}'?\nThis action cannot be undone!" // Use the cloned name
        );
    }

    pub fn next(&mut self) {
        if self.vaults.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => (i + 1) % self.vaults.len(),
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.vaults.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.vaults.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn selected_vault(&self) -> Option<&VaultInfo> {
        self.state.selected().and_then(|i| self.vaults.get(i))
    }

    pub fn handle_input(&mut self, key: KeyEvent) -> Option<VaultAction> {
        match self.mode {
            VaultSelectorMode::Select => self.handle_select_input(key),
            VaultSelectorMode::Create => self.handle_create_input(key),
            VaultSelectorMode::Edit => self.handle_edit_input(key),
            VaultSelectorMode::Delete => self.handle_delete_input(key),
            VaultSelectorMode::Import => self.handle_import_input(key),
        }
    }

    fn handle_select_input(&mut self, key: KeyEvent) -> Option<VaultAction> {
        match key.code {
            KeyCode::Up => {
                self.previous();
                None
            }
            KeyCode::Down => {
                self.next();
                None
            }
            KeyCode::Enter => self.selected_vault().map(|vault| VaultAction::Switch(vault.id.clone())),
            KeyCode::Char('n') => {
                self.start_create();
                None
            }
            KeyCode::Char('e') => {
                self.start_edit();
                None
            }
            KeyCode::Delete | KeyCode::Char('d') => {
                self.start_delete();
                None
            }
            KeyCode::Char('i') => {
                self.mode = VaultSelectorMode::Import;
                self.input_field = InputField::Path;
                self.input_buffer.clear();
                None
            }
            KeyCode::Char('r') => Some(VaultAction::Refresh),
            KeyCode::F(1) => Some(VaultAction::ShowHelp),
            KeyCode::Esc => Some(VaultAction::Close),
            _ => None,
        }
    }

    fn handle_create_input(&mut self, key: KeyEvent) -> Option<VaultAction> {
        match key.code {
            KeyCode::Enter => {
                match self.input_field {
                    InputField::Name => {
                        if self.input_buffer.trim().is_empty() {
                            self.error_message = Some("Vault name cannot be empty".to_string());
                            return None;
                        }
                        self.input_field = InputField::Description;
                        self.input_buffer.clear();
                    }
                    InputField::Description => {
                        self.input_field = InputField::Category;
                        self.input_buffer = "personal".to_string();
                    }
                    InputField::Category => {
                        // Create the vault
                        return Some(VaultAction::Create {
                            name: self.get_field_value("name"),
                            description: if self.get_field_value("description").is_empty() {
                                None
                            } else {
                                Some(self.get_field_value("description"))
                            },
                            category: self.input_buffer.clone(),
                        });
                    }
                    _ => {}
                }
                None
            }
            KeyCode::Esc => {
                self.mode = VaultSelectorMode::Select;
                self.clear_input();
                None
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
                None
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
                None
            }
            _ => None,
        }
    }

    fn handle_edit_input(&mut self, key: KeyEvent) -> Option<VaultAction> {
        match key.code {
            KeyCode::Enter => {
                if let Some(vault) = self.selected_vault() {
                    return Some(VaultAction::Update {
                        vault_id: vault.id.clone(),
                        name: Some(self.input_buffer.clone()),
                        description: None,
                        category: None,
                        favorite: None,
                    });
                }
                None
            }
            KeyCode::Esc => {
                self.mode = VaultSelectorMode::Select;
                self.clear_input();
                None
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
                None
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
                None
            }
            _ => None,
        }
    }

    fn handle_delete_input(&mut self, key: KeyEvent) -> Option<VaultAction> {
        match key.code {
            KeyCode::Char('y' | 'Y') => {
                self.selected_vault().map(|vault| VaultAction::Delete {
                    vault_id: vault.id.clone(),
                    delete_file: false, // Default to not deleting the file
                })
            }
            KeyCode::Char('n' | 'N') | KeyCode::Esc => {
                self.mode = VaultSelectorMode::Select;
                self.clear_input();
                None
            }
            _ => None,
        }
    }

    fn handle_import_input(&mut self, key: KeyEvent) -> Option<VaultAction> {
        match key.code {
            KeyCode::Enter => Some(VaultAction::Import {
                path: self.input_buffer.clone(),
            }),
            KeyCode::Esc => {
                self.mode = VaultSelectorMode::Select;
                self.clear_input();
                None
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
                None
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
                None
            }
            _ => None,
        }
    }

    fn clear_input(&mut self) {
        self.input_buffer.clear();
        self.input_field = InputField::None;
        self.error_message = None;
        self.confirmation_message.clear();
    }

    fn get_field_value(&self, _: &str) -> String {
        // In a real implementation, you'd store intermediate values
        // For now, we'll use the current buffer
        self.input_buffer.clone()
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        match self.mode {
            VaultSelectorMode::Select => self.render_select_mode(frame, area),
            VaultSelectorMode::Create => self.render_create_mode(frame, area),
            VaultSelectorMode::Edit => self.render_edit_mode(frame, area),
            VaultSelectorMode::Delete => self.render_delete_mode(frame, area),
            VaultSelectorMode::Import => self.render_import_mode(frame, area),
        }
    }

    fn render_select_mode(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Vault Manager")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        if self.vaults.is_empty() {
            let empty_msg = Paragraph::new("No vaults found. Press 'n' to create a new vault.")
                .block(block)
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center);
            frame.render_widget(empty_msg, area);
            return;
        }

        let items: Vec<ListItem> = self
            .vaults
            .iter()
            .map(|vault| {
                let status = if vault.is_active { " (active)" } else { "" };
                let favorite = if vault.is_favorite { " ⭐" } else { "" };

                let content = format!("{} [{}]{}{}", vault.name, vault.category, favorite, status);

                let style = if vault.is_active {
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                ListItem::new(content).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
            .highlight_symbol("► ");

        frame.render_stateful_widget(list, area, &mut self.state);

        // Show help text
        let help_area = Rect {
            y: area.y + area.height.saturating_sub(2),
            height: 2,
            ..area
        };

        let help_text = "Enter: Select | n: New | e: Edit | d: Delete | i: Import | r: Refresh | F1: Help | Esc: Close";
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .wrap(Wrap { trim: true });

        frame.render_widget(help, help_area);

        // Show error if any
        if let Some(error) = &self.error_message {
            let error_area = Rect {
                y: area.y + area.height.saturating_sub(4),
                height: 2,
                ..area
            };
            let error_msg = Paragraph::new(error.as_str())
                .style(Style::default().fg(Color::Red))
                .wrap(Wrap { trim: true });
            frame.render_widget(error_msg, error_area);
        }
    }

    fn render_create_mode(&mut self, frame: &mut Frame, area: Rect) {
        // Clear the background
        frame.render_widget(Clear, area);

        let popup_area = centered_rect(60, 50, area);

        let title = match self.input_field {
            InputField::Name => "Create New Vault - Enter Name",
            InputField::Description => "Create New Vault - Enter Description (Optional)",
            InputField::Category => "Create New Vault - Select Category",
            _ => "Create New Vault",
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let inner_area = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        let (content, cursor_line, cursor_col) = if self.input_field == InputField::Category {
            let instruction = "Choose some default available categories (personal, work, team, project, testing, archive) \
            or choose your own\nEnter category: ";
            let content = instruction.to_string() + &self.input_buffer;

            // Calculate cursor position accounting for text wrapping
            let available_width = inner_area.width as usize;
            let mut current_line = 0;
            let mut current_col = 0;

            // Process the instruction text first
            for ch in instruction.chars() {
                if ch == '\n' {
                    current_line += 1;
                    current_col = 0;
                } else if current_col >= available_width {
                    // Line wrapped
                    current_line += 1;
                    current_col = 1; // This character starts the new line
                } else {
                    current_col += 1;
                }
            }

            // Now add the input buffer position
            for _ in self.input_buffer.chars() {
                if current_col >= available_width {
                    // Line wrapped
                    current_line += 1;
                    current_col = 1; // This character starts the new line
                } else {
                    current_col += 1;
                }
            }

            (content, current_line, current_col)
        } else {
            // For single-line fields, still account for wrapping
            let available_width = inner_area.width as usize;
            let mut current_line = 0;
            let mut current_col = 0;

            for _ in self.input_buffer.chars() {
                if current_col >= available_width {
                    current_line += 1;
                    current_col = 1;
                } else {
                    current_col += 1;
                }
            }

            (self.input_buffer.clone(), current_line, current_col)
        };

        let input = Paragraph::new(content)
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true });

        frame.render_widget(input, inner_area);

        // Show cursor at the correct position
        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::cast_sign_loss)]
        let cursor_area = Rect {
            x: inner_area.x + cursor_col as u16,
            y: inner_area.y + cursor_line as u16,
            width: 1,
            height: 1,
        };
        frame.render_widget(Block::default().style(Style::default().bg(Color::White)), cursor_area);

        // Show error if any
        if let Some(error) = &self.error_message {
            let error_area = Rect {
                y: inner_area.y + 2,
                height: 1,
                ..inner_area
            };
            let error_msg = Paragraph::new(error.as_str()).style(Style::default().fg(Color::Red));
            frame.render_widget(error_msg, error_area);
        }
    }

    fn render_edit_mode(&mut self, frame: &mut Frame, area: Rect) {
        // Clear the background
        frame.render_widget(Clear, area);

        let popup_area = centered_rect(50, 30, area);

        let block = Block::default()
            .title("Edit Vault Name")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let inner_area = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        let input = Paragraph::new(self.input_buffer.clone()).style(Style::default().fg(Color::White));

        frame.render_widget(input, inner_area);

        // Show cursor
        #[allow(clippy::cast_possible_truncation)]
        let cursor_area = Rect {
            x: inner_area.x + self.input_buffer.len() as u16,
            y: inner_area.y,
            width: 1,
            height: 1,
        };
        frame.render_widget(Block::default().style(Style::default().bg(Color::White)), cursor_area);
    }

    fn render_delete_mode(&mut self, frame: &mut Frame, area: Rect) {
        // Clear the background
        frame.render_widget(Clear, area);

        let popup_area = centered_rect(60, 40, area);

        let block = Block::default()
            .title("Confirm Deletion")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red));

        let inner_area = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        let confirmation = Paragraph::new(self.confirmation_message.clone())
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center);

        frame.render_widget(confirmation, inner_area);

        let help_area = Rect {
            y: inner_area.y + inner_area.height.saturating_sub(2),
            height: 1,
            ..inner_area
        };

        let help = Paragraph::new("Press 'y' to confirm, 'n' or Esc to cancel")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        frame.render_widget(help, help_area);
    }

    fn render_import_mode(&mut self, frame: &mut Frame, area: Rect) {
        // Clear the background
        frame.render_widget(Clear, area);

        let popup_area = centered_rect(60, 30, area);

        let block = Block::default()
            .title("Import Vault - Enter Path")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue));

        let inner_area = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        let input = Paragraph::new(self.input_buffer.clone()).style(Style::default().fg(Color::White));

        frame.render_widget(input, inner_area);

        // Show cursor
        #[allow(clippy::cast_possible_truncation)]
        let cursor_area = Rect {
            x: inner_area.x + self.input_buffer.len() as u16,
            y: inner_area.y,
            width: 1,
            height: 1,
        };
        frame.render_widget(Block::default().style(Style::default().bg(Color::White)), cursor_area);
    }
}

#[derive(Debug, Clone)]
pub enum VaultAction {
    Switch(String),
    Create {
        name: String,
        description: Option<String>,
        category: String,
    },
    Update {
        vault_id: String,
        name: Option<String>,
        description: Option<String>,
        category: Option<String>,
        favorite: Option<bool>,
    },
    Delete {
        vault_id: String,
        delete_file: bool,
    },
    Import {
        path: String,
    },
    Refresh,
    ShowHelp,
    Close,
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
