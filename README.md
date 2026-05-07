# trubo

trubo is a retro TUI text editor inspired by classic blue-screen DOS tools.

Status: experimental nostalgia project. It edits text files, browses
directories, and keeps the UI intentionally loud.

It's based on TRUST from which this was forked.

## FAQ

**Why?**  
Because Rust deserves a blue-screen IDE from the olden days and someone had to do this.

**Does it save my files?**  
Yes. Use `F2` or `Ctrl+S`. trubo marks dirty buffers with `*` in the editor title. Still, this is more of a fun project so use at your own risk.

**Is this affiliated with any classic DOS IDE vendor?**  
No. trubo is an independent nostalgia project inspired by classic DOS development environments.

## Run

```sh
cargo run -- /path/to/file-or-directory
```

If no path is supplied, trubo opens the current directory. If a file path is
supplied, trubo opens that file directly and uses its parent directory for the
browser pane.

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
