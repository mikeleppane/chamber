use crate::app;
use anyhow::{Result, anyhow};
use chamber_import_export::{ExportFormat, export_items, import_items};
use chamber_password_gen::PasswordConfig;
use chamber_vault::{Item, ItemKind, NewItem, Vault};
use ratatui::prelude::Style;
use ratatui::style::Color;
use std::path::PathBuf;
use tui_textarea::TextArea;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Unlock,
    Main,
    AddItem,
    ViewItem,
    EditItem,
    ChangeMaster,
    GeneratePassword,
    ImportExport,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UnlockField {
    Master,
    Confirm,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ChangeKeyField {
    Current,
    New,
    Confirm,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AddItemField {
    Name,
    Kind,
    Value,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PasswordGenField {
    Length,
    Options,
    Generate,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ImportExportField {
    Path,
    Format,
    Action,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ImportExportMode {
    Export,
    Import,
}

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    All,
    Passwords,
    Environment,
    Notes,
}

impl ViewMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            ViewMode::All => "Items",
            ViewMode::Passwords => "Passwords",
            ViewMode::Environment => "Environment",
            ViewMode::Notes => "Notes",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StatusType {
    Info,
    Success,
    Warning,
    Error,
}

pub struct App {
    pub vault: Vault,
    pub screen: Screen,
    pub master_input: String,
    pub master_confirm_input: String,
    pub master_mode_is_setup: bool,
    pub unlock_focus: UnlockField,
    pub error: Option<String>,

    pub items: Vec<Item>,
    pub selected: usize,
    pub view_mode: ViewMode,
    pub filtered_items: Vec<Item>,
    pub search_query: String,

    pub add_name: String,
    pub add_kind_idx: usize,
    pub add_value: String,
    pub add_value_scroll: usize,
    pub status_message: Option<String>,
    pub status_type: StatusType,
    pub scroll_offset: usize,
    pub add_value_textarea: TextArea<'static>,

    // Change passes key dialog fields
    pub ck_current: String,
    pub ck_new: String,
    pub ck_confirm: String,
    pub ck_focus: ChangeKeyField,
    pub add_focus: AddItemField,
    pub view_item: Option<Item>,
    pub view_show_value: bool,
    pub edit_item: Option<Item>,
    pub edit_value: String,

    // Password generation fields
    pub gen_focus: PasswordGenField,
    pub gen_length_str: String,
    pub gen_config: PasswordConfig,
    pub generated_password: Option<String>,

    // Import/Export fields
    pub ie_focus: ImportExportField,
    pub ie_mode: ImportExportMode,
    pub ie_path: String,
    pub ie_format_idx: usize,
    pub ie_formats: Vec<&'static str>,
}

impl App {
    /// Initializes a new instance of the struct.
    ///
    /// This function creates or opens a vault, determines whether the master mode setup
    /// is required, and initializes the various fields required for managing the application state.
    ///
    /// # Returns
    /// - `Result<Self>`: A `Result` containing the initialized struct instance on success,
    ///   or an error if the vault fails to open or create.
    ///
    /// # Fields
    /// - `vault`: Handles secure storage by opening or creating a vault.
    /// - `screen`: Represents the current active screen, initialized to the `Unlock` screen.
    /// - `master_input`: Stores user input for the master password during setup or unlock phase.
    /// - `master_confirm_input`: Stores user input for confirming the master password during setup.
    /// - `master_mode_is_setup`: Indicates whether the master mode is set up (false if initialization is incomplete).
    /// - `unlock_focus`: Tracks which unlock field is currently focused (e.g., Master field).
    /// - `error`: Holds any error message or state, defaulted to `None`.
    /// - `items`: A vector holding all items stored in the vault.
    /// - `selected`: Tracks the index of the currently selected item in the items list.
    /// - `view_mode`: Specifies the current filter/view mode for items (e.g., All items).
    /// - `filtered_items`: A vector holding the subset of items that match the current search query or filter.
    /// - `search_query`: Stores the user's current search input or query.
    /// - `add_name`: Field for the name of an item to be added.
    /// - `add_kind_idx`: Indicates the index of the kind/type of the item being added.
    /// - `add_value`: The value of the item being added.
    /// - `add_value_scroll`: Tracks the scroll state for long values when adding an item.
    /// - `status_message`: Holds transient status messages to display to the user.
    /// - `status_type`: Indicates the type of status message (e.g., Info, Warning, Error).
    ///
    /// # Change Key Fields
    /// - `ck_current`: Stores the current master key value.
    /// - `ck_new`: Stores the new master key value.
    /// - `ck_confirm`: Confirms the new master key value.
    /// - `ck_focus`: Tracks which field is focused during the change key process.
    ///
    /// # Add Item Fields
    /// - `add_focus`: Tracks which field is focused when adding a new item (e.g., Name).
    ///
    /// # Viewing and Editing Items
    /// - `view_item`: The currently selected item for viewing, if any.
    /// - `view_show_value`: Indicates whether to reveal the value of the viewed item.
    /// - `edit_item`: The item currently being edited, if any.
    /// - `edit_value`: The edited value of the currently selected item.
    ///
    /// # Password Generation
    /// - `gen_focus`: The current focus field in the password generation process (e.g., Length).
    /// - `gen_length_str`: String representation of the desired password length (default: "16").
    /// - `gen_config`: Configuration settings for password generation (e.g., character set, length).
    /// - `generated_password`: Holds the last generated password, if any.
    ///
    /// # Import/Export
    /// - `ie_focus`: Tracks which field is focused during import/export operations (e.g., File Path).
    /// - `ie_mode`: Indicates the mode (Import or Export) for import/export operations.
    /// - `ie_path`: Stores the file path selected for import/export.
    /// - `ie_format_idx`: Tracks the index of the currently selected format for import/export.
    /// - `ie_formats`: A vector containing supported file formats for import/export (e.g., "json", "csv").
    ///
    /// # Errors
    /// Returns an error if the vault cannot be opened or created successfully.
    ///
    /// # Example
    /// ```
    /// let instance = MyStruct::new()?;
    /// ```
    pub fn new() -> Result<Self> {
        let vault = Vault::open_or_create(None)?;
        let master_mode_is_setup = !vault.is_initialized();
        Ok(Self {
            vault,
            screen: Screen::Unlock,
            master_input: String::new(),
            master_confirm_input: String::new(),
            master_mode_is_setup,
            unlock_focus: UnlockField::Master,
            error: None,
            items: vec![],
            selected: 0,
            view_mode: ViewMode::All,
            filtered_items: vec![],
            search_query: String::new(),
            add_name: String::new(),
            add_kind_idx: 0,
            add_value: String::new(),
            add_value_scroll: 0,
            status_message: None,
            status_type: StatusType::Info,
            scroll_offset: 0,
            add_value_textarea: {
                let mut textarea = TextArea::default();
                // Enable line numbers
                textarea.set_line_number_style(Style::default().fg(Color::DarkGray));
                textarea.set_cursor_line_style(Style::default());
                // Optional: set a placeholder text
                textarea.set_placeholder_text("Enter your value here...");
                textarea
            },

            ck_current: String::new(),
            ck_new: String::new(),
            ck_confirm: String::new(),
            ck_focus: ChangeKeyField::Current,
            add_focus: AddItemField::Name,
            view_item: None,
            view_show_value: false,
            edit_item: None,
            edit_value: String::new(),

            // Initialize password generation fields
            gen_focus: PasswordGenField::Length,
            gen_length_str: "16".to_string(),
            gen_config: PasswordConfig::default(),
            generated_password: None,

            // Initialize import/export fields
            ie_focus: ImportExportField::Path,
            ie_mode: ImportExportMode::Export,
            ie_path: String::new(),
            ie_format_idx: 0,
            ie_formats: vec!["json", "csv", "backup"],
        })
    }

    fn validate_master_strength(s: &str) -> Result<()> {
        if s.len() < 8 {
            return Err(anyhow!("Master key must be at least 8 characters long"));
        }
        if !s.chars().any(|c| c.is_ascii_lowercase()) {
            return Err(anyhow!("Master key must contain a lowercase letter"));
        }
        if !s.chars().any(|c| c.is_ascii_uppercase()) {
            return Err(anyhow!("Master key must contain an uppercase letter"));
        }
        if !s.chars().any(|c| c.is_ascii_digit()) {
            return Err(anyhow!("Master key must contain a digit"));
        }
        Ok(())
    }

    /// Unlocks the application vault using the provided master key and performs necessary validations.
    ///
    /// This function checks if the master key setup process is initiated, validates the user input,
    /// and sets up or unlocks the vault accordingly. In case of errors during validation or unlocking
    /// operations, appropriate error messages are set.
    ///
    /// ## Steps
    /// 1. If `master_mode_is_setup` is true:
    ///     - Validate the presence of master input and confirmation input. If either is empty, an error
    ///       message is set, and the function will return early.
    ///     - Compare `master_input` and `master_confirm_input`. If they do not match, an error message
    ///       is set, and the function exits.
    ///     - Validate the strength of the `master_input` using `validate_master_strength`. If it fails,
    ///       sets an error message and exits.
    ///     - Initialize the vault with `master_input` and reset the `master_mode_is_setup` flag to false.
    /// 2. Unlock the vault with the provided `master_input`. On failure to unlock, sets an error message
    ///    with the reason and exits with the corresponding error.
    /// 3. Refresh the items in the application to reflect the unlocked state.
    /// 4. Set the current screen to `Screen::Main`.
    /// 5. Clear any existing error messages to indicate successful operation.
    /// 6. Return `Ok(())` if no errors occurred.
    ///
    /// ## Returns
    /// - `Ok(())` on successful unlocking and initialization of the vault.
    /// - `Err` with the propagation of error from validation or unlocking operations.
    ///
    /// ## Errors
    /// This function sets the `error` field with one of the following messages on failure:
    /// - "Please enter and confirm your master key." - if either master input or confirmation input is missing.
    /// - "Master keys do not match." - if the confirmation of the master key does not match the input.
    /// - Error message returned by `validate_master_strength` - if the master key is deemed weak or invalid.
    /// - "Unlock failed: {e}" - if unlocking the vault fails.
    ///
    /// ## Side Effects
    /// - Updates the `error` field in the struct to reflect any issues encountered during execution.
    /// - Modifies the state of `screen`, `master_mode_is_setup`, and `vault` upon successful execution.
    pub fn unlock(&mut self) -> Result<()> {
        if self.master_mode_is_setup {
            if self.master_input.is_empty() || self.master_confirm_input.is_empty() {
                self.error = Some("Please enter and confirm your master key.".into());
                return Ok(());
            }
            if self.master_input != self.master_confirm_input {
                self.error = Some("Master keys do not match.".into());
                return Ok(());
            }
            if let Err(e) = Self::validate_master_strength(&self.master_input) {
                self.error = Some(e.to_string());
                return Ok(());
            }
            self.vault.initialize(&self.master_input)?;
            self.master_mode_is_setup = false;
        }
        self.vault.unlock(&self.master_input).map_err(|e| {
            self.error = Some(format!("Unlock failed: {e}"));
            e
        })?;
        self.refresh_items()?;
        self.screen = Screen::Main;
        self.error = None;
        Ok(())
    }

    /// Refreshes the list of items and updates the filtered items.
    ///
    /// This method performs the following actions:
    /// 1. Updates the `items` field by retrieving the latest list of items from the `vault`.
    /// 2. Applies filtering logic to update the `filtered_items` list.
    /// 3. Ensures that the current selection (`selected`) is within the bounds of the updated `filtered_items` list.
    ///    If the current selection is out of bounds but the `filtered_items` list is not empty,
    ///    it adjusts `selected` to the last valid index.
    ///
    /// # Errors
    /// Returns an error if fetching the list of items from the `vault` fails.
    ///
    /// # Returns
    /// - `Ok(())` if the operation is successful.
    /// - `Err` with the specific error encountered when listing items from the `vault`.
    pub fn refresh_items(&mut self) -> Result<()> {
        self.items = self.vault.list_items()?;
        self.update_filtered_items();
        if self.selected >= self.filtered_items.len() && !self.filtered_items.is_empty() {
            self.selected = self.filtered_items.len().saturating_sub(1);
        }
        Ok(())
    }

    pub fn update_filtered_items(&mut self) {
        let mut filtered = self.items.clone();

        // Apply view mode filter
        if self.view_mode != ViewMode::All {
            filtered.retain(|item| match self.view_mode {
                ViewMode::Passwords => matches!(item.kind, ItemKind::Password),
                ViewMode::Environment => matches!(item.kind, ItemKind::EnvVar),
                ViewMode::Notes => matches!(item.kind, ItemKind::Note),
                ViewMode::All => true,
            });
        }

        // Apply search filter
        if !self.search_query.is_empty() {
            let query_lower = self.search_query.to_lowercase();
            filtered.retain(|item| {
                item.name.to_lowercase().contains(&query_lower) || item.value.to_lowercase().contains(&query_lower)
            });
        }

        // Sort by kind first, then by name
        filtered.sort_by(|a, b| {
            use std::cmp::Ordering;
            match a.kind.as_str().cmp(b.kind.as_str()) {
                Ordering::Equal => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                other => other,
            }
        });

        self.filtered_items = filtered;

        // Adjust selection if needed
        if self.selected >= self.filtered_items.len() && !self.filtered_items.is_empty() {
            self.selected = self.filtered_items.len() - 1;
        }
    }

    pub fn get_selected_item(&self) -> Option<&Item> {
        self.filtered_items.get(self.selected)
    }

    pub fn get_item_counts(&self) -> (usize, usize, usize, usize, usize, usize, usize, usize) {
        let passwords = self
            .items
            .iter()
            .filter(|i| matches!(i.kind, ItemKind::Password))
            .count();
        let env_vars = self.items.iter().filter(|i| matches!(i.kind, ItemKind::EnvVar)).count();
        let notes = self.items.iter().filter(|i| matches!(i.kind, ItemKind::Note)).count();
        let api_keys = self.items.iter().filter(|i| matches!(i.kind, ItemKind::ApiKey)).count();
        let ssh_keys = self.items.iter().filter(|i| matches!(i.kind, ItemKind::SshKey)).count();
        let certificates = self
            .items
            .iter()
            .filter(|i| matches!(i.kind, ItemKind::Certificate))
            .count();
        let databases = self
            .items
            .iter()
            .filter(|i| matches!(i.kind, ItemKind::Database))
            .count();

        (
            self.items.len(),
            passwords,
            env_vars,
            notes,
            api_keys,
            ssh_keys,
            certificates,
            databases,
        )
    }

    /// Adds a new item to the vault with the specified details and updates the UI.
    ///
    /// # Description
    /// This function creates a new item based on the user input, validates it,
    /// and stores it in the vault. If the operation is successful, the UI is updated
    /// to reflect the addition and the input fields are reset. If an error occurs,
    /// appropriate error messages and statuses are set.
    ///
    /// # Fields Used
    /// - `add_kind_idx`: Determines the type of item being added (e.g., `Password`, `EnvVar`, `Note`, etc.).
    /// - `add_name`: The name of the new item, trimmed of whitespace.
    /// - `add_value_textarea`: The content or value of the new item, usually multi-line.
    /// - `vault`: The storage structure which handles item creation.
    /// - `add_value`: Secondary field for item value, cleared after addition.
    /// - `add_value_scroll`: Resets the scroll position of the textarea after addition.
    /// - `screen`: Sets the screen to the main view upon successful addition.
    /// - `error`: Displays error messages for failed operations.
    /// - `status`: Updates the user-visible status of the addition operation.
    ///
    /// # Process
    /// 1. Determines the item type (`kind`) based on `add_kind_idx`:
    ///    - `0` -> Password
    ///    - `1` -> Environment Variable
    ///    - `3` -> API Key
    ///    - `4` -> SSH Key
    ///    - `5` -> Certificate
    ///    - `6` -> Database
    ///    - Default -> Note
    /// 2. Fetches the item's value from the textarea (`add_value_textarea`), joining multiple lines with `\n`.
    /// 3. Creates a `NewItem` structure with the gathered data.
    /// 4. Attempts to add the item using `vault.create_item`.
    /// 5. Handles responses:
    ///    - **Success**: Resets input fields, updates item list, switches to the main screen, and displays a success message.
    ///    - **Failure**: If the name already exists, prompts the user to choose a different name. For other errors, displays a generic error message.
    ///
    /// # Returns
    /// Returns an `Ok(())` on successful completion of the process or propagates an error if any step fails.
    ///
    /// # Errors
    /// - Returns an error if refreshing the items (`refresh_items`) fails.
    /// - Updates the `error` and `status` fields with detailed context if item creation fails.
    ///
    /// # Notes
    /// - Resets both single-line (`add_value`) and multi-line (`add_value_textarea`) value fields upon successful addition.
    /// - Automatically trims leading and trailing whitespace from the item name.
    ///
    /// # Example
    /// ```rust
    /// let mut app = App::default();
    /// app.add_kind_idx = 0; // Adding a Password item
    /// app.add_name = "My Password".into();
    /// app.add_value_textarea = TextArea::from("SuperSecretPassword123");
    /// let result = app.add_item();
    /// assert!(result.is_ok());
    /// assert!(app.error.is_none());
    /// ```
    pub fn add_item(&mut self) -> Result<()> {
        let kind = match self.add_kind_idx {
            0 => ItemKind::Password,
            1 => ItemKind::EnvVar,
            3 => ItemKind::ApiKey,
            4 => ItemKind::SshKey,
            5 => ItemKind::Certificate,
            6 => ItemKind::Database,
            _ => ItemKind::Note,
        };

        // Get the value from textarea instead of add_value
        let value = self.add_value_textarea.lines().join("\n");

        let new_item = NewItem {
            name: self.add_name.trim().to_string(),
            kind,
            value, // Use the textarea content
        };

        match self.vault.create_item(&new_item) {
            Ok(()) => {
                self.add_name.clear();
                self.add_value.clear();
                // Reset the textarea as well
                self.add_value_textarea = TextArea::default();
                self.add_value_scroll = 0;
                self.refresh_items()?;
                self.screen = Screen::Main;
                self.error = Some("Item added.".into());
                self.set_status("Item added successfully.".to_string(), StatusType::Success);
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("already exists") {
                    self.error = Some(format!("Item '{}' already exists.", new_item.name));
                    self.set_status(
                        format!(
                            "Item '{}' already exists. Please choose a different name.",
                            new_item.name
                        ),
                        StatusType::Warning,
                    );
                } else {
                    self.error = Some(format!("Failed to add item: {msg}"));
                    self.set_status(format!("Failed to add item: {msg}"), StatusType::Error);
                }
            }
        }
        Ok(())
    }

    /// Deletes the currently selected item from the vault.
    ///
    /// This function retrieves the currently selected item, deletes it from the vault
    /// using its unique identifier, and then refreshes the list of items to reflect the changes.
    /// If no item is selected, the function does nothing.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Retrieving the selected item fails.
    /// - Deleting the item from the vault fails.
    /// - Refreshing the item list fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut manager = ItemManager::new();
    /// manager.delete_selected()?;
    /// ```
    pub fn delete_selected(&mut self) -> Result<()> {
        if let Some(item) = self.get_selected_item() {
            let item_id = item.id;
            self.vault.delete_item(item_id)?;
            self.refresh_items()?;
        }
        Ok(())
    }

    /// Changes the master key for the application if provided inputs meet the necessary conditions.
    ///
    /// This function performs several validations to ensure the master key change process is secure:
    /// 1. It checks if all required input fields (`ck_current`, `ck_new`, and `ck_confirm`) are filled.
    /// 2. It validates that the new master key (`ck_new`) matches the confirmation key (`ck_confirm`).
    /// 3. It verifies the strength of the new master key using the `validate_master_strength` method.
    ///
    /// If any of these conditions fail, an appropriate error message is stored in the `error` field,
    /// and the process halts without changing the master key.
    ///
    /// Once all validations are passed, the function updates the master key by calling the
    /// `change_master_key` method of the `vault`. After a successful update, it clears all input fields,
    /// resets the error message, and navigates back to the main screen.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the master key has been successfully changed or
    ///   the process ended due to a validation failure without panicking.
    /// * `Err(Error)` - If an error occurs while attempting to change the key in the `vault`.
    ///
    /// # Errors
    ///
    /// - If any of the following conditions occur, an error is stored in the `error` field,
    ///   and the function returns `Ok`:
    ///   - Any of the required fields (`ck_current`, `ck_new`, or `ck_confirm`) are empty.
    ///   - The new master key and confirmation key do not match.
    ///   - The new master key fails the strength validation.
    /// - If the `vault.change_master_key` method returns an error, it will propagate as a `Result::Err`.
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut manager = KeyManager::new();
    /// manager.ck_current = "old_master_key".into();
    /// manager.ck_new = "new_master_key".into();
    ///
    pub fn change_master(&mut self) -> Result<()> {
        if self.ck_current.is_empty() || self.ck_new.is_empty() || self.ck_confirm.is_empty() {
            self.error = Some("Please fill out all fields.".into());
            return Ok(());
        }
        if self.ck_new != self.ck_confirm {
            self.error = Some("New master keys do not match.".into());
            return Ok(());
        }
        if let Err(e) = Self::validate_master_strength(&self.ck_new) {
            self.error = Some(e.to_string());
            return Ok(());
        }
        self.vault.change_master_key(&self.ck_current, &self.ck_new)?;
        self.ck_current.clear();
        self.ck_new.clear();
        self.ck_confirm.clear();
        self.screen = Screen::Main;
        self.error = None;
        Ok(())
    }

    /// Copies the currently selected item to the clipboard.
    ///
    /// This function retrieves the currently selected item using the `get_selected_item` method.
    /// If an item is selected, it initializes the system clipboard, attempts to copy the selected
    /// item's value to the clipboard, and updates the status message to indicate success.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the item is successfully copied to the clipboard or no item is selected.
    /// * `Err(anyhow::Error)` - If there's an error while accessing the clipboard or copying the
    ///   item to the clipboard.
    ///
    /// # Errors
    ///
    /// - Returns an error if accessing the clipboard fails.
    /// - Returns an error if copying the selected item's value to the clipboard fails.
    ///
    /// # Behavior
    ///
    /// - If no item is selected (`get_selected_item` returns `None`), the function does nothing and
    ///   returns `Ok(())`.
    ///
    /// - If an item is selected (`get_selected_item` returns `Some`), it:
    ///   - Initializes a new `arboard::Clipboard` instance.
    ///   - Copies the `value` of the selected item to the clipboard.
    ///   - Sets a status message indicating that the item has been successfully copied to the clipboard.
    ///
    /// # Dependencies
    ///
    /// This function relies on the `arboard` crate for clipboard interactions and the `anyhow` crate
    /// for error handling. It also assumes the existence of the following methods:
    /// - `get_selected_item`: Retrieves the currently selected item, returning an option.
    /// - `set_status`: Updates the application's status message and type.
    ///
    /// # Example
    ///
    /// ```rust
    /// if let Err(e) = app.copy_selected() {
    ///     eprintln!("Error: {}", e);
    /// }
    /// ```
    pub fn copy_selected(&mut self) -> Result<()> {
        if let Some(item) = self.get_selected_item() {
            let mut clipboard = arboard::Clipboard::new().map_err(|e| anyhow!("Failed to access clipboard: {}", e))?;
            clipboard
                .set_text(&item.value)
                .map_err(|e| anyhow!("Failed to copy to clipboard: {}", e))?;
            self.set_status(format!("Copied '{}' to clipboard", item.name), StatusType::Success);
        }
        Ok(())
    }

    pub fn view_selected(&mut self) {
        if let Some(item) = self.get_selected_item() {
            self.view_item = Some(item.clone());
            self.view_show_value = false;
            self.screen = Screen::ViewItem;
        }
    }

    pub const fn toggle_value_visibility(&mut self) {
        self.view_show_value = !self.view_show_value;
    }

    pub fn edit_selected(&mut self) {
        let selected_item = self.get_selected_item().cloned();
        if let Some(item) = selected_item {
            self.edit_item = Some(item.clone());
            self.edit_value.clone_from(&item.value);
            self.screen = Screen::EditItem;
        }
    }

    /// Attempts to save edits made to an item in the vault and updates the user interface accordingly.
    ///
    /// # Behavior
    /// - If the `edit_value` is empty (after trimming), it sets an error message ("Value cannot be empty") and exits early.
    /// - Otherwise, it updates the item in the vault identified by `edit_item.id` with the new `edit_value`.
    /// - After updating the item, it clears the `edit_item` and `edit_value`, refreshes the list of items, and switches
    ///   the application's screen back to the main screen while clearing any previous errors.
    ///
    /// # Errors
    /// - If updating the vault fails, an error is propagated from the `vault.update_item` method.
    /// - If refreshing items fails, an error is propagated from the `refresh_items` method.
    ///
    /// # Returns
    /// - `Ok(())` if the edit is successfully saved or if the `edit_value` is empty.
    /// - `Err` if an error occurs during vault updates or refreshing items.
    ///
    /// # Fields/State
    /// - `self.edit_item`: The item currently being edited. If `None`, the method does nothing.
    /// - `self.edit_value`: The new value to be saved to the item. If empty (once trimmed), the method sets an error message and exits early.
    /// - `self.error`: An optional error message for display purposes. This is set if the `edit_value` is empty or cleared on successful operation.
    /// - `self.vault`: The storage mechanism used to update the item.
    /// - `self.screen`: Controls the application screen flow. Set to `Screen::Main` after a successful edit.
    ///
    pub fn save_edit(&mut self) -> Result<()> {
        if let Some(item) = &self.edit_item {
            if self.edit_value.trim().is_empty() {
                self.error = Some("Value cannot be empty".into());
                return Ok(());
            }

            self.vault.update_item(item.id, &self.edit_value)?;
            self.edit_item = None;
            self.edit_value.clear();
            self.refresh_items()?;
            self.screen = Screen::Main;
            self.error = None;
        }
        Ok(())
    }

    // Password generation methods
    pub fn open_password_generator(&mut self) {
        self.gen_focus = PasswordGenField::Length;
        self.gen_length_str = self.gen_config.length.to_string();
        self.generated_password = None;
        self.error = None;
        self.screen = Screen::GeneratePassword;
    }

    pub fn generate_password(&mut self) {
        if let Ok(length) = self.gen_length_str.parse::<usize>() {
            self.gen_config.length = length.clamp(4, 128);
        } else {
            self.error = Some("Invalid length".into());
            return;
        }

        match self.gen_config.generate() {
            Ok(password) => {
                self.generated_password = Some(password);
                self.error = None;
            }
            Err(e) => {
                self.error = Some(format!("Generation failed: {e}"));
            }
        }
    }

    /// Copies the generated password to the system clipboard.
    ///
    /// This function attempts to access the generated password stored in the `self.generated_password`
    /// field and copies it to the system clipboard using the `arboard` crate. If the operation succeeds,
    /// a success message is set in the `self.error` field. In the event of an error while accessing the
    /// clipboard or copying the text, the function returns a corresponding error.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the password is successfully copied to the clipboard or no password was generated.
    /// * `Err(anyhow::Error)` - If accessing the clipboard or copying the password fails.
    ///
    /// # Errors
    ///
    /// This function may return an error in the following scenarios:
    /// * Failure to access or initialize the system clipboard.
    /// * Failure to copy the generated password to the clipboard.
    ///
    /// # Side Effects
    ///
    /// * If a password is successfully copied to the clipboard, the `self.error` field is set with a success message.
    ///
    /// # Example
    ///
    /// ```rust
    /// if let Err(e) = my_instance.copy_generated_password() {
    ///     eprintln!("Error copying password to clipboard: {}", e);
    /// }
    /// ```
    ///
    /// # Dependencies
    ///
    /// This function makes use of the `arboard` crate for clipboard access and text manipulation.
    pub fn copy_generated_password(&mut self) -> Result<()> {
        if let Some(password) = &self.generated_password {
            let mut clipboard = arboard::Clipboard::new().map_err(|e| anyhow!("Failed to access clipboard: {}", e))?;
            clipboard
                .set_text(password)
                .map_err(|e| anyhow!("Failed to copy to clipboard: {}", e))?;
            self.error = Some("Password copied to clipboard".into());
        }
        Ok(())
    }

    pub fn use_generated_password(&mut self) {
        if let Some(password) = &self.generated_password {
            self.add_value = password.clone();
            self.screen = Screen::AddItem;
            self.add_focus = AddItemField::Name;
        }
    }

    // Import/Export methods
    pub fn open_import_export(&mut self, mode: ImportExportMode) {
        self.ie_mode = mode;
        self.ie_focus = ImportExportField::Path;
        self.ie_path.clear();
        self.ie_format_idx = 0;
        self.error = None;
        self.screen = Screen::ImportExport;
    }

    /// Executes the import or export operation based on the current application state.
    ///
    /// This method performs the following operations:
    /// 1. Validates the provided file path and ensures it is not empty.
    /// 2. Normalizes the file path to handle different path separators and expand the home directory.
    /// 3. Determines the format of import/export (CSV, JSON, or backup).
    /// 4. Executes the import or export operation based on the selected mode (`ImportExportMode`).
    ///
    /// ### Export Mode
    /// - Creates necessary parent directories if they do not exist.
    /// - Exports the current items to the specified file path in the selected format.
    /// - Sets an appropriate success message indicating the number of items exported and the file path.
    ///
    /// ### Import Mode
    /// - Ensures the specified file exists before proceeding.
    /// - Imports items from the file in the specified format.
    /// - Avoids importing duplicate items by checking against existing item names.
    /// - Records the number of imported and skipped items due to duplication or errors.
    /// - Updates the item list after a successful import and presents an appropriate summary message.
    ///
    /// ### Errors
    /// - If the file path is empty, a user-friendly error message is set and the operation is aborted.
    /// - If a directory creation fails during export, an error is returned.
    /// - If certain items cannot be imported due to conflicts or other errors, they are counted as skipped.
    ///
    /// ### Remarks
    /// - Upon completion (successful or not), the application state is updated to the main screen.
    ///
    /// ### Returns
    /// - `Ok(())` if the operation completes successfully (even if some items were skipped).
    /// - `Err` if a file path normalization or file operation fails during the execution.
    ///
    /// ### Examples
    /// ```rust
    /// // Example 1: Exporting items
    /// manager.ie_mode = ImportExportMode::Export;
    /// manager.ie_path = "/path/to/output.json".to_string();
    /// manager.ie_format_idx = 0; // JSON format
    /// manager.execute_import_export().unwrap();
    ///
    /// // Example 2: Importing items
    /// manager.ie_mode = ImportExportMode::Import;
    /// manager.ie_path = "/path/to/input.csv".to_string();
    /// manager.ie_format_idx = 1; // CSV format
    /// manager.execute_import_export().unwrap();
    /// ```
    ///
    /// ### Preconditions
    /// - `self.ie_path` must be set to a valid file path.
    /// - The `self.ie_formats` array must include supported formats ("csv", "backup", "json").
    /// - The `self.items` list is expected to contain the current application items for export/import validation.
    ///
    /// ### Postconditions
    /// - Updates `self.error` with a descriptive message about the operation result.
    /// - Changes the application state screen to `Screen::Main`.
    pub fn execute_import_export(&mut self) -> Result<()> {
        if self.ie_path.trim().is_empty() {
            self.error = Some("Please enter a file path".into());
            return Ok(());
        }

        // Normalize the path to handle different separators and expand the home directory
        let normalized_path = app::App::normalize_path(&self.ie_path)?;
        let path = PathBuf::from(normalized_path);

        let format = match self.ie_formats[self.ie_format_idx] {
            "csv" => ExportFormat::Csv,
            "backup" => ExportFormat::ChamberBackup,
            _ => ExportFormat::Json,
        };

        match self.ie_mode {
            ImportExportMode::Export => {
                // Create parent directories if they don't exist
                if let Some(parent) = path.parent() {
                    if !parent.exists() {
                        std::fs::create_dir_all(parent)
                            .map_err(|e| anyhow!("Failed to create directory {}: {}", parent.display(), e))?;
                    }
                }

                export_items(&self.items, &format, &path)?;
                self.error = Some(format!("Exported {} items to {}", self.items.len(), path.display()));
            }
            ImportExportMode::Import => {
                if !path.exists() {
                    self.error = Some(format!("File does not exist: {}", path.display()));
                    return Ok(());
                }

                let new_items = import_items(&path, &format)?;
                if new_items.is_empty() {
                    self.error = Some("No items found in file".into());
                    return Ok(());
                }

                let existing_names: std::collections::HashSet<String> =
                    self.items.iter().map(|item| item.name.clone()).collect();

                let mut imported_count = 0;
                let mut skipped_count = 0;

                for item in new_items {
                    if existing_names.contains(&item.name) {
                        skipped_count += 1;
                        continue;
                    }

                    match self.vault.create_item(&item) {
                        Ok(()) => imported_count += 1,
                        Err(_) => skipped_count += 1,
                    }
                }

                self.refresh_items()?;
                self.error = Some(format!("Imported {imported_count} items, skipped {skipped_count}"));
            }
        }

        self.screen = Screen::Main;
        Ok(())
    }

    // Helper method to normalize file paths
    fn normalize_path(input_path: &str) -> Result<String> {
        let path_str = input_path.trim();

        // Handle home directory expansion
        let expanded_path = if path_str.strip_prefix('~').is_some() {
            if let Some(home_dir) = dirs::home_dir() {
                let rest = &path_str[1..];
                let rest = if rest.starts_with('/') || rest.starts_with('\\') {
                    &rest[1..]
                } else {
                    rest
                };
                home_dir.join(rest).to_string_lossy().to_string()
            } else {
                return Err(anyhow!("Unable to determine home directory"));
            }
        } else {
            path_str.to_string()
        };

        // Convert forward slashes to native path separators on Windows
        #[cfg(windows)]
        let normalized = expanded_path.replace('/', "\\");

        #[cfg(not(windows))]
        let normalized = expanded_path;

        Ok(normalized)
    }

    pub fn set_status(&mut self, message: String, status_type: StatusType) {
        self.status_message = Some(message);
        self.status_type = status_type;
    }

    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    pub fn is_in_input_mode(&self) -> bool {
        match self.screen {
            Screen::AddItem
            | Screen::EditItem
            | Screen::ChangeMaster
            | Screen::GeneratePassword
            | Screen::ImportExport
            | Screen::Unlock => true,
            Screen::Main if !self.search_query.is_empty() => true, // Search mode
            _ => false,
        }
    }

    /// Pastes content from the clipboard to the add item value field.
    ///
    /// This function retrieves text content from the system clipboard and appends it to
    /// the current `add_value` field. If the clipboard contains text, it will be added
    /// to the existing value. If accessing the clipboard fails, an appropriate status
    /// message is displayed.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the paste operation completes successfully or if there's no text in clipboard.
    /// * `Err(anyhow::Error)` - If accessing the clipboard fails.
    ///
    /// # Errors
    ///
    /// - Returns an error if accessing the clipboard fails.
    /// - Sets a warning status if the clipboard is empty or contains no text.
    ///
    /// # Behavior
    ///
    /// - Retrieves text from the system clipboard using `arboard::Clipboard`.
    /// - Appends the clipboard content to the current `add_value` field.
    /// - Sets a success status message indicating the paste operation completed.
    /// - If clipboard is empty or contains no text, shows a warning message.
    ///
    /// # Example
    ///
    /// ```rust
    /// if let Err(e) = app.paste_to_add_value() {
    ///     eprintln!("Error pasting from clipboard: {}", e);
    /// }
    /// ```
    pub fn paste_to_add_value(&mut self) -> Result<()> {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Ok(text) = clipboard.get_text() {
                // Clear existing content and insert new text
                self.add_value_textarea.select_all();
                self.add_value_textarea.cut();
                self.add_value_textarea.insert_str(text);
                self.set_status("Pasted from clipboard".to_string(), StatusType::Success);
                Ok(())
            } else {
                self.set_status("No text in clipboard".to_string(), StatusType::Warning);
                Ok(())
            }
        } else {
            self.set_status("Failed to access clipboard".to_string(), StatusType::Error);
            Ok(())
        }
    }
}
