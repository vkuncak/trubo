# trubo

Textual Rust Unicode-Based Operator

trubo is a retro TUI text editor written in Rust.

Status: experimental nostalgia project. It edits text files, browses
directories, and keeps the UI intentionally loud.

It's based on TRUST ( https://github.com/wojtczyk/trust ) from which this was forked.

## Run

```sh
cargo run -- /path/to/file-or-directory
```

If no path is supplied, trubo opens the current directory. If a file path is
supplied, trubo opens that file directly and uses its parent directory for the
browser pane.

## Features

Editing files including Shift-Cursor selection, copy, paste and line wrap.

Loading and saving files.

Navigating directories with quick preview.

Illustration of syntax coloring.

## Keys

- `F1`: help
- `F2` / `Ctrl+S`: save
- `F3` / `Ctrl+O`: open selected file
- `Backspace`: go to the parent directory in the project pane
- `F4` / `Tab` / `Ctrl+F`: cycle focus
- `F10`: open the menu bar
- `Ctrl+C`: copy selected text
- `Ctrl+V`: paste clipboard text
- `Ctrl+X`: cut selected text
- `Esc` / `Ctrl+Q`: quit
- `Alt+X`: delete line
- `Alt+U`: duplicate line
- `Shift+Navigation`: select text

## Menus

- `F10` opens the menu bar.
- Left/right arrows switch menus.
- Up/down arrows move through a dropdown.
- `Enter` activates the highlighted menu item.
- `Esc` closes the menu.
- Mouse clicks on the menu bar and dropdown items work too.
- `Window` switches between the file browser and editor panes.

## Mouse

- Click inside the editor to move the cursor.
- Drag inside the editor to select text.
- Click inside the file pane to open files or navigate directories.
- Click inside either pane to focus it.
- Drag the vertical divider between the browser and editor panes to resize them.
- Scroll inside the browser or editor pane to move through content.

The file pane lists directories and all regular files. trubo opens files as
text regardless of extension.
