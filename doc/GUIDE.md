# trubo User Guide

## What trubo is

`trubo` is a terminal text editor with a built-in file browser. It is designed for direct keyboard use, but it also supports mouse selection, scrolling, menu clicks, and pane resizing.

Start it with either a file or a directory:

```sh
cargo run -- /path/to/file-or-directory
```

If you start with a directory, `trubo` opens in browsing mode. If you start with a file, it opens that file and uses its parent directory as the browser location.

## Core operating modes

### Editor-only mode

Use `Ctrl+B` to toggle editor-only mode.

In this mode:

- the file browser is hidden
- focus is forced to the editor
- editing shortcuts keep working normally
- browser state is kept in memory so you can return to it later

Use this when you want the maximum amount of space for text editing.

### Single-pane mode

Single-pane mode is the default browser layout.

In this mode you have:

- one files pane on the left
- the editor on the right
- a small log/status area below the files pane

Typical workflow:

1. Move through the files pane with `Up` and `Down`.
2. Open the selected file or directory with `Enter` or `F3`.
3. Return to the parent directory with `Backspace`.
4. Switch between the files pane and the editor with `F4`, `Tab`, `Ctrl+Left`, or `Ctrl+Right`.

When the files pane is focused, `trubo` also previews the selected entry after a short delay if the current editor buffer is not dirty:

- files are previewed directly in the editor area
- directories are previewed as a text tree

This is useful for scanning a directory without opening each file permanently.

### Dual-pane mode

Use `` ` `` to toggle the second files pane. You can also use the Window menu.

In dual-pane mode you have:

- files pane 1
- files pane 2
- the editor

Each files pane tracks its own directory and selection. `F4` cycles through all visible panes.

Dual-pane mode is most useful for file management:

- `F5` copies the selected file or directory from the active files pane to the other files pane
- `F6` moves the selected file or directory to the other files pane
- `F7` creates a new directory in the active files pane
- `F8` deletes the selected file or directory

Important detail:

- copy requires dual-pane mode
- move works in either single-pane or dual-pane mode
- if the source and destination are the same directory, `trubo` turns copy or move into copy-as or rename behavior by asking for a new name

## Keyboard essentials

### Global keys

- `F1`: open help
- `F2` or `Ctrl+S`: save current file
- `F3` or `Ctrl+O`: open selected file or directory from the active files pane
- `F4`, `Tab`, `Shift+Tab`: cycle pane focus
- `F9`: build current file
- `F10`: open the menu bar
- `Ctrl+B`: toggle editor-only mode
- `Ctrl+Q`: quit
- `Ctrl+L`: force a full redraw of the screen

### Browser keys

- `Up` and `Down`: move selection
- `PageUp` and `PageDown`: move by a page
- `Home` and `End`: jump to first or last entry
- `Enter`: open selected file or enter selected directory
- `Backspace`: go to the parent directory
- `R`: refresh the current directory listing
- `` ` ``: toggle dual-pane mode while a files pane is active

### Editor keys

- `Arrows`: move the cursor
- `Home` and `End`: move to start or end of line
- `PageUp` and `PageDown`: move by wrapped screen rows
- `Ctrl+Home` or `Ctrl+PageUp`: jump to start of file
- `Ctrl+End` or `Ctrl+PageDown`: jump to end of file
- `Enter`: insert a newline
- `Backspace` and `Delete`: remove text
- typing: insert text
- `Ctrl+K`: delete the current line

### Selection and clipboard

- hold `Shift` while moving to extend the selection
- `Ctrl+Space` toggles persistent selection mode
- `Ctrl+C` or `Ctrl+Insert`: copy selection
- `Ctrl+X` or `Shift+Delete`: cut selection
- `Ctrl+V` or `Shift+Insert`: paste

### Search, undo, redo, run

- `Ctrl+F`: regex search
- `Ctrl+Z`: undo
- `Ctrl+Y`: redo
- `Ctrl+R`: run current file

## Mouse support

You can use the mouse for the main interactions:

- click inside a files pane to select and open an entry
- scroll inside a files pane to move through entries
- click inside the editor to move the cursor
- drag inside the editor to create or extend a selection
- scroll inside the editor to move by pages
- drag the divider next to a files pane to resize it
- click the menu bar and menu items to operate the menus

## File browsing and file management

The browser lists directories first and files second. A `..` entry appears when there is a parent directory.

File operations work on the selected browser entry:

- copy
- move
- delete
- create directory

These operations can target files or directories. Deletes are recursive for directories.

After a successful operation, the browser refreshes automatically.

## Running and building files

`trubo` can run or build the current file based on its detected type. On macOS, the command is launched in a separate Terminal window.

Before run or build:

- the current file is saved automatically if it is dirty
- the command is chosen from the file type configuration

Current language support includes:

- Rust: `cargo run` and `cargo build`
- shell or bash: `bash` and `bash -n`
- Python: `python3` and `python3 -m py_compile`
- TypeScript or TSX: `tsx` and `tsc --noEmit`
- Scala: `scala` and `scalac`
- Lean: `lean`
- OCaml: `ocaml` and `ocamlc -c`

If a file type has no configured command, `trubo` reports that in the status area.

## Key editor features

### Soft-wrapped editing

Long lines wrap to the current editor width. Cursor movement, page movement, and selection work across wrapped screen rows rather than behaving as if each line were a single visual row.

### Unicode-aware text editing

The editor tracks positions by characters instead of raw bytes, so normal editing and selection work correctly with Unicode text.

### Selection across lines

Selections can span multiple lines. Copy, cut, paste, delete, and search results all use the same selection model.

### Undo and redo history

The editor keeps bounded undo and redo stacks, so you can step backward and forward through recent edits.

### Regex search with wraparound

Search is regular-expression based. It starts from the current cursor position, wraps around to the start of the file if needed, and selects the next match it finds.

### Syntax coloring

The editor applies lightweight syntax coloring using file extension or shebang detection. Highlighting is intentionally simple and currently focuses on:

- keywords
- identifier styling
- line comments when the file type has a configured line-comment pattern

### Clipboard integration

Copy, cut, and paste use the system clipboard, not an internal clipboard only.

### Binary-tolerant loading

If you open a non-text file, `trubo` still loads it as lossy text so you can inspect its contents. The header switches from a line count to a byte count for that buffer.

## Menus and dialogs

Use `F10` to open the menu bar.

Inside the menu system:

- `Left` and `Right` switch menus
- `Up` and `Down` move through items
- `Enter` activates the selected item
- `Esc` closes the menu
- highlighted first letters work as menu hotkeys

Dialogs are used for:

- save-on-quit
- regex search
- new directory creation
- copy or move naming
- file operation confirmation
- about

## Practical workflows

### Quick edit of one file

1. Start `trubo` with a file path.
2. Edit in the right pane.
3. Save with `Ctrl+S`.
4. Use `Ctrl+B` if you want a larger editing surface.

### Browse and preview before opening

1. Start `trubo` on a directory.
2. Keep focus in the files pane.
3. Move with `Up` and `Down`.
4. Wait briefly to preview files or directory trees.
5. Press `Enter` when you want to open the selection fully.

### Copy files between directories

1. Turn on dual-pane mode with `` ` ``.
2. Navigate the left and right files panes to the source and destination directories.
3. Focus the source pane.
4. Press `F5` to copy or `F6` to move.

### Search inside a file

1. Press `Ctrl+F`.
2. Enter a regular expression.
3. Press `Enter`.
4. The next match is selected and centered in the editor.