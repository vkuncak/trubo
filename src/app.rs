use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

#[cfg(all(unix, not(target_os = "macos")))]
use std::process::Stdio;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::{
    editor::Editor,
    project::{ProjectEntry, list_directory},
};

pub const MENUS: [Menu; 5] = [
    Menu {
        title: "File",
        items: &[
            MenuItem::action("Open", "F3", MenuAction::Open),
            MenuItem::action("Save", "F2", MenuAction::Save),
            MenuItem::separator(),
            MenuItem::action("Quit", "Ctrl+Q", MenuAction::Quit),
        ],
    },
    Menu {
        title: "Edit",
        items: &[
            MenuItem::action("Copy", "Ctrl+C", MenuAction::Copy),
            MenuItem::action("Cut", "Ctrl+X", MenuAction::Cut),
            MenuItem::action("Paste", "Ctrl+V", MenuAction::Paste),
            MenuItem::separator(),
            MenuItem::action("Delete line", "Alt+X", MenuAction::DeleteLine),
            MenuItem::action("Duplicate line", "Alt+U", MenuAction::DuplicateLine),
        ],
    },
    Menu {
        title: "Window",
        items: &[
            MenuItem::action("Files pane", "", MenuAction::FocusBrowser),
            MenuItem::action("Editor pane", "", MenuAction::FocusEditor),
            MenuItem::separator(),
            MenuItem::action("Next pane", "F4", MenuAction::ToggleFocus),
        ],
    },
    Menu {
        title: "Run",
        items: &[
            MenuItem::action("Run", "F5", MenuAction::CargoRun),
            MenuItem::action("Build", "F9", MenuAction::CargoBuild),
        ],
    },
    Menu {
        title: "Help",
        items: &[
            MenuItem::action("Help", "F1", MenuAction::Help),
            MenuItem::action("About", "", MenuAction::About),
        ],
    },
];

#[derive(Debug, Clone, Copy)]
pub struct Menu {
    pub title: &'static str,
    pub items: &'static [MenuItem],
}

#[derive(Debug, Clone, Copy)]
pub struct MenuItem {
    pub label: &'static str,
    pub shortcut: &'static str,
    pub action: MenuAction,
    pub separator: bool,
}

impl MenuItem {
    pub const fn action(label: &'static str, shortcut: &'static str, action: MenuAction) -> Self {
        Self {
            label,
            shortcut,
            action,
            separator: false,
        }
    }

    pub const fn separator() -> Self {
        Self {
            label: "",
            shortcut: "",
            action: MenuAction::None,
            separator: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    None,
    Open,
    Save,
    Quit,
    Copy,
    Cut,
    Paste,
    DeleteLine,
    DuplicateLine,
    CargoRun,
    CargoBuild,
    ToggleFocus,
    FocusBrowser,
    FocusEditor,
    Help,
    About,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MenuGeometry {
    pub bar_items: [Rect; MENUS.len()],
    pub dropdown: Option<Rect>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Browser,
    Editor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialog {
    About,
    ConfirmExit { dirty: bool, selection: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragTarget {
    BrowserDivider,
    EditorSelection,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Geometry {
    pub root: Rect,
    pub menu_area: Rect,
    pub menu: MenuGeometry,
    pub desktop_inner: Rect,
    pub browser_area: Rect,
    pub browser_inner: Rect,
    pub editor_area: Rect,
    pub editor_inner: Rect,
}

pub const MIN_BROWSER_PANE_WIDTH: u16 = 18;
pub const MIN_EDITOR_PANE_WIDTH: u16 = 24;
const BROWSER_PREVIEW_DELAY: Duration = Duration::from_millis(200);

#[derive(Debug)]
pub struct App {
    pub browser_dir: PathBuf,
    pub entries: Vec<ProjectEntry>,
    pub selected_entry: usize,
    pub editor: Editor,
    pub focus: Focus,
    pub menu_open: bool,
    pub active_menu: usize,
    pub active_menu_item: usize,
    pub help_open: bool,
    pub dialog: Option<Dialog>,
    pub selection_mode: bool,
    pub status: String,
    pub browser_pane_width: u16,
    pub geometry: Geometry,
    full_redraw_requested: bool,
    browser_preview_due_at: Option<Instant>,
    drag_target: Option<DragTarget>,
}

impl App {
    pub fn new(start_path: PathBuf) -> Self {
        let browser_dir = if start_path.is_dir() {
            start_path
        } else {
            start_path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."))
        };

        Self {
            browser_dir,
            entries: Vec::new(),
            selected_entry: 0,
            editor: Editor::scratch(),
            focus: Focus::Browser,
            menu_open: false,
            active_menu: 0,
            active_menu_item: first_selectable_item(0),
            help_open: false,
            dialog: None,
            selection_mode: false,
            status: "Ready".to_string(),
            browser_pane_width: 30,
            geometry: Geometry::default(),
            full_redraw_requested: false,
            browser_preview_due_at: None,
            drag_target: None,
        }
    }

    pub fn refresh_browser(&mut self) {
        self.entries = list_directory(&self.browser_dir);
        if self.selected_entry >= self.entries.len() {
            self.selected_entry = self.entries.len().saturating_sub(1);
        }
        self.schedule_browser_preview();
    }

    pub fn toggle_focus(&mut self) {
        self.close_menu();
        let focus = match self.focus {
            Focus::Browser => Focus::Editor,
            Focus::Editor => Focus::Browser,
        };
        self.assign_focus(focus);
        if self.focus == Focus::Browser {
            self.schedule_browser_preview();
        }
        self.status = format!("Focus: {}", self.focus_name());
    }

    pub fn focus_browser(&mut self) {
        self.set_focus(Focus::Browser);
    }

    pub fn focus_editor(&mut self) {
        self.set_focus(Focus::Editor);
    }

    fn set_focus(&mut self, focus: Focus) {
        self.close_menu();
        self.assign_focus(focus);
        if self.focus == Focus::Browser {
            self.schedule_browser_preview();
        }
        self.status = format!("Focus: {}", self.focus_name());
    }

    fn assign_focus(&mut self, focus: Focus) {
        self.focus = focus;
    }

    pub fn request_full_redraw(&mut self) {
        self.full_redraw_requested = true;
    }

    pub fn take_full_redraw_request(&mut self) -> bool {
        std::mem::take(&mut self.full_redraw_requested)
    }

    pub fn tick_browser_preview(&mut self) {
        let Some(due) = self.browser_preview_due_at else {
            return;
        };

        if Instant::now() < due
            || self.focus != Focus::Browser
            || self.menu_open
            || self.help_open
            || self.dialog.is_some()
            || self.editor.is_dirty()
        {
            return;
        }

        self.browser_preview_due_at = None;

        let Some(entry) = self.entries.get(self.selected_entry) else {
            return;
        };

        if entry.is_directory() {
            return;
        }

        if self.editor.path() == Some(entry.path.as_path()) {
            return;
        }

        let path = entry.path.clone();
        match Editor::open(&path) {
            Ok(editor) => {
                self.editor = editor;
                self.status = format!("Previewed {}", path.display());
            }
            Err(error) => {
                self.status = format!("Preview failed: {error}");
            }
        }
    }

    pub fn focus_name(&self) -> &'static str {
        match self.focus {
            Focus::Browser => "Files",
            Focus::Editor => "Edit",
        }
    }

    pub fn current_file_label(&self) -> String {
        self.editor
            .path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "Untitled".to_string())
    }

    pub fn browser_label(&self) -> String {
        self.browser_dir.display().to_string()
    }

    pub fn open_selected_file(&mut self) {
        self.close_menu();
        let Some(entry) = self.entries.get(self.selected_entry) else {
            self.status = "No files in this directory".to_string();
            return;
        };

        if entry.is_directory() {
            self.navigate_to_dir(entry.path.clone());
            return;
        }

        self.open_path(entry.path.clone());
    }

    pub fn open_path(&mut self, path: PathBuf) {
        match Editor::open(&path) {
            Ok(editor) => {
                self.editor = editor;
                self.assign_focus(Focus::Editor);
                self.status = format!("Opened {}", path.display());
            }
            Err(error) => self.status = format!("Open failed: {error}"),
        }
    }

    fn navigate_to_dir(&mut self, path: PathBuf) {
        let path = path.canonicalize().unwrap_or(path);
        self.browser_dir = path;
        self.selected_entry = 0;
        self.refresh_browser();
        self.assign_focus(Focus::Browser);
        self.status = format!("Browsing {}", self.browser_label());
    }

    pub fn save_current(&mut self) {
        self.close_menu();
        match self.editor.save() {
            Ok(()) => {
                self.status = format!("Saved {}", self.current_file_label());
            }
            Err(error) => self.status = format!("Save failed: {error}"),
        }
    }

    pub fn toggle_selection_mode(&mut self) {
        self.selection_mode = !self.selection_mode;
        self.status = if self.selection_mode {
            "Selection mode ON".to_string()
        } else {
            "Selection mode OFF".to_string()
        };
    }

    pub fn run_current_target(&mut self) {
        self.close_menu();
        if self.editor.is_dirty() {
            self.save_current();
        }

        let Some(path) = self.editor.path() else {
            self.status = "Run needs a saved file".to_string();
            return;
        };

        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or_default();
        let cwd = path.parent().unwrap_or(self.browser_dir.as_path());

        let (command, description) = match extension {
            "rs" => ("cargo run".to_string(), "cargo run".to_string()),
            "scala" => (
                format!("scala {}", shell_quote(path)),
                format!("scala {}", path.display()),
            ),
            "lean" => (
                format!("lean {}", shell_quote(path)),
                format!("lean {}", path.display()),
            ),
            _ => {
                self.status = format!("Run is not configured for .{extension}");
                return;
            }
        };

        match launch_in_interactive_terminal(cwd, &command) {
            Ok(()) => {
                self.request_full_redraw();
                self.status = format!("Launched {description} in external terminal");
            }
            Err(error) => {
                self.status = format!("Run launch failed: {error}");
            }
        }
    }

    pub fn build_current_target(&mut self) {
        self.close_menu();
        if self.editor.is_dirty() {
            self.save_current();
        }

        let Some(path) = self.editor.path() else {
            self.status = "Build needs a saved file".to_string();
            return;
        };

        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or_default();
        let cwd = path.parent().unwrap_or(self.browser_dir.as_path());

        let (command, description) = match extension {
            "rs" => ("cargo build".to_string(), "cargo build".to_string()),
            "scala" => (
                format!("scalac {}", shell_quote(path)),
                format!("scalac {}", path.display()),
            ),
            "lean" => (
                format!("lean {}", shell_quote(path)),
                format!("lean {}", path.display()),
            ),
            _ => {
                self.status = format!("Build is not configured for .{extension}");
                return;
            }
        };

        match launch_in_interactive_terminal(cwd, &command) {
            Ok(()) => {
                self.request_full_redraw();
                self.status = format!("Launched {description} in external terminal");
            }
            Err(error) => {
                self.status = format!("Build launch failed: {error}");
            }
        }
    }

    pub fn handle_active_key(&mut self, key: KeyEvent) {
        if self.dialog.take().is_some() {
            return;
        }

        if self.menu_open {
            return;
        }

        match self.focus {
            Focus::Browser => self.handle_browser_key(key),
            Focus::Editor => self.handle_editor_key(key),
        }
    }

    pub fn handle_dialog_key(&mut self, _key: KeyEvent) -> Action {
        let Some(dialog) = self.dialog else {
            return Action::None;
        };

        match dialog {
            Dialog::About => {
                self.dialog = None;
                Action::None
            }
            Dialog::ConfirmExit { .. } => match _key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => Action::Quit,
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.dialog = None;
                    self.status = "Exit cancelled".to_string();
                    Action::None
                }
                _ => Action::None,
            },
        }
    }

    pub fn request_quit(&mut self) -> Action {
        self.close_menu();
        self.help_open = false;

        let dirty = self.editor.is_dirty();
        let selection = self.editor.has_selection();

        if dirty || selection {
            self.dialog = Some(Dialog::ConfirmExit { dirty, selection });
            self.status = "Confirm exit: unsaved edits or active selection".to_string();
            return Action::None;
        }

        Action::Quit
    }

    pub fn open_menu(&mut self) {
        self.menu_open = true;
        self.active_menu = self.active_menu.min(MENUS.len() - 1);
        self.active_menu_item = first_selectable_item(self.active_menu);
        self.status = format!("Menu: {}", MENUS[self.active_menu].title);
    }

    pub fn close_menu(&mut self) {
        self.menu_open = false;
    }

    pub fn toggle_menu(&mut self) {
        if self.menu_open {
            self.close_menu();
        } else {
            self.open_menu();
        }
    }

    pub fn handle_menu_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc | KeyCode::F(10) => self.close_menu(),
            KeyCode::Left => self.select_previous_menu(),
            KeyCode::Right => self.select_next_menu(),
            KeyCode::Up => self.select_previous_menu_item(),
            KeyCode::Down => self.select_next_menu_item(),
            KeyCode::Home => self.select_menu(0),
            KeyCode::End => self.select_menu(MENUS.len().saturating_sub(1)),
            KeyCode::Enter => return self.activate_selected_menu_item(),
            KeyCode::Char(character) => {
                if let Some(menu_index) = menu_index_for_hotkey(character) {
                    if menu_index == self.active_menu {
                        return self.activate_selected_menu_item();
                    }
                    self.select_menu(menu_index);
                } else if let Some(item_index) = item_index_for_hotkey(self.active_menu, character)
                {
                    self.active_menu_item = item_index;
                    return self.activate_selected_menu_item();
                }
            }
            _ => {}
        }

        Action::None
    }

    fn select_menu(&mut self, index: usize) {
        self.active_menu = index.min(MENUS.len().saturating_sub(1));
        self.active_menu_item = first_selectable_item(self.active_menu);
        self.status = format!("Menu: {}", MENUS[self.active_menu].title);
    }

    fn select_previous_menu(&mut self) {
        let index = if self.active_menu == 0 {
            MENUS.len() - 1
        } else {
            self.active_menu - 1
        };
        self.select_menu(index);
    }

    fn select_next_menu(&mut self) {
        self.select_menu((self.active_menu + 1) % MENUS.len());
    }

    fn select_previous_menu_item(&mut self) {
        let items = MENUS[self.active_menu].items;
        let mut index = self.active_menu_item;
        for _ in 0..items.len() {
            index = if index == 0 { items.len() - 1 } else { index - 1 };
            if !items[index].separator {
                self.active_menu_item = index;
                break;
            }
        }
    }

    fn select_next_menu_item(&mut self) {
        let items = MENUS[self.active_menu].items;
        let mut index = self.active_menu_item;
        for _ in 0..items.len() {
            index = (index + 1) % items.len();
            if !items[index].separator {
                self.active_menu_item = index;
                break;
            }
        }
    }

    fn activate_selected_menu_item(&mut self) -> Action {
        let item = MENUS[self.active_menu].items[self.active_menu_item];
        self.perform_menu_action(item.action)
    }

    fn perform_menu_action(&mut self, action: MenuAction) -> Action {
        self.close_menu();
        match action {
            MenuAction::None => {}
            MenuAction::Open => self.open_selected_file(),
            MenuAction::Save => self.save_current(),
            MenuAction::Quit => return self.request_quit(),
            MenuAction::Copy => self.copy_selection(),
            MenuAction::Cut => self.cut_selection(),
            MenuAction::Paste => self.paste_from_clipboard(),
            MenuAction::DeleteLine => self.editor.delete_line(),
            MenuAction::DuplicateLine => self.editor.duplicate_line(),
            MenuAction::CargoRun => self.run_current_target(),
            MenuAction::CargoBuild => self.build_current_target(),
            MenuAction::ToggleFocus => self.toggle_focus(),
            MenuAction::FocusBrowser => self.set_focus(Focus::Browser),
            MenuAction::FocusEditor => self.set_focus(Focus::Editor),
            MenuAction::Help => self.help_open = true,
            MenuAction::About => self.dialog = Some(Dialog::About),
        }

        Action::None
    }

    pub fn copy_selection(&mut self) {
        let Some(text) = self.editor.selected_text() else {
            self.status = "No editor selection to copy".to_string();
            return;
        };

        match crate::clipboard::set_text(&text) {
            Ok(()) => self.status = format!("Copied {} characters", text.chars().count()),
            Err(error) => self.status = format!("Copy failed: {error}"),
        }
    }

    pub fn cut_selection(&mut self) {
        let Some(text) = self.editor.selected_text() else {
            self.status = "No editor selection to cut".to_string();
            return;
        };

        match crate::clipboard::set_text(&text) {
            Ok(()) => {
                self.editor.cut_selection();
                self.status = format!("Cut {} characters", text.chars().count());
            }
            Err(error) => self.status = format!("Cut failed: {error}"),
        }
    }

    pub fn paste_from_clipboard(&mut self) {
        match crate::clipboard::get_text() {
            Ok(text) => self.paste_text(&text),
            Err(error) => self.status = format!("Paste failed: {error}"),
        }
    }

    pub fn paste_text(&mut self, text: &str) {
        if text.is_empty() {
            self.status = "Clipboard is empty".to_string();
            return;
        }

        self.dialog = None;
        self.help_open = false;
        self.assign_focus(Focus::Editor);
        self.editor.insert_text(text);
        self.status = format!("Pasted {} characters", text.chars().count());
    }

    pub fn undo_last_edit(&mut self) {
        self.dialog = None;
        self.help_open = false;
        self.assign_focus(Focus::Editor);

        if self.editor.undo() {
            self.status = "Undo applied".to_string();
        } else {
            self.status = "Nothing to undo".to_string();
        }
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) -> Action {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.handle_mouse_down(mouse.column, mouse.row)
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                self.handle_mouse_drag(mouse.column, mouse.row);
                Action::None
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.drag_target = None;
                Action::None
            }
            MouseEventKind::ScrollUp => {
                self.handle_mouse_scroll(mouse.column, mouse.row, -3);
                Action::None
            }
            MouseEventKind::ScrollDown => {
                self.handle_mouse_scroll(mouse.column, mouse.row, 3);
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_mouse_down(&mut self, column: u16, row: u16) -> Action {
        self.drag_target = None;

        if self.help_open {
            self.help_open = false;
            return Action::None;
        }

        if self.dialog.take().is_some() {
            return Action::None;
        }

        if contains(self.geometry.menu_area, column, row) {
            self.open_menu();
            if let Some(menu_index) = menu_bar_index_at(&self.geometry.menu, column, row) {
                self.select_menu(menu_index);
            }
            return Action::None;
        }

        if self.menu_open {
            if let Some((menu_index, item_index)) =
                menu_dropdown_item_at(&self.geometry.menu, self.active_menu, column, row)
            {
                self.select_menu(menu_index);
                self.active_menu_item = item_index;
                return self.activate_selected_menu_item();
            }
            self.close_menu();
        }

        if self.is_browser_divider(column, row) {
            self.drag_target = Some(DragTarget::BrowserDivider);
            self.resize_browser_pane(column);
            return Action::None;
        }

        if contains(self.geometry.browser_inner, column, row) {
            self.assign_focus(Focus::Browser);
            self.select_entry_at(row);
        } else if contains(self.geometry.editor_inner, column, row) {
            self.assign_focus(Focus::Editor);
            self.place_cursor_at(column, row, false);
            self.drag_target = Some(DragTarget::EditorSelection);
        } else if contains(self.geometry.browser_area, column, row) {
            self.assign_focus(Focus::Browser);
            self.status = "Focus: Files".to_string();
        } else if contains(self.geometry.editor_area, column, row) {
            self.assign_focus(Focus::Editor);
            self.status = "Focus: Edit".to_string();
        }

        Action::None
    }

    fn handle_mouse_drag(&mut self, column: u16, row: u16) {
        match self.drag_target {
            Some(DragTarget::BrowserDivider) => self.resize_browser_pane(column),
            Some(DragTarget::EditorSelection) => self.place_cursor_at(column, row, true),
            None => {}
        }
    }

    fn handle_mouse_scroll(&mut self, column: u16, row: u16, amount: isize) {
        if self.dialog.is_some() || self.help_open {
            return;
        }

        if contains(self.geometry.browser_inner, column, row) {
            self.assign_focus(Focus::Browser);
            if amount < 0 {
                for _ in 0..amount.unsigned_abs() {
                    self.select_previous_entry();
                }
            } else {
                for _ in 0..amount as usize {
                    self.select_next_entry();
                }
            }
        } else if contains(self.geometry.editor_inner, column, row) {
            self.assign_focus(Focus::Editor);
            if amount < 0 {
                self.editor.page_up(amount.unsigned_abs());
            } else {
                self.editor.page_down(amount as usize);
            }
        }
    }

    fn handle_browser_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => self.select_previous_entry(),
            KeyCode::Down => self.select_next_entry(),
            KeyCode::Home => self.set_selected_entry(0),
            KeyCode::End => {
                self.set_selected_entry(self.entries.len().saturating_sub(1));
            }
            KeyCode::Enter => self.open_selected_file(),
            KeyCode::Backspace => {
                if let Some(parent) = self.browser_dir.parent() {
                    self.navigate_to_dir(parent.to_path_buf());
                }
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.refresh_browser();
                self.status = "Directory refreshed".to_string();
            }
            _ => {}
        }
    }

    fn handle_editor_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::ALT) {
            match key.code {
                KeyCode::Char('x') | KeyCode::Char('X') => self.editor.delete_line(),
                KeyCode::Char('u') | KeyCode::Char('U') => self.editor.duplicate_line(),
                _ => {}
            }
            return;
        }

        let selecting = key.modifiers.contains(KeyModifiers::SHIFT) || self.selection_mode;
        match key.code {
            KeyCode::Left if selecting => self.editor.extend_left(),
            KeyCode::Left => self.editor.move_left(),
            KeyCode::Right if selecting => self.editor.extend_right(),
            KeyCode::Right => self.editor.move_right(),
            KeyCode::Up if selecting => self.editor.extend_up(),
            KeyCode::Up => self.editor.move_up(),
            KeyCode::Down if selecting => self.editor.extend_down(),
            KeyCode::Down => self.editor.move_down(),
            KeyCode::Home if selecting => self.editor.extend_home(),
            KeyCode::Home => self.editor.home(),
            KeyCode::End if selecting => self.editor.extend_end(),
            KeyCode::End => self.editor.end(),
            KeyCode::PageUp if selecting => self.editor.extend_page_up(12),
            KeyCode::PageUp => self.editor.page_up(12),
            KeyCode::PageDown if selecting => self.editor.extend_page_down(12),
            KeyCode::PageDown => self.editor.page_down(12),
            KeyCode::Backspace => self.editor.backspace(),
            KeyCode::Delete => self.editor.delete(),
            KeyCode::Enter => self.editor.insert_newline(),
            KeyCode::Char(character) => {
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT {
                    self.editor.insert_char(character);
                }
            }
            _ => {}
        }
    }

    fn select_previous_entry(&mut self) {
        self.set_selected_entry(self.selected_entry.saturating_sub(1));
    }

    fn select_next_entry(&mut self) {
        if !self.entries.is_empty() {
            self.set_selected_entry((self.selected_entry + 1).min(self.entries.len() - 1));
        }
    }

    fn set_selected_entry(&mut self, index: usize) {
        let clamped = index.min(self.entries.len().saturating_sub(1));
        if clamped != self.selected_entry {
            self.selected_entry = clamped;
            self.schedule_browser_preview();
        }
    }

    fn schedule_browser_preview(&mut self) {
        self.browser_preview_due_at = Some(Instant::now() + BROWSER_PREVIEW_DELAY);
    }

    fn select_entry_at(&mut self, row: u16) {
        if self.entries.is_empty() {
            return;
        }

        let visible_rows = self.geometry.browser_inner.height as usize;
        let start = self
            .selected_entry
            .saturating_sub(visible_rows.saturating_sub(1));
        let clicked = start + row.saturating_sub(self.geometry.browser_inner.y) as usize;

        if clicked < self.entries.len() {
            let label = self.entries[clicked].label.clone();
            self.set_selected_entry(clicked);
            self.status = format!("Selected {label}");
            self.open_selected_file();
        }
    }

    fn place_cursor_at(&mut self, column: u16, row: u16, selecting: bool) {
        let inner = self.geometry.editor_inner;
        let line_number_width = self.editor_line_number_width();
        let text_x = inner.x.saturating_add(line_number_width);
        let text_cols = inner.width.saturating_sub(line_number_width + 1).max(1) as usize;
        let target_screen_row = row.saturating_sub(inner.y) as usize;
        let mut remaining = target_screen_row;
        let mut file_row = self.editor.row_offset();
        let mut segment_offset = self.editor.row_segment_offset();

        while let Some(line) = self.editor.lines().get(file_row) {
            let line_len = line.chars().count();
            let wrapped = line_len.max(1).div_ceil(text_cols);
            let visible_wrapped = wrapped.saturating_sub(segment_offset);
            if remaining < visible_wrapped {
                break;
            }
            remaining -= visible_wrapped;
            segment_offset = 0;
            file_row += 1;
        }

        if self.editor.lines().is_empty() {
            file_row = 0;
        } else {
            file_row = file_row.min(self.editor.lines().len() - 1);
        }

        let segment = remaining.saturating_add(segment_offset);
        let segment_col = column.saturating_sub(text_x) as usize;
        let mut file_col = segment
            .saturating_mul(text_cols)
            .saturating_add(segment_col);
        if let Some(line) = self.editor.lines().get(file_row) {
            file_col = file_col.min(line.chars().count());
        }

        if selecting {
            self.editor.select_to(file_row, file_col);
        } else {
            self.editor.set_cursor(file_row, file_col);
        }
    }

    pub fn editor_line_number_width(&self) -> u16 {
        (self.editor.lines().len().max(1).to_string().len().max(3) + 1) as u16
    }

    fn is_browser_divider(&self, column: u16, row: u16) -> bool {
        if !contains_y(self.geometry.browser_area, row) {
            return false;
        }

        let right_edge = self
            .geometry
            .browser_area
            .x
            .saturating_add(self.geometry.browser_area.width.saturating_sub(1));

        column == right_edge || column == right_edge.saturating_add(1)
    }

    fn resize_browser_pane(&mut self, column: u16) {
        let desktop = self.geometry.desktop_inner;
        if desktop.width == 0 {
            return;
        }

        let max_width = desktop.width.saturating_sub(MIN_EDITOR_PANE_WIDTH).max(1);
        let min_width = MIN_BROWSER_PANE_WIDTH.min(max_width);
        let width = column
            .saturating_sub(desktop.x)
            .saturating_add(1)
            .clamp(min_width, max_width);

        self.browser_pane_width = width;
        self.status = format!("Files pane: {width} columns");
    }
}

pub fn read_to_string(path: &Path) -> io::Result<String> {
    let bytes = fs::read(path)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn contains(area: Rect, column: u16, row: u16) -> bool {
    contains_x(area, column) && contains_y(area, row)
}

fn contains_x(area: Rect, column: u16) -> bool {
    column >= area.x && column < area.x.saturating_add(area.width)
}

fn contains_y(area: Rect, row: u16) -> bool {
    row >= area.y && row < area.y.saturating_add(area.height)
}

fn first_selectable_item(menu_index: usize) -> usize {
    MENUS[menu_index]
        .items
        .iter()
        .position(|item| !item.separator)
        .unwrap_or(0)
}

fn shell_quote(path: &Path) -> String {
    let value = path.to_string_lossy();
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(target_os = "macos")]
fn launch_in_interactive_terminal(cwd: &Path, command: &str) -> io::Result<()> {
    let full_command = format!("cd {} && {}", shell_quote(cwd), command);
    let status = Command::new("osascript")
        .arg("-e")
        .arg("on run argv")
        .arg("-e")
        .arg("set commandText to item 1 of argv")
        .arg("-e")
        .arg("tell application \"Terminal\"")
        .arg("-e")
        .arg("activate")
        .arg("-e")
        .arg("do script commandText")
        .arg("-e")
        .arg("end tell")
        .arg("-e")
        .arg("end run")
        .arg(full_command)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "osascript exited with status {status}"
        )))
    }
}

#[cfg(target_os = "windows")]
fn launch_in_interactive_terminal(cwd: &Path, command: &str) -> io::Result<()> {
    let status = Command::new("cmd")
        .args(["/C", "start", "cmd", "/K", command])
        .current_dir(cwd)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "cmd exited with status {status}"
        )))
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn launch_in_interactive_terminal(cwd: &Path, command: &str) -> io::Result<()> {
    let full_command = format!("cd {} && {}", shell_quote(cwd), command);
    let launchers: [(&str, &[&str]); 3] = [
        ("x-terminal-emulator", &["-e", "sh", "-lc"]),
        ("gnome-terminal", &["--", "sh", "-lc"]),
        ("xterm", &["-e", "sh", "-lc"]),
    ];

    let mut last_error = io::Error::other("no terminal launcher configured");
    for (program, args) in launchers {
        match Command::new(program)
            .args(args)
            .arg(&full_command)
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(_) => return Ok(()),
            Err(error) => last_error = error,
        }
    }

    Err(last_error)
}

fn menu_index_for_hotkey(character: char) -> Option<usize> {
    let needle = character.to_ascii_lowercase();
    MENUS.iter().position(|menu| {
        menu.title
            .chars()
            .next()
            .is_some_and(|candidate| candidate.to_ascii_lowercase() == needle)
    })
}

fn item_index_for_hotkey(menu_index: usize, character: char) -> Option<usize> {
    let needle = character.to_ascii_lowercase();
    MENUS[menu_index].items.iter().position(|item| {
        !item.separator
            && item
                .label
                .chars()
                .next()
                .is_some_and(|candidate| candidate.to_ascii_lowercase() == needle)
    })
}

fn menu_bar_index_at(menu: &MenuGeometry, column: u16, row: u16) -> Option<usize> {
    menu.bar_items
        .iter()
        .position(|area| contains(*area, column, row))
}

fn menu_dropdown_item_at(
    menu: &MenuGeometry,
    active_menu: usize,
    column: u16,
    row: u16,
) -> Option<(usize, usize)> {
    let area = menu.dropdown?;
    if !contains(area, column, row) {
        return None;
    }

    let inner_y = area.y.saturating_add(1);
    let inner_bottom = area.y.saturating_add(area.height).saturating_sub(1);
    if row < inner_y || row >= inner_bottom {
        return None;
    }

    let item_index = row.saturating_sub(inner_y) as usize;
    let item = MENUS[active_menu].items.get(item_index)?;
    (!item.separator).then_some((active_menu, item_index))
}
