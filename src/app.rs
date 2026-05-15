use std::{
    collections::BTreeSet,
    env, fs, io,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

#[cfg(all(unix, not(target_os = "macos")))]
use std::process::Stdio;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use regex::Regex;
use ratatui::layout::Rect;

use crate::{
    editor::{Editor, SaveOutcome},
    file_types::{ToolInvocation, detect_file_type},
    project::{ProjectEntry, ProjectEntryKind, directory_subtree_lines, list_directory},
};

pub const MENUS: [Menu; 6] = [
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
            MenuItem::action("Undo", "Ctrl+Z", MenuAction::Undo),
            MenuItem::action("Redo", "Ctrl+Y", MenuAction::Redo),
            MenuItem::separator(),
            MenuItem::action("Copy", "Ctrl+C", MenuAction::Copy),
            MenuItem::action("Cut", "Ctrl+X", MenuAction::Cut),
            MenuItem::action("Paste", "Ctrl+V", MenuAction::Paste),
            MenuItem::action("Search", "Ctrl+F", MenuAction::Search),
            MenuItem::separator(),
            MenuItem::action("Delete line", "Ctrl+K", MenuAction::DeleteLine),
        ],
    },
    Menu {
        title: "Window",
        items: &[
            MenuItem::action("Files pane", "", MenuAction::FocusBrowser),
            MenuItem::action("Editor pane", "", MenuAction::FocusEditor),
            MenuItem::action("Toggle Dual Pane", "`", MenuAction::ToggleDualPane),
            MenuItem::action("Editor Only", "Ctrl+B", MenuAction::ToggleEditorOnly),
            MenuItem::separator(),
            MenuItem::action("Next pane", "F4", MenuAction::ToggleFocus),
        ],
    },
    Menu {
        title: "Files",
        items: &[
            MenuItem::action("Copy", "F5", MenuAction::CopyEntry),
            MenuItem::action("Move", "F6", MenuAction::MoveEntry),
            MenuItem::action("New directory", "F7", MenuAction::NewDirectory),
            MenuItem::action("Delete", "F8", MenuAction::DeleteEntry),
        ],
    },
    Menu {
        title: "Run",
        items: &[
            MenuItem::action("Run", "Ctrl+R", MenuAction::CargoRun),
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
    Undo,
    Redo,
    Copy,
    Cut,
    Paste,
    Search,
    CopyEntry,
    MoveEntry,
    NewDirectory,
    DeleteEntry,
    DeleteLine,
    CargoRun,
    CargoBuild,
    ToggleFocus,
    ToggleDualPane,
    ToggleEditorOnly,
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
    BrowserPrimary,
    BrowserSecondary,
    Editor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialog {
    About,
    SaveFile,
    NewDirectory,
    OpenFilePath,
    RegexSearch,
    BrowserIncrementalSearch,
    BrowserSelectionPattern,
    FileOperationName,
    ConfirmFileOperation,
    ResolveFileConflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PendingUnsavedAction {
    Quit,
    Focus(Focus),
    OpenPath { path: PathBuf, browser_index: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileOperationKind {
    Copy,
    Move,
    Delete,
}

#[derive(Debug, Clone)]
struct PendingFileOperation {
    kind: FileOperationKind,
    sources: Vec<PathBuf>,
    target_dir: Option<PathBuf>,
    target_name: Option<PathBuf>,
    browser_index: usize,
    current_index: usize,
    completed_targets: Vec<PathBuf>,
    overwritten_count: usize,
    skipped_count: usize,
    rename_from_conflict: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileConflictResolution {
    Overwrite,
    Skip,
    Rename,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileOperationStep {
    Continue,
    Done,
    Conflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BrowserSelectionPatternMode {
    Add,
    Remove,
}

#[derive(Debug, Clone, Default)]
struct TextInputState {
    text: String,
    cursor: usize,
}

impl TextInputState {
    fn as_str(&self) -> &str {
        self.text.as_str()
    }

    fn cursor(&self) -> usize {
        self.cursor
    }

    fn set_text(&mut self, text: String) {
        self.text = text;
        self.cursor = self.text.chars().count();
    }

    fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    fn insert_char(&mut self, character: char) {
        let byte_index = char_to_byte_index(&self.text, self.cursor);
        self.text.insert(byte_index, character);
        self.cursor += 1;
    }

    fn delete_left(&mut self) {
        if self.cursor == 0 {
            return;
        }

        let remove_at = self.cursor - 1;
        let byte_index = char_to_byte_index(&self.text, remove_at);
        self.text.remove(byte_index);
        self.cursor = remove_at;
    }

    fn delete_right(&mut self) {
        if self.cursor >= self.text.chars().count() {
            return;
        }

        let byte_index = char_to_byte_index(&self.text, self.cursor);
        self.text.remove(byte_index);
    }

    fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    fn move_right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.text.chars().count());
    }

    fn move_home(&mut self) {
        self.cursor = 0;
    }

    fn move_end(&mut self) {
        self.cursor = self.text.chars().count();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragTarget {
    BrowserDivider(usize),
    EditorSelection,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Geometry {
    pub root: Rect,
    pub menu_area: Rect,
    pub menu: MenuGeometry,
    pub desktop_inner: Rect,
    pub browser_areas: [Rect; 2],
    pub browser_inners: [Rect; 2],
    pub editor_area: Rect,
    pub editor_inner: Rect,
}

pub const MIN_BROWSER_PANE_WIDTH: u16 = 18;
pub const MIN_EDITOR_PANE_WIDTH: u16 = 24;
const BROWSER_PANE_COUNT: usize = 2;
const BROWSER_PREVIEW_DELAY: Duration = Duration::from_millis(200);
const DIRECTORY_PREVIEW_MAX_ENTRIES: usize = 256;
const DEFAULT_BROWSER_SELECTION_PATTERN: &str = r".*\..*";

#[derive(Debug, Clone)]
pub struct BrowserPane {
    pub dir: PathBuf,
    pub entries: Vec<ProjectEntry>,
    pub selected_entry: usize,
    pub selected_paths: BTreeSet<PathBuf>,
}

impl BrowserPane {
    fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            entries: Vec::new(),
            selected_entry: 0,
            selected_paths: BTreeSet::new(),
        }
    }
}

#[derive(Debug)]
pub struct App {
    pub browsers: [BrowserPane; BROWSER_PANE_COUNT],
    pub secondary_browser_enabled: bool,
    pub editor_only_mode: bool,
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
    preview_label: Option<String>,
    pending_unsaved_action: Option<PendingUnsavedAction>,
    pending_new_directory_browser: Option<usize>,
    new_directory_input: TextInputState,
    pending_open_file_browser: Option<usize>,
    open_file_input: TextInputState,
    pending_file_operation: Option<PendingFileOperation>,
    pending_file_operation_name_input: TextInputState,
    search_pattern: String,
    browser_selection_pattern: String,
    search_input: TextInputState,
    pending_browser_incremental_search_index: Option<usize>,
    pending_browser_incremental_search_original_entry: Option<usize>,
    pending_browser_selection_pattern_mode: Option<BrowserSelectionPatternMode>,
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
            browsers: [
                BrowserPane::new(browser_dir.clone()),
                BrowserPane::new(browser_dir),
            ],
            secondary_browser_enabled: false,
            editor_only_mode: false,
            editor: Editor::scratch(),
            focus: Focus::BrowserPrimary,
            menu_open: false,
            active_menu: 0,
            active_menu_item: first_selectable_item(0),
            help_open: false,
            dialog: None,
            selection_mode: false,
            status: "Ready".to_string(),
            browser_pane_width: 30,
            geometry: Geometry::default(),
            preview_label: None,
            pending_unsaved_action: None,
            pending_new_directory_browser: None,
            new_directory_input: TextInputState::default(),
            pending_open_file_browser: None,
            open_file_input: TextInputState::default(),
            pending_file_operation: None,
            pending_file_operation_name_input: TextInputState::default(),
            search_pattern: String::new(),
            browser_selection_pattern: String::new(),
            search_input: TextInputState::default(),
            pending_browser_incremental_search_index: None,
            pending_browser_incremental_search_original_entry: None,
            pending_browser_selection_pattern_mode: None,
            full_redraw_requested: false,
            browser_preview_due_at: None,
            drag_target: None,
        }
    }

    pub fn refresh_browser(&mut self) {
        self.refresh_browser_pane(0);
        self.schedule_browser_preview();
    }

    pub fn toggle_focus(&mut self) {
        self.close_menu();
        if self.editor_only_mode {
            self.assign_focus(Focus::Editor);
            self.status = "Focus: Edit".to_string();
            return;
        }
        let focus = match self.focus {
            Focus::BrowserPrimary if self.secondary_browser_enabled => Focus::BrowserSecondary,
            Focus::BrowserPrimary => Focus::Editor,
            Focus::BrowserSecondary => Focus::BrowserPrimary,
            Focus::Editor => Focus::BrowserPrimary,
        };
        self.set_focus(focus);
    }

    pub fn focus_browser(&mut self) {
        self.set_focus(Focus::BrowserPrimary);
    }

    pub fn focus_editor(&mut self) {
        self.set_focus(Focus::Editor);
    }

    fn set_focus(&mut self, focus: Focus) {
        self.close_menu();
        let focus = if self.editor_only_mode { Focus::Editor } else { focus };
        if self.focus == Focus::Editor && focus != Focus::Editor && self.editor.is_dirty() {
            self.request_unsaved_action(PendingUnsavedAction::Focus(focus));
            return;
        }
        self.apply_focus_change(focus);
    }

    fn apply_focus_change(&mut self, focus: Focus) {
        self.assign_focus(focus);
        if self.focus_browser_index().is_some() {
            self.schedule_browser_preview();
        }
        self.status = format!("Focus: {}", self.focus_name());
    }

    fn assign_focus(&mut self, focus: Focus) {
        self.focus = focus;
    }

    pub fn toggle_secondary_browser(&mut self) {
        self.close_menu();
        if self.secondary_browser_enabled {
            self.browsers[0] = self.browsers[1].clone();
            self.secondary_browser_enabled = false;
            self.assign_focus(if self.editor_only_mode { Focus::Editor } else { Focus::BrowserPrimary });
            self.status = format!("Files pane: {}", self.browsers[0].dir.display());
        } else {
            self.browsers[1] = self.browsers[0].clone();
            self.secondary_browser_enabled = true;
            self.assign_focus(if self.editor_only_mode { Focus::Editor } else { Focus::BrowserSecondary });
            self.status = format!("Second files pane: {}", self.browsers[1].dir.display());
        }
        self.schedule_browser_preview();
    }

    pub fn toggle_editor_only_mode(&mut self) {
        self.close_menu();
        self.help_open = false;
        self.editor_only_mode = !self.editor_only_mode;
        self.assign_focus(Focus::Editor);
        self.status = if self.editor_only_mode {
            "Editor only mode enabled".to_string()
        } else {
            "Editor only mode disabled".to_string()
        };
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

        let Some(browser_index) = self.focus_browser_index() else {
            return;
        };

        if Instant::now() < due
            || self.menu_open
            || self.help_open
            || self.dialog.is_some()
            || self.editor.is_dirty()
        {
            return;
        }

        self.browser_preview_due_at = None;

        let browser = &self.browsers[browser_index];
        let Some(entry) = browser.entries.get(browser.selected_entry) else {
            return;
        };

        if entry.kind == ProjectEntryKind::Parent {
            self.status = "Directory Up".to_string();
            return;
        }

        if entry.kind == ProjectEntryKind::Directory {
            let preview_label = format!("{} [tree]", entry.path.display());
            if self.preview_label.as_deref() == Some(preview_label.as_str()) {
                return;
            }

            self.editor = Editor::from_lines(directory_subtree_lines(
                &entry.path,
                DIRECTORY_PREVIEW_MAX_ENTRIES,
            ));
            self.preview_label = Some(preview_label);
            self.status = format!("Previewed tree for {}", entry.path.display());
            return;
        }

        if self.editor.path() == Some(entry.path.as_path()) {
            return;
        }

        let path = entry.path.clone();
        match Editor::open(&path) {
            Ok(editor) => {
                self.editor = editor;
                self.preview_label = None;
                self.status = format!("Previewed {}", path.display());
            }
            Err(error) => {
                self.status = format!("Preview failed: {error}");
            }
        }
    }

    pub fn focus_name(&self) -> &'static str {
        match self.focus {
            Focus::BrowserPrimary => "Files 1",
            Focus::BrowserSecondary => "Files 2",
            Focus::Editor => "Edit",
        }
    }

    pub fn current_file_label(&self) -> String {
        if let Some(label) = &self.preview_label {
            return label.clone();
        }

        self.editor
            .path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "Untitled".to_string())
    }

    pub fn save_file_dialog_title(&self) -> &'static str {
        match self.pending_unsaved_action.as_ref() {
            Some(PendingUnsavedAction::Quit) => "Save File Before Exiting?",
            Some(PendingUnsavedAction::Focus(_)) => "Save File Before Changing Pane?",
            Some(PendingUnsavedAction::OpenPath { .. }) => "Save File Before Opening Another File?",
            None => "Save File?",
        }
    }

    pub fn save_file_dialog_yes_label(&self) -> &'static str {
        match self.pending_unsaved_action.as_ref() {
            Some(PendingUnsavedAction::Quit) => " = Save and exit",
            Some(PendingUnsavedAction::Focus(_)) => " = Save and change pane",
            Some(PendingUnsavedAction::OpenPath { .. }) => " = Save and open file",
            None => " = Save",
        }
    }

    pub fn save_file_dialog_no_label(&self) -> &'static str {
        match self.pending_unsaved_action.as_ref() {
            Some(PendingUnsavedAction::Quit) => " = Exit without saving, changes lost!",
            Some(PendingUnsavedAction::Focus(_)) => " = Change pane without saving",
            Some(PendingUnsavedAction::OpenPath { .. }) => " = Open without saving, changes lost!",
            None => " = Continue without saving",
        }
    }

    pub fn save_file_dialog_cancel_label(&self) -> &'static str {
        match self.pending_unsaved_action.as_ref() {
            Some(PendingUnsavedAction::Quit) => " = Stay in application and continue editing",
            Some(PendingUnsavedAction::Focus(_)) => " = Stay in editor and continue editing",
            Some(PendingUnsavedAction::OpenPath { .. }) => " = Stay on current file and continue editing",
            None => " = Continue editing",
        }
    }

    pub fn pending_file_operation_title(&self) -> Option<&'static str> {
        let operation = self.pending_file_operation.as_ref()?;
        Some(operation.kind.confirm_title(operation.source_parent(), operation.target_dir.as_deref(), operation.sources.len()))
    }

    pub fn pending_file_operation_paths(&self) -> Option<(String, Option<String>)> {
        let operation = self.pending_file_operation.as_ref()?;
        let source = if operation.sources.len() == 1 {
            operation.sources[0].display().to_string()
        } else {
            format!("{} files", operation.sources.len())
        };
        let target = match operation.kind {
            FileOperationKind::Delete => None,
            FileOperationKind::Copy | FileOperationKind::Move => operation
                .result_paths()
                .first()
                .map(|path| path.display().to_string())
                .or_else(|| operation.target_dir.as_ref().map(|path| path.display().to_string())),
        };
        Some((source, target))
    }

    pub fn pending_file_operation_browser_index(&self) -> Option<usize> {
        let operation = self.pending_file_operation.as_ref()?;
        Some(operation.browser_index)
    }

    pub fn pending_file_operation_prompt_title(&self) -> Option<&'static str> {
        let operation = self.pending_file_operation.as_ref()?;
        if operation.rename_from_conflict {
            Some("Enter a new file name")
        } else {
            Some(operation.kind.name_prompt_title())
        }
    }

    pub fn pending_file_operation_name(&self) -> Option<&str> {
        self.pending_file_operation.as_ref()?;
        Some(self.pending_file_operation_name_input.as_str())
    }

    pub fn pending_file_operation_name_cursor(&self) -> Option<usize> {
        self.pending_file_operation.as_ref()?;
        Some(self.pending_file_operation_name_input.cursor())
    }

    pub fn pending_file_conflict_title(&self) -> Option<&'static str> {
        let operation = self.pending_file_operation.as_ref()?;
        Some(operation.conflict_title())
    }

    pub fn pending_file_conflict_paths(&self) -> Option<(String, String)> {
        let operation = self.pending_file_operation.as_ref()?;
        Some((
            operation.current_source()?.display().to_string(),
            operation.current_target_path()?.display().to_string(),
        ))
    }

    pub fn pending_new_directory_parent(&self) -> Option<String> {
        let browser_index = self.pending_new_directory_browser?;
        Some(self.browsers[browser_index].dir.display().to_string())
    }

    pub fn pending_new_directory_name(&self) -> Option<&str> {
        if self.pending_new_directory_browser.is_some() {
            Some(self.new_directory_input.as_str())
        } else {
            None
        }
    }

    pub fn open_file_input(&self) -> Option<&str> {
        if self.pending_open_file_browser.is_some() {
            Some(self.open_file_input.as_str())
        } else {
            None
        }
    }

    pub fn open_file_input_cursor(&self) -> Option<usize> {
        if self.pending_open_file_browser.is_some() {
            Some(self.open_file_input.cursor())
        } else {
            None
        }
    }

    pub fn search_pattern(&self) -> &str {
        self.search_input.as_str()
    }

    pub fn search_pattern_cursor(&self) -> usize {
        self.search_input.cursor()
    }

    pub fn browser_selection_pattern_title(&self) -> &'static str {
        match self.pending_browser_selection_pattern_mode {
            Some(BrowserSelectionPatternMode::Add) => "Add files to selection by regex",
            Some(BrowserSelectionPatternMode::Remove) => "Remove files from selection by regex",
            None => "File name regex",
        }
    }

    pub fn browser_label(&self, index: usize) -> String {
        let browser = &self.browsers[index];
        let selected_count = browser.selected_paths.len();
        if selected_count == 0 {
            browser.dir.display().to_string()
        } else {
            format!("{} ({selected_count} selected)", browser.dir.display())
        }
    }

    pub fn browser_entry_is_selected(&self, browser_index: usize, entry: &ProjectEntry) -> bool {
        self.browsers[browser_index].selected_paths.contains(&entry.path)
    }

    pub fn open_selected_file(&mut self) {
        self.close_menu();
        let Some(browser_index) = self.focus_browser_index() else {
            self.status = "Files pane is not active".to_string();
            return;
        };

        let Some(entry) = self.browsers[browser_index]
            .entries
            .get(self.browsers[browser_index].selected_entry)
            .cloned()
        else {
            self.status = "No files in this directory".to_string();
            return;
        };

        if entry.is_directory() {
            let return_path = if entry.kind == ProjectEntryKind::Parent {
                Some(self.browsers[browser_index].dir.clone())
            } else {
                None
            };
            self.navigate_to_dir(browser_index, entry.path, return_path.as_deref());
            return;
        }

        self.open_path_in_browser(browser_index, entry.path);
    }

    pub fn open_path(&mut self, path: PathBuf) {
        let browser_index = self.focus_browser_index().unwrap_or(0);
        self.open_path_in_browser(browser_index, path);
    }

    fn open_path_in_browser(&mut self, browser_index: usize, path: PathBuf) {
        if self.editor.is_dirty() {
            self.request_unsaved_action(PendingUnsavedAction::OpenPath { path, browser_index });
            return;
        }

        self.apply_open_path(browser_index, path);
    }

    fn apply_open_path(&mut self, browser_index: usize, path: PathBuf) {
        let created = if path.exists() {
            false
        } else {
            match fs::write(&path, "") {
                Ok(()) => true,
                Err(error) => {
                    self.status = format!("Open failed: {error}");
                    return;
                }
            }
        };

        let path = path.canonicalize().unwrap_or(path);
        if let Some(parent) = path.parent() {
            self.navigate_to_dir(browser_index, parent.to_path_buf(), Some(&path));
            if created {
                for other_index in 0..self.visible_browser_count() {
                    if other_index == browser_index || self.browsers[other_index].dir != parent {
                        continue;
                    }
                    self.refresh_browser_pane(other_index);
                    self.select_entry_for_path(other_index, &path);
                }
            }
        }

        match Editor::open(&path) {
            Ok(editor) => {
                self.editor = editor;
                self.preview_label = None;
                self.assign_focus(Focus::Editor);
                self.status = if created {
                    format!("Opened new {}", path.display())
                } else {
                    format!("Opened {}", path.display())
                };
            }
            Err(error) => self.status = format!("Open failed: {error}"),
        }
    }

    fn navigate_to_dir(&mut self, browser_index: usize, path: PathBuf, selected_path: Option<&Path>) {
        let path = path.canonicalize().unwrap_or(path);
        let browser = &mut self.browsers[browser_index];
        browser.dir = path;
        browser.selected_entry = 0;
        browser.selected_paths.clear();
        self.refresh_browser_pane(browser_index);
        if let Some(selected_path) = selected_path {
            self.select_entry_for_path(browser_index, selected_path);
        }
        self.assign_focus(Self::focus_for_browser_index(browser_index));
        self.status = format!("Browsing {}", self.browser_label(browser_index));
    }

    pub fn save_current(&mut self) -> bool {
        self.close_menu();
        match self.editor.save() {
            Ok(SaveOutcome::Saved) => {
                self.refresh_buffers_after_save();
                self.status = format!("Saved {}", self.current_file_label());
                true
            }
            Ok(SaveOutcome::Unchanged) => true,
            Err(error) => {
                self.status = format!("Save failed: {error}");
                false
            }
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

        let cwd = path.parent().unwrap_or(self.browsers[0].dir.as_path());

        let Some(spec) = detect_file_type(Some(path), self.editor.lines().first().map(String::as_str)) else {
            let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or_default();
            self.status = if extension.is_empty() {
                "Run is not configured for this file".to_string()
            } else {
                format!("Run is not configured for .{extension}")
            };
            return;
        };

        let Some(invocation) = spec.run else {
            self.status = format!("Run is not configured for .{}", spec.extension);
            return;
        };

        let (command, description) = format_tool_invocation(path, invocation);

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

        let cwd = path.parent().unwrap_or(self.browsers[0].dir.as_path());

        let Some(spec) = detect_file_type(Some(path), self.editor.lines().first().map(String::as_str)) else {
            let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or_default();
            self.status = if extension.is_empty() {
                "Build is not configured for this file".to_string()
            } else {
                format!("Build is not configured for .{extension}")
            };
            return;
        };

        let Some(invocation) = spec.build else {
            self.status = format!("Build is not configured for .{}", spec.extension);
            return;
        };

        let (command, description) = format_tool_invocation(path, invocation);

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
            Focus::BrowserPrimary | Focus::BrowserSecondary => self.handle_browser_key(key),
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
            Dialog::SaveFile => match _key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => self.confirm_save_file_dialog(true),
                KeyCode::Char('n') | KeyCode::Char('N') => self.confirm_save_file_dialog(false),
                KeyCode::Esc => {
                    self.dialog = None;
                    self.pending_unsaved_action = None;
                    self.status = "Action cancelled".to_string();
                    Action::None
                }
                _ => Action::None,
            },
            Dialog::NewDirectory => match _key.code {
                KeyCode::Enter => {
                    self.confirm_new_directory();
                    Action::None
                }
                KeyCode::Esc => {
                    self.clear_new_directory_request();
                    self.status = "New directory cancelled".to_string();
                    Action::None
                }
                _ => handle_text_input_key(&mut self.new_directory_input, _key),
            },
            Dialog::OpenFilePath => match _key.code {
                KeyCode::Enter => {
                    self.confirm_open_file_dialog();
                    Action::None
                }
                KeyCode::Esc => {
                    self.clear_open_file_request();
                    self.status = "Open file cancelled".to_string();
                    Action::None
                }
                _ => handle_text_input_key(&mut self.open_file_input, _key),
            },
            Dialog::RegexSearch => match _key.code {
                KeyCode::Enter => {
                    self.confirm_regex_search();
                    Action::None
                }
                KeyCode::Esc => {
                    self.dialog = None;
                    self.status = "Search cancelled".to_string();
                    Action::None
                }
                _ => handle_text_input_key(&mut self.search_input, _key),
            },
            Dialog::BrowserIncrementalSearch => match _key.code {
                KeyCode::Enter => {
                    self.confirm_browser_incremental_search();
                    Action::None
                }
                KeyCode::Esc => {
                    self.clear_browser_incremental_search_request(true);
                    self.status = "Incremental search cancelled".to_string();
                    Action::None
                }
                _ => {
                    handle_text_input_key(&mut self.search_input, _key);
                    self.update_browser_incremental_search();
                    Action::None
                }
            },
            Dialog::BrowserSelectionPattern => match _key.code {
                KeyCode::Enter => {
                    self.confirm_browser_selection_pattern();
                    Action::None
                }
                KeyCode::Esc => {
                    self.clear_browser_selection_pattern_request();
                    self.status = "Selection pattern cancelled".to_string();
                    Action::None
                }
                _ => handle_text_input_key(&mut self.search_input, _key),
            },
            Dialog::FileOperationName => match _key.code {
                KeyCode::Enter => {
                    self.confirm_file_operation_name();
                    Action::None
                }
                KeyCode::Esc => {
                    self.clear_file_operation_request();
                    self.status = "File operation cancelled".to_string();
                    Action::None
                }
                _ => handle_text_input_key(&mut self.pending_file_operation_name_input, _key),
            },
            Dialog::ConfirmFileOperation => match _key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    self.run_pending_file_operation();
                    Action::None
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.clear_file_operation_request();
                    self.status = "File operation cancelled".to_string();
                    Action::None
                }
                _ => Action::None,
            },
            Dialog::ResolveFileConflict => match _key.code {
                KeyCode::Char('o') | KeyCode::Char('O') => {
                    self.resolve_file_conflict(FileConflictResolution::Overwrite);
                    Action::None
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.resolve_file_conflict(FileConflictResolution::Skip);
                    Action::None
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    self.resolve_file_conflict(FileConflictResolution::Rename);
                    Action::None
                }
                KeyCode::Esc => {
                    self.clear_file_operation_request();
                    self.status = "File operation cancelled".to_string();
                    Action::None
                }
                _ => Action::None,
            },
        }
    }

    pub fn request_quit(&mut self) -> Action {
        self.close_menu();
        self.help_open = false;

        if self.editor.is_dirty() {
            self.request_unsaved_action(PendingUnsavedAction::Quit);
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
            MenuAction::Save => {
                self.save_current();
            }
            MenuAction::Quit => return self.request_quit(),
            MenuAction::Undo => self.undo_last_edit(),
            MenuAction::Redo => self.redo_last_edit(),
            MenuAction::Copy => self.copy_selection(),
            MenuAction::Cut => self.cut_selection(),
            MenuAction::Paste => self.paste_from_clipboard(),
            MenuAction::Search => self.request_search(),
            MenuAction::CopyEntry => self.request_copy_selected_entry(),
            MenuAction::MoveEntry => self.request_move_selected_entry(),
            MenuAction::NewDirectory => self.request_new_directory(),
            MenuAction::DeleteEntry => self.request_delete_selected_entry(),
            MenuAction::DeleteLine => self.editor.delete_line(),
            MenuAction::CargoRun => self.run_current_target(),
            MenuAction::CargoBuild => self.build_current_target(),
            MenuAction::ToggleFocus => self.toggle_focus(),
            MenuAction::ToggleDualPane => self.toggle_secondary_browser(),
            MenuAction::ToggleEditorOnly => self.toggle_editor_only_mode(),
            MenuAction::FocusBrowser => self.set_focus(Focus::BrowserPrimary),
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

    pub fn redo_last_edit(&mut self) {
        self.dialog = None;
        self.help_open = false;
        self.assign_focus(Focus::Editor);

        if self.editor.redo() {
            self.status = "Redo applied".to_string();
        } else {
            self.status = "Nothing to redo".to_string();
        }
    }

    pub fn request_copy_selected_entry(&mut self) {
        self.request_selected_file_operation(FileOperationKind::Copy);
    }

    pub fn request_move_selected_entry(&mut self) {
        self.request_selected_file_operation(FileOperationKind::Move);
    }

    pub fn request_delete_selected_entry(&mut self) {
        self.request_selected_file_operation(FileOperationKind::Delete);
    }

    pub fn toggle_selected_browser_entry(&mut self) {
        self.close_menu();
        let Some(browser_index) = self.focus_browser_index() else {
            self.status = "Activate a files pane first".to_string();
            return;
        };

        let Some(entry) = self.browsers[browser_index]
            .entries
            .get(self.browsers[browser_index].selected_entry)
            .cloned()
        else {
            self.status = "No file selected".to_string();
            return;
        };

        if entry.kind == ProjectEntryKind::Parent {
            self.status = "Parent entry cannot be selected".to_string();
            return;
        }

        let browser = &mut self.browsers[browser_index];
        if browser.selected_paths.remove(&entry.path) {
            let remaining = browser.selected_paths.len();
            self.status = if remaining == 0 {
                format!("Deselected {}", entry.label)
            } else {
                format!("Deselected {} ({remaining} selected)", entry.label)
            };
        } else {
            browser.selected_paths.insert(entry.path.clone());
            self.status = format!("Selected {} ({} selected)", entry.label, browser.selected_paths.len());
        }

        let next_index = (browser.selected_entry + 1).min(browser.entries.len().saturating_sub(1));
        if next_index != browser.selected_entry {
            browser.selected_entry = next_index;
            self.schedule_browser_preview();
        }
    }

    pub fn request_new_directory(&mut self) {
        self.close_menu();
        let Some(browser_index) = self.focus_browser_index() else {
            self.status = "Activate a files pane first".to_string();
            return;
        };

        self.pending_new_directory_browser = Some(browser_index);
        self.new_directory_input.clear();
        self.dialog = Some(Dialog::NewDirectory);
        self.status = format!("New sub-directory in {}", self.browser_label(browser_index));
    }

    pub fn request_open_file_dialog(&mut self) {
        self.close_menu();
        let browser_index = self.focus_browser_index().unwrap_or(0);
        self.pending_open_file_browser = Some(browser_index);
        self.open_file_input.clear();
        self.dialog = Some(Dialog::OpenFilePath);
        self.status = format!("Open file in {}", self.browser_label(browser_index));
    }

    pub fn request_regex_search(&mut self) {
        self.close_menu();
        self.assign_focus(Focus::Editor);
        self.pending_browser_selection_pattern_mode = None;
        self.search_input.set_text(self.search_pattern.clone());
        self.dialog = Some(Dialog::RegexSearch);
        self.status = "Enter regular expression to search".to_string();
    }

    pub fn request_search(&mut self) {
        if self.focus_browser_index().is_some() {
            self.request_browser_incremental_search();
        } else {
            self.request_regex_search();
        }
    }

    fn request_browser_incremental_search(&mut self) {
        self.close_menu();
        let Some(browser_index) = self.focus_browser_index() else {
            self.status = "Activate a files pane first".to_string();
            return;
        };

        self.pending_browser_selection_pattern_mode = None;
        self.pending_browser_incremental_search_index = Some(browser_index);
        self.pending_browser_incremental_search_original_entry =
            Some(self.browsers[browser_index].selected_entry);
        self.search_input.clear();
        self.dialog = Some(Dialog::BrowserIncrementalSearch);
        self.status = "Incremental search: type to locate a file".to_string();
    }

    fn request_browser_selection_pattern(&mut self, mode: BrowserSelectionPatternMode) {
        self.close_menu();
        let Some(_) = self.focus_browser_index() else {
            self.status = "Activate a files pane first".to_string();
            return;
        };

        self.pending_browser_selection_pattern_mode = Some(mode);
        let initial_pattern = if self.browser_selection_pattern.is_empty() {
            DEFAULT_BROWSER_SELECTION_PATTERN
        } else {
            self.browser_selection_pattern.as_str()
        };
        self.search_input.set_text(initial_pattern.to_string());
        self.dialog = Some(Dialog::BrowserSelectionPattern);
        self.status = match mode {
            BrowserSelectionPatternMode::Add => {
                "Enter file name regex to add matching files to selection".to_string()
            }
            BrowserSelectionPatternMode::Remove => {
                "Enter file name regex to remove matching files from selection".to_string()
            }
        };
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

        if let Some(divider_index) = self.browser_divider_at(column, row) {
            self.drag_target = Some(DragTarget::BrowserDivider(divider_index));
            self.resize_browser_pane(divider_index, column);
            return Action::None;
        }

        if let Some(browser_index) = self.browser_inner_at(column, row) {
            self.assign_focus(Self::focus_for_browser_index(browser_index));
            self.select_entry_at(browser_index, row);
        } else if contains(self.geometry.editor_inner, column, row) {
            self.assign_focus(Focus::Editor);
            self.place_cursor_at(column, row, false);
            self.drag_target = Some(DragTarget::EditorSelection);
        } else if let Some(browser_index) = self.browser_area_at(column, row) {
            self.assign_focus(Self::focus_for_browser_index(browser_index));
            self.status = format!("Focus: {}", self.focus_name());
        } else if contains(self.geometry.editor_area, column, row) {
            self.assign_focus(Focus::Editor);
            self.status = "Focus: Edit".to_string();
        }

        Action::None
    }

    fn handle_mouse_drag(&mut self, column: u16, row: u16) {
        match self.drag_target {
            Some(DragTarget::BrowserDivider(divider_index)) => {
                self.resize_browser_pane(divider_index, column)
            }
            Some(DragTarget::EditorSelection) => self.place_cursor_at(column, row, true),
            None => {}
        }
    }

    fn handle_mouse_scroll(&mut self, column: u16, row: u16, amount: isize) {
        if self.dialog.is_some() || self.help_open {
            return;
        }

        if let Some(browser_index) = self.browser_inner_at(column, row) {
            self.assign_focus(Self::focus_for_browser_index(browser_index));
            if amount < 0 {
                for _ in 0..amount.unsigned_abs() {
                    self.select_previous_entry(browser_index);
                }
            } else {
                for _ in 0..amount as usize {
                    self.select_next_entry(browser_index);
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
        let Some(browser_index) = self.focus_browser_index() else {
            return;
        };

        match key.code {
            KeyCode::Char('`') => self.toggle_secondary_browser(),
            KeyCode::Up => self.select_previous_entry(browser_index),
            KeyCode::Down => self.select_next_entry(browser_index),
            KeyCode::PageUp => self.page_up_browser(browser_index),
            KeyCode::PageDown => self.page_down_browser(browser_index),
            KeyCode::Home => self.set_selected_entry(browser_index, 0),
            KeyCode::End => {
                let last = self.browsers[browser_index].entries.len().saturating_sub(1);
                self.set_selected_entry(browser_index, last);
            }
            KeyCode::Enter => self.open_selected_file(),
            KeyCode::Backspace => {
                let current_dir = self.browsers[browser_index].dir.clone();
                let parent = current_dir.parent().map(Path::to_path_buf);
                if let Some(parent) = parent {
                    self.navigate_to_dir(browser_index, parent, Some(&current_dir));
                }
            }
            KeyCode::Char('+') => {
                self.request_browser_selection_pattern(BrowserSelectionPatternMode::Add)
            }
            KeyCode::Char('-') => {
                self.request_browser_selection_pattern(BrowserSelectionPatternMode::Remove)
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.refresh_browser_pane(browser_index);
                self.status = "Directory refreshed".to_string();
            }
            _ => {}
        }
    }

    fn request_selected_file_operation(&mut self, kind: FileOperationKind) {
        self.close_menu();
        let Some(browser_index) = self.focus_browser_index() else {
            self.status = "Activate a files pane first".to_string();
            return;
        };

        let (sources, source_name) = {
            let browser = &self.browsers[browser_index];
            let mut sources = if browser.selected_paths.is_empty() {
                let Some(entry) = browser.entries.get(browser.selected_entry) else {
                    self.status = "No file selected".to_string();
                    return;
                };

                if entry.kind == ProjectEntryKind::Parent {
                    self.status = "Parent entry cannot be used for file operations".to_string();
                    return;
                }
                vec![entry.path.clone()]
            } else {
                browser.selected_paths.iter().cloned().collect::<Vec<_>>()
            };
            sources.sort();
            let source_name = sources
                .first()
                .and_then(|path| path.file_name())
                .map(PathBuf::from)
                .unwrap_or_default();
            (sources, source_name)
        };

        let target_dir = match kind {
            FileOperationKind::Delete => None,
            FileOperationKind::Copy | FileOperationKind::Move => Some(if self.secondary_browser_enabled {
                self.browsers[1 - browser_index].dir.clone()
            } else {
                self.browsers[browser_index].dir.clone()
            }),
        };

        if sources.len() > 1
            && matches!(kind, FileOperationKind::Copy | FileOperationKind::Move)
            && target_dir.as_deref() == sources.first().and_then(|path| path.parent())
        {
            self.status = format!(
                "{} {} selected files needs a different target directory",
                kind.label().to_ascii_uppercase(),
                sources.len()
            );
            return;
        }

        let single_source = sources.len() == 1;

        self.pending_file_operation = Some(PendingFileOperation {
            kind,
            sources,
            target_dir,
            target_name: (kind != FileOperationKind::Delete && single_source)
                .then_some(source_name),
            browser_index,
            current_index: 0,
            completed_targets: Vec::new(),
            overwritten_count: 0,
            skipped_count: 0,
            rename_from_conflict: false,
        });

        if self.should_prompt_for_file_operation_name() {
            let default_name = self
                .pending_file_operation
                .as_ref()
                .and_then(|operation| operation.sources.first())
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            self.pending_file_operation_name_input.set_text(default_name);
            self.dialog = Some(Dialog::FileOperationName);
            self.status = format!("Choose new name for {}", kind.label());
            return;
        }

        self.dialog = Some(Dialog::ConfirmFileOperation);
        self.status = format!("Confirm {}", kind.label());
    }

    fn confirm_new_directory(&mut self) {
        let Some(browser_index) = self.pending_new_directory_browser else {
            self.dialog = None;
            return;
        };

        let name = self.new_directory_input.as_str().trim();
        if name.is_empty() {
            self.status = "Directory name cannot be empty".to_string();
            return;
        }
        if name == "." || name == ".." || name.chars().any(std::path::is_separator) {
            self.status = "Directory name must be a single path component".to_string();
            return;
        }

        let target = self.browsers[browser_index].dir.join(name);
        if target.exists() {
            self.status = format!("{} already exists", target.display());
            return;
        }

        match fs::create_dir(&target) {
            Ok(()) => {
                self.clear_new_directory_request();
                for index in 0..self.visible_browser_count() {
                    self.refresh_browser_pane(index);
                }
                self.assign_focus(Self::focus_for_browser_index(browser_index));
                self.select_entry_for_path(browser_index, &target);
                self.schedule_browser_preview();
                self.status = format!("Created {}", target.display());
            }
            Err(error) => {
                self.status = format!("Create directory failed: {error}");
            }
        }
    }

    fn clear_new_directory_request(&mut self) {
        self.dialog = None;
        self.pending_new_directory_browser = None;
        self.new_directory_input.clear();
    }

    fn confirm_open_file_dialog(&mut self) {
        let Some(browser_index) = self.pending_open_file_browser else {
            self.dialog = None;
            return;
        };

        let entered = self.open_file_input.as_str().trim().to_string();
        if entered.is_empty() {
            self.status = "Path cannot be empty".to_string();
            return;
        }

        let Some(path) = self.resolve_open_file_input(browser_index, &entered) else {
            return;
        };

        self.clear_open_file_request();
        self.open_path_in_browser(browser_index, path);
    }

    fn clear_open_file_request(&mut self) {
        self.dialog = None;
        self.pending_open_file_browser = None;
        self.open_file_input.clear();
    }

    fn resolve_open_file_input(&mut self, browser_index: usize, input: &str) -> Option<PathBuf> {
        let expanded = match expand_tilde_path(input) {
            Ok(path) => path,
            Err(message) => {
                self.status = message;
                return None;
            }
        };

        let path = if expanded.is_absolute() {
            expanded
        } else {
            self.browsers[browser_index].dir.join(expanded)
        };

        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            self.status = "Path must end with a file name".to_string();
            return None;
        };
        if file_name == "." || file_name == ".." {
            self.status = "Path must end with a file name".to_string();
            return None;
        }

        let Some(directory) = path.parent() else {
            self.status = "Path must include a directory or file name".to_string();
            return None;
        };
        if !directory.is_dir() {
            self.status = format!("Directory not found: {}", directory.display());
            return None;
        }

        Some(path)
    }

    fn confirm_regex_search(&mut self) {
        if self.search_input.as_str().is_empty() {
            self.status = "Search pattern cannot be empty".to_string();
            return;
        }

        let pattern = self.search_input.as_str().to_string();
        let regex = match Regex::new(&pattern) {
            Ok(regex) => regex,
            Err(error) => {
                self.status = format!("Invalid regex: {error}");
                return;
            }
        };

        let Some((row, start_col, end_col)) = find_regex_match(
            self.editor.lines(),
            self.editor.cursor_row(),
            self.editor.cursor_col(),
            &regex,
        ) else {
            self.status = format!("No match for /{pattern}/");
            return;
        };

        self.assign_focus(Focus::Editor);
        self.search_pattern = pattern.clone();
        self.editor.set_cursor(row, start_col);
        self.editor.begin_selection();
        self.editor.select_to(row, end_col);
        self.editor.center_view_on(row, start_col);
        self.status = format!("Found /{pattern}/ at {}:{}", row + 1, start_col + 1);
    }

    fn confirm_browser_incremental_search(&mut self) {
        let label = self
            .pending_browser_incremental_search_index
            .and_then(|browser_index| {
                self.browsers[browser_index]
                    .entries
                    .get(self.browsers[browser_index].selected_entry)
                    .map(|entry| entry.label.clone())
            });
        self.clear_browser_incremental_search_request(false);
        if let Some(label) = label {
            self.status = format!("Incremental search selected {label}");
        } else {
            self.status = "Incremental search closed".to_string();
        }
    }

    fn clear_browser_incremental_search_request(&mut self, restore_selection: bool) {
        if restore_selection
            && let (Some(browser_index), Some(original_entry)) = (
                self.pending_browser_incremental_search_index,
                self.pending_browser_incremental_search_original_entry,
            )
        {
            self.set_selected_entry(browser_index, original_entry);
        }

        self.dialog = None;
        self.pending_browser_incremental_search_index = None;
        self.pending_browser_incremental_search_original_entry = None;
        self.search_input.clear();
    }

    fn update_browser_incremental_search(&mut self) {
        let Some(browser_index) = self.pending_browser_incremental_search_index else {
            return;
        };

        let query = self.search_input.as_str().trim().to_ascii_lowercase();
        if query.is_empty() {
            if let Some(original_entry) = self.pending_browser_incremental_search_original_entry {
                self.set_selected_entry(browser_index, original_entry);
            }
            self.status = "Incremental search: type to locate a file".to_string();
            return;
        }

        let entries = &self.browsers[browser_index].entries;
        if entries.is_empty() {
            self.status = "No files in this directory".to_string();
            return;
        }

        let start = self.browsers[browser_index].selected_entry;
        let mut found = None;
        for offset in 0..entries.len() {
            let index = (start + offset) % entries.len();
            let entry = &entries[index];
            if entry.kind == ProjectEntryKind::Parent {
                continue;
            }

            if entry.label.to_ascii_lowercase().contains(&query) {
                found = Some(index);
                break;
            }
        }

        if let Some(index) = found {
            let label = entries[index].label.clone();
            self.set_selected_entry(browser_index, index);
            self.status = format!("Match: {label}");
        } else {
            self.status = format!("No files matching \"{query}\"");
        }
    }

    fn confirm_browser_selection_pattern(&mut self) {
        let Some(mode) = self.pending_browser_selection_pattern_mode else {
            self.dialog = None;
            return;
        };

        let Some(browser_index) = self.focus_browser_index() else {
            self.clear_browser_selection_pattern_request();
            self.status = "Activate a files pane first".to_string();
            return;
        };

        if self.search_input.as_str().is_empty() {
            self.status = "Search pattern cannot be empty".to_string();
            return;
        }

        let pattern = self.search_input.as_str().to_string();
        let regex = match Regex::new(&pattern) {
            Ok(regex) => regex,
            Err(error) => {
                self.status = format!("Invalid regex: {error}");
                return;
            }
        };
        self.browser_selection_pattern = pattern.clone();

        let browser = &mut self.browsers[browser_index];
        let mut changed = 0usize;
        let mut first_match: Option<PathBuf> = None;

        for entry in &browser.entries {
            if entry.kind == ProjectEntryKind::Parent {
                continue;
            }
            if !regex.is_match(&entry.label) {
                continue;
            }

            if first_match.is_none() {
                first_match = Some(entry.path.clone());
            }

            match mode {
                BrowserSelectionPatternMode::Add => {
                    if browser.selected_paths.insert(entry.path.clone()) {
                        changed += 1;
                    }
                }
                BrowserSelectionPatternMode::Remove => {
                    if browser.selected_paths.remove(&entry.path) {
                        changed += 1;
                    }
                }
            }
        }

        self.clear_browser_selection_pattern_request();
        if let Some(path) = first_match.as_deref() {
            self.select_entry_for_path(browser_index, path);
            self.schedule_browser_preview();
        }
        self.status = match mode {
            BrowserSelectionPatternMode::Add => {
                format!("Added {changed} files to selection with /{pattern}/")
            }
            BrowserSelectionPatternMode::Remove => {
                format!("Removed {changed} files from selection with /{pattern}/")
            }
        };
    }

    fn clear_browser_selection_pattern_request(&mut self) {
        self.dialog = None;
        self.pending_browser_selection_pattern_mode = None;
        self.search_input.clear();
    }

    fn should_prompt_for_file_operation_name(&self) -> bool {
        let Some(operation) = self.pending_file_operation.as_ref() else {
            return false;
        };

        if operation.sources.len() != 1 {
            return false;
        }

        matches!(operation.kind, FileOperationKind::Copy | FileOperationKind::Move)
            && operation.sources.first().and_then(|path| path.parent()) == operation.target_dir.as_deref()
    }

    fn confirm_file_operation_name(&mut self) {
        let Some(operation) = self.pending_file_operation.as_ref() else {
            self.dialog = None;
            return;
        };

        let name = self.pending_file_operation_name_input.as_str().trim();
        if name.is_empty() {
            self.status = "File name cannot be empty".to_string();
            return;
        }
        if name == "." || name == ".." || name.chars().any(std::path::is_separator) {
            self.status = "File name must be a single path component".to_string();
            return;
        }

        let Some(parent) = operation.target_dir.as_deref() else {
            self.clear_file_operation_request();
            self.status = "File operation target is missing".to_string();
            return;
        };

        let target_path = parent.join(name);
        if operation.kind == FileOperationKind::Copy
            && operation.sources.first().is_some_and(|source| target_path == *source)
        {
            self.status = "Copy name must be changed".to_string();
            return;
        }
        if let Some(operation) = self.pending_file_operation.as_mut() {
            operation.target_name = target_path.file_name().map(PathBuf::from);
            operation.rename_from_conflict = false;
        }
        self.run_pending_file_operation();
    }

    fn clear_file_operation_request(&mut self) {
        self.dialog = None;
        self.pending_file_operation = None;
        self.pending_file_operation_name_input.clear();
    }


    fn run_pending_file_operation(&mut self) {
        self.dialog = None;

        loop {
            let Some(operation) = self.pending_file_operation.as_mut() else {
                return;
            };

            match try_process_pending_file_operation_step(operation) {
                Ok(FileOperationStep::Continue) => {}
                Ok(FileOperationStep::Done) => {
                    let operation = self.pending_file_operation.clone().expect("pending operation missing");
                    self.finish_pending_file_operation(operation, Ok(()));
                    return;
                }
                Ok(FileOperationStep::Conflict) => {
                    self.dialog = Some(Dialog::ResolveFileConflict);
                    self.status = "File already exists".to_string();
                    return;
                }
                Err(error) => {
                    let operation = self.pending_file_operation.clone().expect("pending operation missing");
                    self.finish_pending_file_operation(operation, Err(error));
                    return;
                }
            }
        }
    }

    fn resolve_file_conflict(&mut self, resolution: FileConflictResolution) {
        match resolution {
            FileConflictResolution::Overwrite => {
                let Some(operation) = self.pending_file_operation.as_mut() else {
                    self.dialog = None;
                    return;
                };
                let Some(target_path) = operation.current_target_path() else {
                    let operation = self.pending_file_operation.clone().expect("pending operation missing");
                    self.finish_pending_file_operation(
                        operation,
                        Err(io::Error::other("file operation target is missing")),
                    );
                    return;
                };
                if let Err(error) = remove_path(&target_path) {
                    let operation = self.pending_file_operation.clone().expect("pending operation missing");
                    self.finish_pending_file_operation(operation, Err(error));
                    return;
                }
                operation.overwritten_count += 1;
                self.run_pending_file_operation();
            }
            FileConflictResolution::Skip => {
                if let Some(operation) = self.pending_file_operation.as_mut() {
                    operation.advance_after_skip();
                }
                self.run_pending_file_operation();
            }
            FileConflictResolution::Rename => {
                let Some(operation) = self.pending_file_operation.as_mut() else {
                    self.dialog = None;
                    return;
                };
                let default_name = operation
                    .current_source()
                    .and_then(|path| path.file_name())
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                operation.rename_from_conflict = true;
                self.pending_file_operation_name_input.set_text(default_name);
                self.dialog = Some(Dialog::FileOperationName);
                self.status = "Choose a new file name".to_string();
            }
        }
    }

    fn finish_pending_file_operation(
        &mut self,
        operation: PendingFileOperation,
        result: io::Result<()>,
    ) {
        self.clear_file_operation_request();

        match result {
            Ok(()) => {
                let moved_targets = operation.completed_targets();
                for browser_index in 0..self.visible_browser_count() {
                    self.refresh_browser_pane(browser_index);
                }
                self.clear_browser_selection(operation.browser_index);
                self.assign_focus(Self::focus_for_browser_index(operation.browser_index));
                if let Some(target_path) = moved_targets.first() {
                    self.select_entry_for_path(operation.browser_index, target_path);
                }
                self.schedule_browser_preview();
                self.status = operation.success_message();
            }
            Err(error) => {
                self.status = format!("{} failed: {error}", operation.kind.label());
            }
        }
    }

    fn handle_editor_key(&mut self, key: KeyEvent) {
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

    fn select_previous_entry(&mut self, browser_index: usize) {
        let selected = self.browsers[browser_index].selected_entry;
        self.set_selected_entry(browser_index, selected.saturating_sub(1));
    }

    fn select_next_entry(&mut self, browser_index: usize) {
        let browser = &self.browsers[browser_index];
        if !browser.entries.is_empty() {
            self.set_selected_entry(
                browser_index,
                (browser.selected_entry + 1).min(browser.entries.len() - 1),
            );
        }
    }

    fn page_up_browser(&mut self, browser_index: usize) {
        let page_size = self.browser_page_size(browser_index);
        let selected = self.browsers[browser_index].selected_entry;
        self.set_selected_entry(browser_index, selected.saturating_sub(page_size));
    }

    fn page_down_browser(&mut self, browser_index: usize) {
        if self.browsers[browser_index].entries.is_empty() {
            return;
        }

        let page_size = self.browser_page_size(browser_index);
        let browser = &self.browsers[browser_index];
        self.set_selected_entry(
            browser_index,
            (browser.selected_entry + page_size).min(browser.entries.len() - 1),
        );
    }

    fn browser_page_size(&self, browser_index: usize) -> usize {
        (self.geometry.browser_inners[browser_index].height as usize).max(1)
    }

    fn set_selected_entry(&mut self, browser_index: usize, index: usize) {
        let browser = &mut self.browsers[browser_index];
        let clamped = index.min(browser.entries.len().saturating_sub(1));
        if clamped != browser.selected_entry {
            browser.selected_entry = clamped;
            self.schedule_browser_preview();
        }
    }

    fn select_entry_for_path(&mut self, browser_index: usize, path: &Path) {
        let Some(index) = self.browsers[browser_index]
            .entries
            .iter()
            .position(|entry| entry.path == path)
        else {
            return;
        };

        self.browsers[browser_index].selected_entry = index;
    }

    fn refresh_buffers_after_save(&mut self) {
        let saved_path = self.editor.path().map(Path::to_path_buf);

        for browser_index in 0..BROWSER_PANE_COUNT {
            self.refresh_browser_pane(browser_index);
            if let Some(path) = saved_path.as_deref() {
                self.select_entry_for_path(browser_index, path);
            }
        }

        if self.focus_browser_index().is_some() {
            self.schedule_browser_preview();
        }
        self.request_full_redraw();
    }

    fn schedule_browser_preview(&mut self) {
        self.browser_preview_due_at = Some(Instant::now() + BROWSER_PREVIEW_DELAY);
    }

    fn select_entry_at(&mut self, browser_index: usize, row: u16) {
        let browser = &self.browsers[browser_index];
        if browser.entries.is_empty() {
            return;
        }

        let visible_rows = self.geometry.browser_inners[browser_index].height as usize;
        let start = browser
            .selected_entry
            .saturating_sub(visible_rows.saturating_sub(1));
        let clicked = start + row.saturating_sub(self.geometry.browser_inners[browser_index].y) as usize;

        if clicked < browser.entries.len() {
            let label = browser.entries[clicked].label.clone();
            self.set_selected_entry(browser_index, clicked);
            self.status = format!("Selected {label}");
            self.assign_focus(Self::focus_for_browser_index(browser_index));
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

    fn browser_divider_at(&self, column: u16, row: u16) -> Option<usize> {
        for browser_index in 0..self.visible_browser_count() {
            if !contains_y(self.geometry.browser_areas[browser_index], row) {
                continue;
            }

            let right_edge = self.geometry.browser_areas[browser_index]
                .x
                .saturating_add(self.geometry.browser_areas[browser_index].width.saturating_sub(1));
            if column == right_edge || column == right_edge.saturating_add(1) {
                return Some(browser_index);
            }
        }

        None
    }

    fn request_unsaved_action(&mut self, action: PendingUnsavedAction) {
        let action_label = match &action {
            PendingUnsavedAction::Quit => "exit",
            PendingUnsavedAction::Focus(_) => "change pane",
            PendingUnsavedAction::OpenPath { .. } => "open another file",
        };
        self.pending_unsaved_action = Some(action);
        self.dialog = Some(Dialog::SaveFile);
        self.status = format!("Save file before {action_label}: {}", self.current_file_label());
    }

    fn confirm_save_file_dialog(&mut self, save: bool) -> Action {
        if save && !self.save_current() {
            return Action::None;
        }

        self.dialog = None;
        match self.pending_unsaved_action.take() {
            Some(PendingUnsavedAction::Quit) => Action::Quit,
            Some(PendingUnsavedAction::Focus(focus)) => {
                self.apply_focus_change(focus);
                Action::None
            }
            Some(PendingUnsavedAction::OpenPath { path, browser_index }) => {
                self.apply_open_path(browser_index, path);
                Action::None
            }
            None => Action::None,
        }
    }

    fn resize_browser_pane(&mut self, divider_index: usize, column: u16) {
        let desktop = self.geometry.desktop_inner;
        if desktop.width == 0 {
            return;
        }

        let visible_browsers = self.visible_browser_count() as u16;
        let max_width = desktop
            .width
            .saturating_sub(MIN_EDITOR_PANE_WIDTH)
            .checked_div(visible_browsers)
            .unwrap_or(1)
            .max(1);
        let min_width = MIN_BROWSER_PANE_WIDTH.min(max_width);
        let relative = column.saturating_sub(desktop.x).saturating_add(1);
        let width = if self.secondary_browser_enabled && divider_index == 1 {
            (relative / 2).clamp(min_width, max_width)
        } else {
            relative.clamp(min_width, max_width)
        };

        self.browser_pane_width = width;
        self.status = format!("Files pane: {width} columns");
    }

    pub fn visible_browser_count(&self) -> usize {
        if self.secondary_browser_enabled { 2 } else { 1 }
    }

    pub fn browser_entries(&self, browser_index: usize) -> &[ProjectEntry] {
        &self.browsers[browser_index].entries
    }

    pub fn browser_selected_entry(&self, browser_index: usize) -> usize {
        self.browsers[browser_index].selected_entry
    }

    fn refresh_browser_pane(&mut self, browser_index: usize) {
        let browser = &mut self.browsers[browser_index];
        browser.entries = list_directory(&browser.dir);
        browser.selected_paths.retain(|path| browser.entries.iter().any(|entry| entry.path == *path));
        if browser.selected_entry >= browser.entries.len() {
            browser.selected_entry = browser.entries.len().saturating_sub(1);
        }
    }

    fn clear_browser_selection(&mut self, browser_index: usize) {
        self.browsers[browser_index].selected_paths.clear();
    }

    fn focus_browser_index(&self) -> Option<usize> {
        if self.editor_only_mode {
            return None;
        }

        match self.focus {
            Focus::BrowserPrimary => Some(0),
            Focus::BrowserSecondary if self.secondary_browser_enabled => Some(1),
            Focus::BrowserSecondary | Focus::Editor => None,
        }
    }

    fn focus_for_browser_index(browser_index: usize) -> Focus {
        match browser_index {
            0 => Focus::BrowserPrimary,
            _ => Focus::BrowserSecondary,
        }
    }

    fn browser_inner_at(&self, column: u16, row: u16) -> Option<usize> {
        (0..self.visible_browser_count())
            .find(|&index| contains(self.geometry.browser_inners[index], column, row))
    }

    fn browser_area_at(&self, column: u16, row: u16) -> Option<usize> {
        (0..self.visible_browser_count())
            .find(|&index| contains(self.geometry.browser_areas[index], column, row))
    }
}

impl FileOperationKind {
    fn label(self) -> &'static str {
        match self {
            FileOperationKind::Copy => "copy",
            FileOperationKind::Move => "move",
            FileOperationKind::Delete => "delete",
        }
    }

    fn confirm_title(self, source_parent: Option<&Path>, target_parent: Option<&Path>, count: usize) -> &'static str {
        let plural = count > 1;
        match self {
            FileOperationKind::Copy if plural => "Copy selected files?",
            FileOperationKind::Copy => "Copy selected file?",
            FileOperationKind::Move if !plural && source_parent == target_parent => "Rename selected file?",
            FileOperationKind::Move if plural => "Move selected files?",
            FileOperationKind::Move => "Move selected file?",
            FileOperationKind::Delete if plural => "Delete selected files?",
            FileOperationKind::Delete => "Delete selected file?",
        }
    }

    fn name_prompt_title(self) -> &'static str {
        match self {
            FileOperationKind::Copy => "Copy selected file as",
            FileOperationKind::Move => "Rename selected file to",
            FileOperationKind::Delete => "Delete selected file",
        }
    }
}

impl PendingFileOperation {
    fn single_source(&self) -> Option<&Path> {
        match self.sources.as_slice() {
            [source] => Some(source.as_path()),
            _ => None,
        }
    }

    fn source_parent(&self) -> Option<&Path> {
        self.single_source().and_then(Path::parent)
    }

    fn target_name(&self) -> Option<&Path> {
        self.target_name.as_deref()
    }

    fn current_source(&self) -> Option<&Path> {
        self.sources.get(self.current_index).map(PathBuf::as_path)
    }

    fn current_target_path(&self) -> Option<PathBuf> {
        let source = self.current_source()?;
        let target_dir = self.target_dir.as_deref()?;
        let target_name = self
            .target_name()
            .or_else(|| source.file_name().map(Path::new))?;
        Some(target_dir.join(target_name))
    }

    fn advance_after_success(&mut self, target_path: Option<PathBuf>) {
        if let Some(target_path) = target_path {
            self.completed_targets.push(target_path);
        }
        self.current_index += 1;
        self.rename_from_conflict = false;
        self.target_name = None;
    }

    fn advance_after_skip(&mut self) {
        self.skipped_count += 1;
        self.current_index += 1;
        self.rename_from_conflict = false;
        self.target_name = None;
    }

    fn completed_targets(&self) -> Vec<PathBuf> {
        self.completed_targets.clone()
    }

    fn conflict_title(&self) -> &'static str {
        match self.kind {
            FileOperationKind::Copy => "Copy target exists",
            FileOperationKind::Move => "Move target exists",
            FileOperationKind::Delete => "Target exists",
        }
    }

    fn result_paths(&self) -> Vec<PathBuf> {
        match self.kind {
            FileOperationKind::Delete => Vec::new(),
            FileOperationKind::Copy | FileOperationKind::Move => {
                let Some(target_dir) = self.target_dir.as_deref() else {
                    return Vec::new();
                };
                if self.sources.len() == 1 {
                    let Some(name) = self
                        .target_name()
                        .or_else(|| self.sources.first().and_then(|path| path.file_name()).map(Path::new))
                    else {
                        return Vec::new();
                    };
                    vec![target_dir.join(name)]
                } else {
                    self.sources
                        .iter()
                        .filter_map(|source| source.file_name().map(|name| target_dir.join(name)))
                        .collect()
                }
            }
        }
    }

    fn success_message(&self) -> String {
        let count = match self.kind {
            FileOperationKind::Delete => self.sources.len().saturating_sub(self.skipped_count),
            FileOperationKind::Copy | FileOperationKind::Move => self.completed_targets.len(),
        };
        match self.kind {
            FileOperationKind::Copy => format!(
                "Copy stats: copied {}, overwritten {}, skipped {}",
                count, self.overwritten_count, self.skipped_count
            ),
            FileOperationKind::Move => format!(
                "Move stats: moved {}, overwritten {}, skipped {}",
                count, self.overwritten_count, self.skipped_count
            ),
            FileOperationKind::Delete => {
                let mut message = format!("Deleted {count} files");
                if self.skipped_count > 0 {
                    message.push_str(&format!(", skipped {}", self.skipped_count));
                }
                message
            }
        }
    }
}

fn try_process_pending_file_operation_step(
    operation: &mut PendingFileOperation,
) -> io::Result<FileOperationStep> {
    let Some(source) = operation.current_source().map(Path::to_path_buf) else {
        return Ok(FileOperationStep::Done);
    };

    match operation.kind {
        FileOperationKind::Delete => {
            remove_path(&source)?;
            operation.advance_after_success(None);
            Ok(if operation.current_index >= operation.sources.len() {
                FileOperationStep::Done
            } else {
                FileOperationStep::Continue
            })
        }
        FileOperationKind::Copy | FileOperationKind::Move => {
            let Some(target_path) = operation.current_target_path() else {
                return Err(io::Error::other(match operation.kind {
                    FileOperationKind::Copy => "copy target is missing",
                    FileOperationKind::Move => "move target is missing",
                    FileOperationKind::Delete => unreachable!(),
                }));
            };

            if target_path.exists() {
                return Ok(FileOperationStep::Conflict);
            }

            match operation.kind {
                FileOperationKind::Copy => copy_path(&source, &target_path)?,
                FileOperationKind::Move => move_path(&source, &target_path)?,
                FileOperationKind::Delete => unreachable!(),
            }

            operation.advance_after_success(Some(target_path));
            Ok(if operation.current_index >= operation.sources.len() {
                FileOperationStep::Done
            } else {
                FileOperationStep::Continue
            })
        }
    }
}

fn copy_path(source: &Path, target: &Path) -> io::Result<()> {
    let metadata = fs::metadata(source)?;
    if metadata.is_dir() {
        fs::create_dir(target)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let child_source = entry.path();
            let child_target = target.join(entry.file_name());
            copy_path(&child_source, &child_target)?;
        }
        Ok(())
    } else {
        fs::copy(source, target)?;
        Ok(())
    }
}

fn move_path(source: &Path, target: &Path) -> io::Result<()> {
    match fs::rename(source, target) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::CrossesDevices => {
            copy_path(source, target)?;
            remove_path(source)
        }
        Err(error) => Err(error),
    }
}

fn remove_path(path: &Path) -> io::Result<()> {
    let metadata = fs::metadata(path)?;
    if metadata.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
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

fn char_to_byte_index(value: &str, char_index: usize) -> usize {
    value
        .char_indices()
        .nth(char_index)
        .map(|(byte_index, _)| byte_index)
        .unwrap_or(value.len())
}

fn expand_tilde_path(value: &str) -> Result<PathBuf, String> {
    if value == "~" {
        return env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| "Home directory is not available".to_string());
    }

    if let Some(stripped) = value.strip_prefix("~/") {
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| "Home directory is not available".to_string())?;
        return Ok(home.join(stripped));
    }

    Ok(PathBuf::from(value))
}

fn handle_text_input_key(input: &mut TextInputState, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Backspace => input.delete_left(),
        KeyCode::Delete => input.delete_right(),
        KeyCode::Left => input.move_left(),
        KeyCode::Right => input.move_right(),
        KeyCode::Home => input.move_home(),
        KeyCode::End => input.move_end(),
        KeyCode::Char(character)
            if !key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::ALT) =>
        {
            input.insert_char(character);
        }
        _ => {}
    }
    Action::None
}

fn byte_to_char_index(value: &str, byte_index: usize) -> usize {
    value[..byte_index].chars().count()
}

fn find_regex_match(
    lines: &[String],
    start_row: usize,
    start_col: usize,
    regex: &Regex,
) -> Option<(usize, usize, usize)> {
    if lines.is_empty() {
        return None;
    }

    let start_row = start_row.min(lines.len() - 1);
    for row in start_row..lines.len() {
        let row_start = if row == start_row { start_col } else { 0 };
        if let Some((match_start, match_end)) = find_regex_match_in_line(&lines[row], row_start, None, regex) {
            return Some((row, match_start, match_end));
        }
    }

    for row in 0..=start_row {
        let row_end = if row == start_row { Some(start_col) } else { None };
        if let Some((match_start, match_end)) = find_regex_match_in_line(&lines[row], 0, row_end, regex) {
            return Some((row, match_start, match_end));
        }
    }

    None
}

fn find_regex_match_in_line(
    line: &str,
    start_col: usize,
    end_col: Option<usize>,
    regex: &Regex,
) -> Option<(usize, usize)> {
    let start_byte = char_to_byte_index(line, start_col);
    let end_byte = end_col
        .map(|column| char_to_byte_index(line, column))
        .unwrap_or(line.len());

    if start_byte >= end_byte {
        return None;
    }

    let haystack = &line[start_byte..end_byte];
    let matched = regex.find_iter(haystack).find(|matched| matched.start() != matched.end())?;
    let match_start = start_byte + matched.start();
    let match_end = start_byte + matched.end();
    Some((
        byte_to_char_index(line, match_start),
        byte_to_char_index(line, match_end),
    ))
}

fn shell_quote(path: &Path) -> String {
    let value = path.to_string_lossy();
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn format_tool_invocation(path: &Path, invocation: ToolInvocation) -> (String, String) {
    match invocation {
        ToolInvocation::Cargo { subcommand } => {
            let command = format!("cargo {subcommand}");
            (command.clone(), command)
        }
        ToolInvocation::Program {
            program,
            args,
            pass_file_path,
        } => {
            let mut command_parts = vec![program.to_string()];
            command_parts.extend(args.iter().map(|arg| (*arg).to_string()));
            if pass_file_path {
                command_parts.push(shell_quote(path));
                let command = command_parts.join(" ");
                let description = format!("{} {}", [program].into_iter().chain(args.iter().copied()).collect::<Vec<_>>().join(" "), path.display());
                (
                    command,
                    description,
                )
            } else {
                let command = command_parts.join(" ");
                (command.clone(), command)
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        path::PathBuf,
        process,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{Action, App, Dialog, Focus, PendingUnsavedAction};

    static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock before unix epoch")
                .as_nanos();
            let counter = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = env::temp_dir().join(format!(
                "trubo-app-tests-{}-{unique}-{counter}",
                process::id()
            ));
            fs::create_dir_all(&path).expect("create temp test directory");
            Self { path }
        }

        fn path(&self) -> &PathBuf {
            &self.path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn dirty_focus_change_prompts_to_save() {
        let test_dir = TestDir::new();
        let path = test_dir.path().join("demo.txt");
        fs::write(&path, "hello").expect("write test file");

        let mut app = App::new(test_dir.path().to_path_buf());
        app.refresh_browser();
        app.open_path(path);
        app.editor.insert_char('!');

        app.focus_browser();

        assert_eq!(app.dialog, Some(Dialog::SaveFile));
        assert_eq!(app.focus, Focus::Editor);
        assert!(matches!(
            app.pending_unsaved_action,
            Some(PendingUnsavedAction::Focus(Focus::BrowserPrimary))
        ));
    }

    #[test]
    fn confirming_open_without_saving_replaces_dirty_buffer() {
        let test_dir = TestDir::new();
        let first = test_dir.path().join("first.txt");
        let second = test_dir.path().join("second.txt");
        fs::write(&first, "alpha").expect("write first file");
        fs::write(&second, "beta").expect("write second file");

        let mut app = App::new(test_dir.path().to_path_buf());
        app.refresh_browser();
        app.open_path(first.clone());
        app.editor.insert_char('!');

        app.open_path(second.clone());

        assert_eq!(app.dialog, Some(Dialog::SaveFile));
        assert!(matches!(
            app.pending_unsaved_action,
            Some(PendingUnsavedAction::OpenPath {
                path: ref pending_path,
                browser_index: 0,
            }) if pending_path == &second
        ));
        assert_eq!(app.confirm_save_file_dialog(false), Action::None);
        assert_eq!(app.dialog, None);
        assert_eq!(app.pending_unsaved_action, None);
        let canonical_second = second.canonicalize().expect("canonical second path");
        assert_eq!(app.editor.path(), Some(canonical_second.as_path()));
        assert_eq!(app.editor.lines(), &["beta".to_string()]);
        assert!(!app.editor.is_dirty());
    }

    #[test]
    fn confirm_open_file_dialog_creates_and_opens_missing_file() {
        let test_dir = TestDir::new();
        let nested = test_dir.path().join("nested");
        fs::create_dir(&nested).expect("create nested directory");

        let mut app = App::new(test_dir.path().to_path_buf());
        app.refresh_browser();
        app.request_open_file_dialog();
        app.open_file_input.set_text("nested/fresh.txt".to_string());

        app.confirm_open_file_dialog();

        let target = nested.join("fresh.txt");
        assert!(target.exists());
        let canonical_target = target.canonicalize().expect("canonical target path");
        assert_eq!(app.editor.path(), Some(canonical_target.as_path()));
        assert_eq!(app.browsers[0].dir, nested.canonicalize().expect("canonical nested path"));
        assert_eq!(app.focus, Focus::Editor);
    }

    #[test]
    fn open_file_dialog_accepts_absolute_paths() {
        let test_dir = TestDir::new();
        let target = test_dir.path().join("absolute.txt");

        let mut app = App::new(test_dir.path().to_path_buf());
        app.refresh_browser();
        app.request_open_file_dialog();
        app.open_file_input.set_text(target.display().to_string());

        app.confirm_open_file_dialog();

        let canonical_target = target.canonicalize().expect("canonical target path");
        assert_eq!(app.editor.path(), Some(canonical_target.as_path()));
    }

    #[test]
    fn open_file_dialog_expands_tilde_paths() {
        let test_dir = TestDir::new();
        let home = test_dir.path().join("home");
        fs::create_dir(&home).expect("create home directory");

        let original_home = env::var_os("HOME");
        unsafe {
            env::set_var("HOME", &home);
        }

        let mut app = App::new(test_dir.path().to_path_buf());
        app.refresh_browser();
        app.request_open_file_dialog();
        app.open_file_input.set_text("~/tilde.txt".to_string());

        app.confirm_open_file_dialog();

        let target = home.join("tilde.txt");
        let canonical_target = target.canonicalize().expect("canonical target path");
        assert_eq!(app.editor.path(), Some(canonical_target.as_path()));

        unsafe {
            match original_home {
                Some(value) => env::set_var("HOME", value),
                None => env::remove_var("HOME"),
            }
        }
    }
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