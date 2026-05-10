# trubo Architecture

## Overview

`trubo` is a single-process terminal editor with an integrated file browser, optional dual browser panes, and a renderer-driven TUI built on `crossterm` and `ratatui`.

The application stays intentionally small:

- no background workers
- no async runtime
- no persistent project database
- no separate command bus

Most behavior is coordinated directly through `App`, which owns both session state and the active `Editor` instance.

## High-Level Structure

The runtime is split across a small set of modules:

1. [src/main.rs](../src/main.rs) boots the process, parses CLI arguments, configures the terminal, runs the event loop, and handles top-level shortcuts.
2. [src/app.rs](../src/app.rs) owns the application session state and implements the controller logic for panes, dialogs, menus, browser actions, search, clipboard orchestration, and run/build actions.
3. [src/editor.rs](../src/editor.rs) implements the text buffer, cursor movement, selection, editing operations, soft-wrap-aware navigation, and undo/redo history.
4. [src/project.rs](../src/project.rs) provides directory listing and directory-tree preview generation.
5. [src/file_types.rs](../src/file_types.rs) maps file extensions and shebangs to syntax keywords, line-comment detection, and run/build commands.
6. [src/clipboard.rs](../src/clipboard.rs) bridges to the operating system clipboard via external tools.
7. [src/ui.rs](../src/ui.rs) renders the full terminal interface, including menus, browser panes, dialogs, help, status, syntax coloring, and editor selection highlighting.

## Runtime Flow

Startup begins in [src/main.rs](../src/main.rs):

- `parse_args()` resolves either `--help` or a startup path.
- If the target path does not exist, the process creates an empty file there.
- The terminal enters raw mode, alternate screen, mouse capture, and bracketed paste mode.
- `App::new(...)` initializes browser state from the startup path.
- `refresh_browser()` populates the primary browser.
- If the startup path is a file, `open_path(...)` loads it immediately. Otherwise the app starts in directory browsing mode.

The main loop repeatedly:

1. advances delayed browser preview via `tick_browser_preview()`
2. redraws the whole UI via `ui::draw(...)`
3. polls for keyboard, mouse, paste, and resize events
4. dispatches events into `App`

Run and build actions are not executed inside the current TUI. Instead, `App` launches an external interactive terminal window and asks it to run the configured command for the current file type.

## State Model

### App State

`App` in [src/app.rs](../src/app.rs) is the main state container. It owns:

- two browser pane models (`browsers`)
- mode flags for `secondary_browser_enabled` and `editor_only_mode`
- the active `Editor`
- focus state across `BrowserPrimary`, `BrowserSecondary`, and `Editor`
- menu and dialog state
- selection-mode toggle state
- status text
- shared browser pane width
- geometry captured during rendering for later mouse hit-testing
- pending state for new-directory, copy/move/delete, and regex-search dialogs
- delayed preview timing

This makes `App` both the controller and the source of truth for most session behavior.

### Editor State

`Editor` in [src/editor.rs](../src/editor.rs) owns:

- optional file path
- line-based text storage (`Vec<String>`)
- optional binary-size metadata when the source file is not valid UTF-8 text
- cursor position
- soft-wrap viewport offsets
- selection anchor
- bounded undo and redo stacks
- dirty flag

Files are read as raw bytes and then decoded with `String::from_utf8_lossy(...)`. That means `trubo` can open arbitrary files, but binary content is treated as lossy text rather than as a structured binary view.

### Derived Geometry

`Geometry` is produced by [src/ui.rs](../src/ui.rs) during drawing and then reused by [src/app.rs](../src/app.rs) for mouse behavior. This is a deliberate coupling point:

- render computes rectangles for panes, menu items, and editor text area
- input logic reuses those exact rectangles for clicks, drags, scrolling, and divider resizing

The view layer is therefore not purely read-only.

## Layout And Modes

`trubo` supports three practical layouts:

### Editor-only mode

- enabled with `Ctrl+B`
- hides all browser panes
- forces focus to the editor
- preserves browser state so the user can return to it later

### Single-pane mode

- shows one browser pane plus the editor
- the primary browser also includes the status/log area
- selecting entries in the browser schedules a delayed preview into the editor when the editor is not dirty and no modal UI is open

### Dual-pane mode

- enabled by toggling the second browser pane
- shows two independent browser panes plus the editor
- both browser panes share the same configured width
- `F4` cycles across both browser panes and the editor
- copy and move actions use the inactive browser pane as the destination directory when appropriate

The layout is rendered in [src/ui.rs](../src/ui.rs), while mode transitions and focus behavior are controlled in [src/app.rs](../src/app.rs).

## Browser Subsystem

The browser model is intentionally simple.

`project::list_directory(...)` in [src/project.rs](../src/project.rs):

- adds a `..` parent entry when a parent exists
- separates directories and files
- sorts both groups alphabetically
- returns directories before files

Each browser pane stores:

- current directory
- current entry list
- selected row

Browser behavior in `App` includes:

- open file or enter directory on `Enter`
- go to parent directory on `Backspace`
- refresh directory on `R`
- delayed preview of selected files or directory trees
- page and home/end navigation
- mouse selection, opening, and scrolling

Directory preview is not a live tree widget. `project::directory_subtree_lines(...)` generates a textual tree preview and loads it into a scratch `Editor` instance.

## File Operations

File operations are controller-level features in [src/app.rs](../src/app.rs).

Supported operations:

- copy selected entry
- move selected entry
- delete selected entry
- create sub-directory

Important behavior:

- copy requires dual-pane mode so there is a destination directory to copy into
- move works in both single-pane and dual-pane mode
- when moving or copying within the same parent directory, the user is prompted for a new name
- delete can remove either files or directories
- cross-device move falls back to copy-then-delete

The browser is refreshed after successful operations, and focus returns to the initiating pane.

## Editor Model

The editor is line-based and optimized for straightforward terminal editing rather than for very large files.

Key responsibilities in [src/editor.rs](../src/editor.rs):

- open and save files
- Unicode-aware cursor motion and mutation
- anchor-based selection across lines
- insertion, deletion, newline splitting, and selection replacement
- line deletion
- soft-wrap-aware vertical navigation
- viewport maintenance
- undo and redo with bounded history

Notable design details:

- cursor columns are tracked in character positions, with conversions to byte offsets only when mutating UTF-8 strings
- page-up, page-down, up, and down operate on wrapped visual rows rather than only on logical file lines
- the editor always keeps the cursor visible by recomputing viewport offsets
- paste, regex search, undo, and redo all route focus back to the editor

## Search

Regex search is implemented in [src/app.rs](../src/app.rs), not in the editor model.

The flow is:

1. open the regex dialog
2. compile the pattern with `regex::Regex`
3. search from the current cursor position to the end of the file
4. wrap around to the beginning if needed
5. move the cursor to the match and select the matched span
6. center the view around the result

Search state is stored in `App` so the most recent pattern can be reused.

## File Types, Highlighting, Run, And Build

[src/file_types.rs](../src/file_types.rs) centralizes language-specific configuration.

For each known file type it can provide:

- syntax keywords for highlighting
- a line-comment pattern for comment coloring
- a run command
- a build command

Current configured families include:

- Rust
- shell/bash
- Python
- Scala
- TypeScript and TSX
- Lean
- OCaml and `.mli`

The UI consumes the keyword and comment metadata to render simple syntax coloring. The app consumes run/build metadata to launch external terminals with either cargo commands or direct program invocations.

## Clipboard Integration

[src/clipboard.rs](../src/clipboard.rs) uses platform-native command-line tools instead of a Rust clipboard crate.

Platform strategy:

- macOS: `pbcopy` and `pbpaste`
- Windows: PowerShell clipboard commands
- Unix/Linux: `wl-copy` and `wl-paste`, then `xclip`, then `xsel`

Clipboard failures are reported through `App.status` instead of crashing the application.

## UI Layer

[src/ui.rs](../src/ui.rs) is responsible for the entire presentation layer.

It renders:

- top header or menu bar
- browser panes
- status/log panel in the primary browser pane
- editor contents with line numbers and selection highlighting
- menu dropdowns
- help overlay
- modal dialogs for save-on-quit, search, new directory, file operations, and about

The UI also:

- computes browser and editor rectangles
- clamps browser widths against minimum editor and browser sizes
- applies language-aware token styling
- places the terminal cursor in the correct wrapped visual row

The most notable coupling in the codebase is here: rendering updates `App.geometry` and drives `Editor::set_viewport(...)`, so draw-time logic directly affects later input behavior.

## Architectural Character

The codebase is closest to a pragmatic MVC-style split:

- model: `Editor`, `ProjectEntry`, file-type metadata
- view: [src/ui.rs](../src/ui.rs)
- controller/session layer: `App` plus the top-level dispatch in [src/main.rs](../src/main.rs)

The separation is useful but intentionally loose. `App` contains both domain behavior and UI state, while the renderer performs some state mutation required by mouse interaction and soft-wrap viewport management.

That tradeoff is reasonable at the current scale: the code remains compact, easy to trace, and centered around direct data flow rather than framework abstraction.