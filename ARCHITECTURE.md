# trubo Architecture

This agent-generated file is meant to help undertand the code base. 
It currently uses absolute line numbers for references to individual functions. In case the code changes, these will need to be updated or else they will point to wrong places in code.

## Overview

 `trubo` is a single-process, single-threaded terminal text editor with an integrated file browser.

At a high level, the application is organized into these layers:

1. [src/main.rs](src/main.rs): process startup, terminal setup/teardown, event loop, and top-level key dispatch.
2. [src/app.rs](src/app.rs): application controller and session state. This is the coordination layer between input, the editor model, browser state, clipboard actions, dialogs, and menu behavior.
3. [src/editor.rs](src/editor.rs): core text buffer model, cursor movement, selection, editing operations, soft-wrap-aware navigation, and single-step undo.
4. [src/project.rs](src/project.rs): file browser model for listing directories as `ProjectEntry` values.
5. [src/clipboard.rs](src/clipboard.rs): platform-specific clipboard integration via external OS commands.
6. [src/ui.rs](src/ui.rs): terminal rendering, pane layout, menus, dialogs, help screen, and editor syntax coloring.

The design is intentionally compact. There is no background worker, no async runtime, no message queue, and no persistent document model outside the in-memory `Editor` owned by `App`.

## External Libraries

The application depends on two crates:

- `crossterm`: terminal raw mode, alternate screen, mouse capture, bracketed paste, and input events (`Event`, `KeyEvent`, `MouseEvent`). It is the input and terminal-control layer.
- `ratatui`: layout and rendering primitives (`Terminal`, `Frame`, `Block`, `Paragraph`, `List`, styles, spans, and layout rectangles). It is the presentation layer.

Standard library dependencies are also important architecturally:

- `std::fs` and `std::path`: file loading, saving, and directory traversal.
- `std::process::Command`: clipboard shell commands and `cargo run` / `cargo build` integration.
- `std::time::{Duration, Instant}`: event polling cadence and delayed browser preview.

## Architectural Style

The codebase is closest to a lightweight MVC/MVU hybrid:

- Model:
	- `Editor` in [src/editor.rs](src/editor.rs)
	- `ProjectEntry` in [src/project.rs](src/project.rs)
- View:
	- `ui::draw` and related helpers in [src/ui.rs](src/ui.rs)
- Controller / application session state:
	- `App` in [src/app.rs](src/app.rs)

This separation is useful, but not strict:

- `Editor` is a reasonably self-contained document model.
- `App` owns both domain state and UI/session state such as focus, menus, dialog visibility, pane sizes, geometry, status text, and delayed preview timing.
- [src/ui.rs](src/ui.rs) is mostly a view, but it also updates derived layout state inside `App` (`geometry`) and calls `editor.set_viewport(...)` during rendering. So rendering is not a pure read-only projection.

That makes the architecture pragmatic rather than strictly layered.

## Module Responsibilities

### [src/main.rs](src/main.rs)

[src/main.rs](src/main.rs) owns the runtime shell of the application.

Key responsibilities:

- Parse the startup target (`parse_args`).
- Ensure stdout is an interactive terminal.
- Enter and leave raw terminal mode (`setup_terminal`, `restore_terminal`).
- Construct `App` and bootstrap the browser/editor state.
- Run the main event loop (`run`).
- Route top-level shortcuts (`handle_key`).

Important functions:

- [main()](src/main.rs#L28): bootstraps the terminal session and initial `App` state.
- [parse_args()](src/main.rs#L71), [setup_terminal()](src/main.rs#L97), and [restore_terminal()](src/main.rs#L109): startup and terminal lifecycle helpers.
- [run(...)](src/main.rs#L120): repeatedly
	- advances delayed browser preview,
	- redraws the UI,
	- polls for input,
	- dispatches events.
- [handle_key(...)](src/main.rs#L147): top-level command routing before falling through to `App`-level handlers.

Primary dependencies:

- Depends on `App` from [src/app.rs](src/app.rs).
- Uses `crossterm` for terminal and event handling.
- Uses `ui::draw(...)` from [src/ui.rs](src/ui.rs) for rendering.

### [src/app.rs](src/app.rs)

[src/app.rs](src/app.rs) is the central coordinator.

The `App` struct is the main application state container. It owns:

- browser state:
	- current browser directory,
	- listed entries,
	- selected entry,
	- browser pane width,
	- delayed preview timer.
- editor state:
	- the active `Editor` model.
- UI/session state:
	- focused pane,
	- menu visibility and selection,
	- help dialog,
	- confirmation dialog,
	- selection mode,
	- status text,
	- geometry for hit-testing mouse input.

Key responsibilities:

- Keyboard handling inside the active pane.
- Mouse handling for menus, browser clicks, editor selection, scroll, and pane resizing.
- File open/save behavior.
- Clipboard copy/cut/paste orchestration.
- Running `cargo` commands.
- Delayed browser preview.
- Quit confirmation policy.

Important types:

- `Menu`, `MenuItem`, `MenuAction`: declarative menu model.
- `Focus`: active pane (`Browser` or `Editor`).
- `Dialog`: current modal dialog.
- `Geometry`: rectangles populated during rendering and consumed by mouse logic.

Important functions and their dependencies:

- `App::new(...)` in [src/app.rs](src/app.rs#L183)
	- initializes app/session state.
- [refresh_browser()](src/app.rs#L213)
	- depends on `project::list_directory(...)`.
	- also schedules delayed preview.
- [tick_browser_preview()](src/app.rs#L242)
	- depends on `Editor::open(...)` and internal focus/dirty checks.
	- does not steal focus from the browser.
- [open_selected_file()](src/app.rs#L301)
	- uses selected `ProjectEntry` from browser state.
	- delegates to `open_path(...)` or directory navigation.
- [open_path(...)](src/app.rs#L316)
	- depends on `Editor::open(...)`.
- [save_current()](src/app.rs#L336)
	- depends on `Editor::save()`.
- [handle_active_key(...)](src/app.rs#L391)
	- routes to `handle_browser_key(...)` or `handle_editor_key(...)`.
- [open_menu()](src/app.rs#L444), [handle_menu_key()](src/app.rs#L463), and [perform_menu_action()](src/app.rs#L539)
	- implement the menu controller path.
- [copy_selection()](src/app.rs#L563), [cut_selection()](src/app.rs#L575), [paste_from_clipboard()](src/app.rs#L590), [paste_text()](src/app.rs#L597), and [undo_last_edit()](src/app.rs#L610)
	- bridge controller actions into editor and clipboard behavior.
- [handle_mouse()](src/app.rs#L622)
	- translates mouse input into browser/editor/menu behavior.
- [handle_browser_key(...)](src/app.rs#L736) and [handle_editor_key(...)](src/app.rs#L758)
	- depends heavily on `Editor` movement and edit methods.

This file is the most controller-like part of the system.

### [src/editor.rs](src/editor.rs)

[src/editor.rs](src/editor.rs) contains the text buffer model.

The `Editor` struct owns:

- file identity (`path`),
- text storage (`Vec<String>`),
- cursor location,
- viewport offsets for soft-wrapped rendering,
- selection anchor,
- single-step undo snapshot,
- dirty flag.

Key responsibilities:

- Open/save text files.
- Cursor motion across lines and wrapped visual rows.
- Selection creation and querying.
- Text insertion, deletion, newline splitting, line duplication, and line deletion.
- Converting selection into extracted or deleted text.
- Maintaining viewport visibility.
- Single-step undo.

Important functions:

- Construction / persistence:
	- [scratch()](src/editor.rs#L55)
	- [open(...)](src/editor.rs#L72)
	- [save()](src/editor.rs#L95)
- Selection:
	- [begin_selection()](src/editor.rs#L144)
	- [select_to(...)](src/editor.rs#L148)
	- `selection_bounds()` and `selection_range_for_line(...)`
	- [selected_text()](src/editor.rs#L194)
	- `cut_selection()`
- Navigation:
	- `move_left/right/up/down()`
	- `extend_left/right/up/down()`
	- `home/end/page_up/page_down()`
	- internal `move_cursor(...)`, `step_up(...)`, `step_down(...)`
- Editing:
	- [insert_char(...)](src/editor.rs#L295)
	- [insert_text(...)](src/editor.rs#L306)
	- [insert_newline()](src/editor.rs#L341)
	- [backspace()](src/editor.rs#L354)
	- [delete()](src/editor.rs#L380)
	- [delete_line()](src/editor.rs#L406)
	- [duplicate_line()](src/editor.rs#L421)
- Undo:
	- `capture_undo_state()`
	- [undo()](src/editor.rs#L431)

Important internal design details:

- Text is stored as lines, not as a rope or gap buffer.
- Cursor columns are character-based, with helper conversion to byte offsets when mutating UTF-8 strings.
- Vertical movement is aware of soft wrapping through `viewport_cols` and `wrapped_rows(...)`.
- The selection model is anchor/cursor based.
- Undo is intentionally simple: one snapshot only.

Primary dependencies:

- Depends on `app::read_to_string(...)` from [src/app.rs](src/app.rs) for lossy UTF-8 file loading.
- Otherwise mostly self-contained.

### [src/project.rs](src/project.rs)

[src/project.rs](src/project.rs) is the file browser model.

Key responsibilities:

- Represent a browser row as `ProjectEntry`.
- Distinguish parent directory, directory, and file entries.
- Enumerate and sort directory contents.

Important functions:

- [list_directory(...)](src/project.rs#L29)
	- reads the directory,
	- injects `..` parent entry when available,
	- separates directories from files,
	- sorts both groups,
	- returns directories first, then files.

This module is intentionally small and stateless.

### [src/clipboard.rs](src/clipboard.rs)

[src/clipboard.rs](src/clipboard.rs) provides OS-specific clipboard integration by shelling out to platform tools rather than using a clipboard crate.

Platform strategy:

- macOS: `pbcopy` / `pbpaste`
- Windows: PowerShell `Set-Clipboard` / `Get-Clipboard -Raw`
- Unix/Linux: tries `wl-copy` / `wl-paste`, then `xclip`, then `xsel`

Important functions:

- [set_text(...)](src/clipboard.rs#L6)
- [get_text(...)](src/clipboard.rs#L37)
- `write_to_command(...)`
- `read_from_command(...)`

This is effectively an infrastructure adapter used by `App`.

### [src/ui.rs](src/ui.rs)

[src/ui.rs](src/ui.rs) renders the application state using `ratatui`.

Key responsibilities:

- Global layout and pane sizing.
- Menu bar and menu dropdown rendering.
- File browser pane rendering.
- Log/status pane rendering.
- Editor rendering with line numbers, selection highlighting, and keyword/identifier coloring.
- Help and modal dialog rendering.
- Writing geometry back into `App` so mouse hit-testing can use the same rectangles.

Important functions:

- [draw(...)](src/ui.rs#L97)
	- top-level render entry point.
- [draw_desktop(...)](src/ui.rs#L342)
	- splits browser and editor panes.
- [draw_browser(...)](src/ui.rs#L364)
	- renders file list and browser hints.
- [draw_browser_log(...)](src/ui.rs#L443)
	- renders status text and current directory.
- [draw_editor(...)](src/ui.rs#L482)
	- renders wrapped lines, line numbers, selection, highlighting, and cursor.
- [draw_help(...)](src/ui.rs#L649) / [draw_dialog(...)](src/ui.rs#L705)
	- render modal overlays.
- [render_editor_segment(...)](src/ui.rs#L850)
	- maps text segments into styled spans.
- [tokenize_line(...)](src/ui.rs#L919)
	- performs simple keyword/identifier classification for syntax highlighting.

Primary dependencies:

- Reads `App`, `Editor`, and layout constants from [src/app.rs](src/app.rs).
- Uses `ratatui` extensively.

## Link Notes

This document uses relative Markdown links such as [src/main.rs](src/main.rs), which open reliably in VS Code.

For functions inside Rust files, VS Code Markdown does not provide a native symbol anchor format like it does for Markdown headings. That means file links are the reliable option.

Best-effort deep links can be written with line anchors such as `src/main.rs#L120`, but those are less portable than plain file links and may depend on how the link is opened.

## Core State Model

The three most important state aggregates are:

### 1. Application session state: `App`

`App` is the source of truth for user session behavior.

Examples:

- Which pane is focused.
- What file is selected in the browser.
- What dialog or menu is open.
- What message is shown in the status/log pane.
- What `Editor` instance is active.

### 2. Document model: `Editor`

`Editor` is the source of truth for editable content.

Examples:

- Buffer lines.
- Cursor position.
- Selection anchor.
- Dirty state.
- Undo snapshot.
- Wrapped viewport offsets.

### 3. Derived view geometry: `Geometry`

`Geometry` is not business state. It is derived during render and then reused for mouse hit-testing.

That is a notable architectural choice:

- render pass computes rectangles,
- input pass uses those exact rectangles,
- no separate layout engine exists outside [src/ui.rs](src/ui.rs).

## Model / View / Controller Aspects

### Model

- `Editor`
- `ProjectEntry`

### View

- [src/ui.rs](src/ui.rs)
- theme values, layout splits, widgets, syntax highlighting presentation

### Controller

- `App` in [src/app.rs](src/app.rs)
- `main::handle_key(...)` in [src/main.rs](src/main.rs)

### Notable crossover

The view is not fully passive because `ui::draw(...)` in [src/ui.rs](src/ui.rs) mutates `App.geometry` and calls `editor.set_viewport(...)`.

This means:

- drawing computes information needed later by input handling,
- the render step helps maintain editor scrolling/visibility state,
- the UI and controller are somewhat coupled.

That is acceptable for a small TUI application, but it is worth knowing if the codebase grows.

## Key Control Flows

### Startup

Pseudo-code:

```text
main():
	startup_target = parse_args()
	ensure stdout is a terminal
	canonicalize target path
	terminal = setup_terminal()

	app = App::new(target)
	app.refresh_browser()

	if target is file:
		app.open_path(target)
	else:
		app.status = "Browsing ..."

	run(terminal, app)
	restore_terminal(terminal)
```

Dependencies:

- `parse_args()`
- `setup_terminal()`
- `App::new(...)`
- `App::refresh_browser()`
- `App::open_path(...)`
- `run(...)`

### Main Event Loop

Pseudo-code:

```text
run(app):
	loop forever:
		app.tick_browser_preview()
		ui::draw(app)

		if event::poll(120ms):
			event = event::read()

			match event:
				Key   => if handle_key(app, key) == Quit: break
				Mouse => if app.handle_mouse(mouse) == Quit: break
				Paste => app.paste_text(text)
				Resize => ignore
```

Key architectural properties:

- The program is event-driven.
- There is no separate update thread.
- Delayed browser preview is implemented by polling time in the same loop.

### Top-Level Key Dispatch

Pseudo-code:

```text
handle_key(app, key):
	if help is open:
		close help
		return

	if dialog is open:
		return app.handle_dialog_key(key)

	if menu is open:
		return app.handle_menu_key(key)

	if Ctrl+Q:
		return app.request_quit()

	if Ctrl+Space:
		app.toggle_selection_mode()
		return

	if Ctrl+<shortcut>:
		dispatch copy/cut/paste/undo/save/focus/open/run/build
		return

	dispatch function keys or active-pane key handling
```

This split keeps [src/main.rs](src/main.rs) responsible for global key semantics while `App` handles pane-local behavior.

### Browser Navigation and Delayed Preview

Pseudo-code:

```text
browser selection changes:
	app.set_selected_entry(index)
	schedule preview for now + 200ms

tick_browser_preview():
	if preview not due yet: return
	if focus != browser: return
	if menu/help/dialog open: return
	if editor is dirty: return
	if selected entry is directory: return
	if selected file already open: return

	editor = Editor::open(selected_file)
	app.editor = editor
	app.status = "Previewed ..."
```

This is a good example of controller logic living in `App` rather than in the render layer.

### Editor Input Flow

Pseudo-code:

```text
App::handle_editor_key(key):
	if Alt+X => editor.delete_line()
	if Alt+U => editor.duplicate_line()

	selecting = Shift held OR selection_mode enabled

	match key:
		arrows/home/end/page => movement or extend movement
		backspace/delete     => destructive edits
		enter                => insert_newline
		printable char       => insert_char
```

This keeps the `App` layer responsible for key interpretation and the `Editor` layer responsible for document mutation.

### Save / Quit Flow

Pseudo-code:

```text
request_quit():
	close menus/help
	dirty = editor.is_dirty()
	selection = editor.has_selection()

	if dirty or selection:
		show ConfirmExit dialog
		return Action::None
	else:
		return Action::Quit
```

This shows that quit policy is session/controller logic, not editor logic.

## Important Dependencies Between Modules

The main dependency graph is:

```text
main.rs
	-> app.rs
	-> ui.rs
	-> crossterm
	-> ratatui

app.rs
	-> editor.rs
	-> project.rs
	-> clipboard.rs
	-> std::process::Command
	-> crossterm event types
	-> ratatui::layout::Rect

editor.rs
	-> app::read_to_string(...)
	-> std::fs

ui.rs
	-> app.rs types/state
	-> ratatui

project.rs
	-> std::fs

clipboard.rs
	-> std::process::Command
```

Noteworthy coupling points:

- [src/editor.rs](src/editor.rs) depends on [src/app.rs](src/app.rs) for file loading via `read_to_string(...)`. That is slightly inverted from a strict layering perspective.
- [src/ui.rs](src/ui.rs) both consumes and mutates state (`App.geometry`, editor viewport).
- [src/app.rs](src/app.rs) is the most coupled module because it orchestrates almost everything.

## Rendering Model

Rendering is immediate-mode.

Each loop iteration:

1. `ui::draw(...)` recomputes layout.
2. Pane rectangles are stored into `App.geometry`.
3. The editor viewport is recalculated from current dimensions.
4. Widgets are drawn from current state only.

There is no retained scene graph beyond `ratatui` widget data assembled for the current frame.

The editor rendering pipeline is roughly:

```text
draw_editor():
	compute visible text width from pane size
	editor.set_viewport(rows, cols)

	for each visible visual row:
		map from file row + wrap segment
		compute selection coverage
		tokenize line
		build styled spans

	render paragraph
	render wrap markers on border
	place terminal cursor if editor is focused
```

## File and Text Handling Notes

- Files are loaded with lossy UTF-8 conversion via `String::from_utf8_lossy(...)`.
- Saving joins lines with `"\n"`; original line-ending style is not preserved.
- Editing operations work on `Vec<String>` line storage.
- Column logic is character-based, but actual string mutation uses character-to-byte conversion helpers.
- Soft wrapping is a view-aware concern shared between `Editor` and `ui.rs`.

## Current Strengths of the Design

- Small and easy to trace end-to-end.
- Clear runtime loop.
- Editor core is reasonably self-contained.
- `App` centralizes interaction logic, making user-visible behavior easy to locate.
- Rendering logic is localized in one file.

## Current Tradeoffs

- `App` is large and acts as a god object for session control.
- Rendering is not fully pure because it feeds geometry and viewport state back into the model/controller.
- `Editor` depends on `app.rs` for file reading, which is a mild layering leak.
- Undo is only single-step.
- Clipboard behavior depends on external OS commands being present.

## If the Codebase Grows

Natural future refactoring seams would be:

- extract browser-specific controller logic from `App` in [src/app.rs](src/app.rs) into a browser module,
- extract menu/dialog state handling from `App`,
- move file I/O helpers fully into [src/editor.rs](src/editor.rs) or a dedicated persistence module,
- make [src/ui.rs](src/ui.rs) more purely view-oriented by separating geometry computation from mutation,
- replace single-step undo with an undo stack.
