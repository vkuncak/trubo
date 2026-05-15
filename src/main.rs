mod app;
mod clipboard;
mod editor;
mod file_types;
mod project;
mod ui;

use std::{
    env,
    ffi::OsString,
    fs,
    io::{self, IsTerminal, Stdout},
    path::PathBuf,
    time::Duration,
};

use app::{Action, App, Focus};
use crossterm::{
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, KeyCode, KeyEvent, KeyModifiers,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

type TerminalUi = Terminal<CrosstermBackend<Stdout>>;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let target = match parse_args()? {
        Startup::Help => {
            print_usage();
            return Ok(());
        }
        Startup::Open(path) => path,
    };

    if !io::stdout().is_terminal() {
        return Err("trubo must be run in an interactive terminal".into());
    }

    if !target.exists() {
        fs::write(&target, "")?;
    }
    let target = target.canonicalize().unwrap_or(target);

    let mut terminal = setup_terminal()?;
    let mut app = App::new(target.clone());
    app.refresh_browser();
    if target.is_file() {
        app.open_path(target);
    } else {
        app.status = format!("Browsing {}", app.browser_label(0));
    }

    let result = run(&mut terminal, &mut app);
    restore_terminal(&mut terminal)?;

    if let Err(error) = result {
        eprintln!("trubo: {error}");
        std::process::exit(1);
    }

    Ok(())
}

enum Startup {
    Help,
    Open(PathBuf),
}

fn parse_args() -> Result<Startup, Box<dyn std::error::Error>> {
    let args = env::args_os().skip(1).collect::<Vec<_>>();

    match args.as_slice() {
        [] => Ok(Startup::Open(env::current_dir()?)),
        [flag] if is_help_flag(flag) => Ok(Startup::Help),
        [path] => Ok(Startup::Open(PathBuf::from(path))),
        _ => Err("usage: trubo [FILE_OR_DIRECTORY]".into()),
    }
}

fn is_help_flag(value: &OsString) -> bool {
    value == "-h" || value == "--help"
}

fn print_usage() {
    println!("trubo - retro DOS-style terminal text editor");
    println!();
    println!("Usage:");
    println!("  trubo [FILE_OR_DIRECTORY]");
    println!();
    println!("Start with a directory to browse, or a file to open it directly.");
    println!();
    println!("Important keys:");
    println!("  F1       Help");
    println!("  Ctrl+Q   Quit trubo");
    println!("  Ctrl+B   Toggle editor-only mode");
    println!("  `        Toggle dual-pane mode from a files pane");
    println!("  F4/Tab   Cycle focus between panes");
    println!();
    println!("Documentation:");
    println!("  doc/GUIDE.md          User guide and workflows");
    println!("  doc/ARCHITECTURE.md   Codebase architecture");
}

fn setup_terminal() -> io::Result<TerminalUi> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste
    )?;
    Terminal::new(CrosstermBackend::new(stdout))
}

fn restore_terminal(terminal: &mut TerminalUi) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableBracketedPaste,
        DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()
}

fn run(terminal: &mut TerminalUi, app: &mut App) -> io::Result<()> {
    loop {
        app.tick_browser_preview();
        if app.take_full_redraw_request() {
            terminal.clear()?;
        }
        terminal.draw(|frame| ui::draw(frame, app))?;

        if event::poll(Duration::from_millis(120))? {
            match event::read()? {
                Event::Key(key) => {
                    if handle_key(app, key) == Action::Quit {
                        break;
                    }
                }
                Event::Mouse(mouse) => {
                    if app.handle_mouse(mouse) == Action::Quit {
                        break;
                    }
                }
                Event::Paste(text) => app.paste_text(&text),
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }

    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) -> Action {
    if app.help_open {
        app.help_open = false;
        return Action::None;
    }

    if app.dialog.is_some() {
        return app.handle_dialog_key(key);
    }

    if app.menu_open {
        return app.handle_menu_key(key);
    }

    if key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q'))
    {
        return app.request_quit();
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char(' ') {
        app.compute_selected_entry_size();
        return Action::None;
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Left => app.focus_browser(),
            KeyCode::Right => app.focus_editor(),
            KeyCode::Home | KeyCode::PageUp if app.focus == Focus::Editor => app.editor.file_home(),
            KeyCode::End | KeyCode::PageDown if app.focus == Focus::Editor => app.editor.file_end(),
            KeyCode::Char('l') | KeyCode::Char('L') => app.request_full_redraw(),
            KeyCode::Char('t') | KeyCode::Char('T') => app.toggle_selected_browser_entry(),
            KeyCode::Char('c') | KeyCode::Char('C') => app.copy_selection(),
            KeyCode::Char('x') | KeyCode::Char('X') => app.cut_selection(),
            KeyCode::Char('v') | KeyCode::Char('V') => app.paste_from_clipboard(),
            KeyCode::Char('k') | KeyCode::Char('K') if app.focus == Focus::Editor => {
                app.editor.delete_line()
            }
            KeyCode::Char('z') | KeyCode::Char('Z') => app.undo_last_edit(),
            KeyCode::Char('y') | KeyCode::Char('Y') => app.redo_last_edit(),
            KeyCode::Insert => app.copy_selection(),
            KeyCode::Char('s') | KeyCode::Char('S') => {
                app.save_current();
            }
            KeyCode::Char('f') | KeyCode::Char('F') => app.request_search(),
            KeyCode::Char('o') | KeyCode::Char('O') => app.request_open_file_dialog(),
            KeyCode::Char('r') | KeyCode::Char('R') => app.run_current_target(),
            KeyCode::Char('b') | KeyCode::Char('B') => app.toggle_editor_only_mode(),
            _ => {}
        }
        return Action::None;
    }

    match key.code {
        KeyCode::Insert
            if key.modifiers.is_empty()
                && matches!(app.focus, Focus::BrowserPrimary | Focus::BrowserSecondary) =>
        {
            app.toggle_selected_browser_entry()
        }
        KeyCode::Insert if key.modifiers.contains(KeyModifiers::SHIFT) => app.paste_from_clipboard(),
        KeyCode::Delete if key.modifiers.contains(KeyModifiers::SHIFT) => app.cut_selection(),
        KeyCode::F(1) => app.help_open = true,
        KeyCode::F(2) => {
            app.save_current();
        }
        KeyCode::F(3) => app.open_selected_file(),
        KeyCode::F(4) => app.toggle_focus(),
        KeyCode::F(5) => app.request_copy_selected_entry(),
        KeyCode::F(6) => app.request_move_selected_entry(),
        KeyCode::F(7) => app.request_new_directory(),
        KeyCode::F(8) => app.request_delete_selected_entry(),
        KeyCode::F(9) => app.build_current_target(),
        KeyCode::F(10) => app.toggle_menu(),
        KeyCode::Tab => app.toggle_focus(),
        KeyCode::BackTab => app.toggle_focus(),
        _ => app.handle_active_key(key),
    }

    Action::None
}
