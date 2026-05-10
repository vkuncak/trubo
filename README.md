# trubo

Text (Rust, Unicode, Basic) Operator

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

## Documentation

User-facing usage is documented in [doc/GUIDE.md](doc/GUIDE.md).

Architecture and module structure are documented in [doc/ARCHITECTURE.md](doc/ARCHITECTURE.md).

That documentation is the source of truth for:

- operating modes: editor-only, single-pane, dual-pane
- current keyboard and mouse controls
- browser file operations
- run/build behavior
- editor features and limitations

## Quick Start

The most important keys are:

- `F1`: in-app help
- `Ctrl+Q`: quit the editor
- `Ctrl+B`: toggle editor-only mode
- `` ` ``: toggle dual-pane mode
- `F4` or `Tab`: cycle pane focus
- `F2` or `Ctrl+S`: save
- `Ctrl+F`: regex search

For the full guide, see [doc/GUIDE.md](doc/GUIDE.md).

## Notes

- The README stays intentionally short to avoid drifting from the actual UI behavior.
- If the README and the guide ever disagree, treat [doc/GUIDE.md](doc/GUIDE.md) as authoritative.

The file pane lists directories and all regular files. trubo opens files as
lossy text regardless of extension.
