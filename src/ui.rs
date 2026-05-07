use crate::app::{
    App, Dialog, Focus, Geometry, MENUS, MIN_BROWSER_PANE_WIDTH, MIN_EDITOR_PANE_WIDTH,
    MenuGeometry,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

const DOS_BLUE: Color = Color::Rgb(0, 0, 170);
const DOS_CYAN: Color = Color::Rgb(0, 170, 170);
const DOS_GRAY: Color = Color::Rgb(170, 170, 170);
const DOS_DARK_GRAY: Color = Color::Rgb(85, 85, 85);
const DOS_YELLOW: Color = Color::Rgb(255, 255, 85);
const DOS_WHITE: Color = Color::Rgb(255, 255, 255);
const DOS_BLACK: Color = Color::Rgb(0, 0, 0);
const DOS_RED: Color = Color::Rgb(170, 0, 0);
const DOS_BRIGHT_RED: Color = Color::Rgb(255, 85, 85);

pub fn draw(frame: &mut Frame, app: &mut App) {
    let root = frame.area();
    frame.render_widget(Block::default().style(Style::default().bg(DOS_BLUE)), root);

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(8), Constraint::Length(1)])
        .split(root);

    app.geometry = Geometry {
        root,
        menu_area: vertical[0],
        ..Geometry::default()
    };

    draw_menu(frame, vertical[0], app);
    draw_desktop(frame, vertical[1], app);
    draw_status(frame, vertical[2], app);

    if app.menu_open {
        draw_menu_dropdown(frame, app);
    }

    if app.help_open {
        draw_help(frame, centered(root, 72, 15));
    } else if let Some(dialog) = app.dialog {
        draw_dialog(frame, dialog, centered(root, 60, 10));
    }
}

fn draw_menu(frame: &mut Frame, area: Rect, app: &mut App) {
    app.geometry.menu = MenuGeometry::default();
    let mut spans = vec![Span::styled(
        " trubo ",
        Style::default()
            .fg(DOS_YELLOW)
            .bg(DOS_DARK_GRAY)
            .add_modifier(Modifier::BOLD),
    )];
    let mut x = area.x + " trubo ".len() as u16;

    for (index, menu) in MENUS.iter().enumerate() {
        let active = app.menu_open && app.active_menu == index;
        let title = menu.title;
        let hot = &title[..1];
        let rest = &title[1..];
        let menu_width = title.len() as u16 + 1;
        app.geometry.menu.bar_items[index] = Rect {
            x,
            y: area.y,
            width: menu_width,
            height: 1,
        };
        x = x.saturating_add(menu_width);

        let style = if active {
            Style::default()
                .fg(DOS_BLACK)
                .bg(DOS_CYAN)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(DOS_BLACK).bg(DOS_GRAY)
        };
        let hot_style = if active {
            Style::default()
                .fg(DOS_RED)
                .bg(DOS_CYAN)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(DOS_RED)
                .bg(DOS_GRAY)
                .add_modifier(Modifier::BOLD)
        };

        spans.push(Span::styled(" ", style));
        spans.push(Span::styled(hot, hot_style));
        spans.push(Span::styled(rest, style));
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().fg(DOS_BLACK).bg(DOS_GRAY)),
        area,
    );
}

fn draw_menu_dropdown(frame: &mut Frame, app: &mut App) {
    let menu = MENUS[app.active_menu];
    let bar = app.geometry.menu.bar_items[app.active_menu];
    let width = menu
        .items
        .iter()
        .map(|item| {
            if item.separator {
                8
            } else {
                item.label.len() + item.shortcut.len() + 5
            }
        })
        .max()
        .unwrap_or(14)
        .max(menu.title.len() + 4) as u16;
    let height = menu.items.len() as u16 + 2;
    let max_x = app.geometry.root.x + app.geometry.root.width.saturating_sub(width);
    let area = Rect {
        x: bar.x.min(max_x),
        y: app.geometry.menu_area.y.saturating_add(1),
        width,
        height: height.min(app.geometry.root.height.saturating_sub(1)),
    };
    app.geometry.menu.dropdown = Some(area);

    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(DOS_YELLOW).bg(DOS_CYAN))
        .style(Style::default().fg(DOS_BLACK).bg(DOS_GRAY));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = menu
        .items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            if item.separator {
                Line::from(Span::styled(
                    "-".repeat(inner.width as usize),
                    Style::default().fg(DOS_DARK_GRAY).bg(DOS_GRAY),
                ))
            } else {
                let active = index == app.active_menu_item;
                let style = if active {
                    Style::default()
                        .fg(DOS_WHITE)
                        .bg(DOS_BLUE)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(DOS_BLACK).bg(DOS_GRAY)
                };
                let hot_style = if active {
                    Style::default()
                        .fg(DOS_BRIGHT_RED)
                        .bg(DOS_BLUE)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(DOS_RED)
                        .bg(DOS_GRAY)
                        .add_modifier(Modifier::BOLD)
                };
                let shortcut_gap = inner
                    .width
                    .saturating_sub(item.label.len() as u16)
                    .saturating_sub(item.shortcut.len() as u16)
                    .saturating_sub(2) as usize;
                let hot = &item.label[..1];
                let rest = &item.label[1..];
                Line::from(vec![
                    Span::styled(" ", style),
                    Span::styled(hot, hot_style),
                    Span::styled(rest, style),
                    Span::styled(
                        format!("{}{} ", " ".repeat(shortcut_gap), item.shortcut),
                        style,
                    ),
                ])
            }
        })
        .collect::<Vec<_>>();

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(DOS_BLACK).bg(DOS_GRAY)),
        inner,
    );
}

fn draw_desktop(frame: &mut Frame, area: Rect, app: &mut App) {
    let desktop = area.inner(Margin {
        horizontal: 1,
        vertical: 0,
    });
    app.browser_pane_width = clamp_browser_width(desktop.width, app.browser_pane_width);
    app.geometry.desktop_inner = desktop;

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(app.browser_pane_width),
            Constraint::Min(MIN_EDITOR_PANE_WIDTH),
        ])
        .split(desktop);

    app.geometry.browser_area = columns[0];
    app.geometry.editor_area = columns[1];
    draw_browser(frame, columns[0], app);
    draw_editor(frame, columns[1], app);
}

fn draw_browser(frame: &mut Frame, area: Rect, app: &mut App) {
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(6)])
        .split(area);

    let title = format!(" Files: {} ", app.browser_label());
    let block = retro_block(
        &title,
        app.focus == Focus::Browser,
        "Enter Open",
        Some("Backspace Up"),
    );
    let inner = block.inner(split[0]);
    app.geometry.browser_inner = inner;
    frame.render_widget(block, split[0]);

    if app.entries.is_empty() {
        frame.render_widget(
            Paragraph::new(Text::from(vec![
                Line::from("Directory is empty."),
                Line::from(""),
                Line::from("Use arrows to browse."),
            ]))
            .style(Style::default().fg(DOS_WHITE).bg(DOS_BLUE))
            .alignment(Alignment::Center),
            inner,
        );
        return;
    }

    let height = inner.height as usize;
    let selected = app.selected_entry;
    let start = selected.saturating_sub(height.saturating_sub(1));
    let items = app
        .entries
        .iter()
        .enumerate()
        .skip(start)
        .take(height)
        .map(|(index, entry)| {
            let style = if index == selected {
                Style::default()
                    .fg(DOS_BLACK)
                    .bg(DOS_CYAN)
                    .add_modifier(Modifier::BOLD)
            } else if entry.is_directory() {
                Style::default().fg(DOS_YELLOW).bg(DOS_BLUE)
            } else {
                Style::default().fg(DOS_WHITE).bg(DOS_BLUE)
            };
            let label = if entry.is_directory() {
                format!("[D] {}", entry.label)
            } else {
                format!("    {}", entry.label)
            };
            ListItem::new(Line::from(Span::styled(
                truncate(&label, inner.width),
                style,
            )))
        })
        .collect::<Vec<_>>();

    frame.render_widget(List::new(items).style(Style::default().bg(DOS_BLUE)), inner);

    draw_browser_log(frame, split[1], app);
}

fn draw_browser_log(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Log ")
        .title_style(Style::default().fg(DOS_YELLOW).bg(DOS_BLUE))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(DOS_WHITE).bg(DOS_BLUE))
        .style(Style::default().bg(DOS_BLUE));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let log_lines = Text::from(vec![
        Line::from(Span::styled(
            truncate(&app.status, inner.width),
            Style::default().fg(DOS_WHITE).bg(DOS_BLUE),
        )),
        Line::from(Span::styled(
            truncate(&format!("Dir: {}", app.browser_label()), inner.width),
            Style::default().fg(DOS_CYAN).bg(DOS_BLUE),
        )),
    ]);

    frame.render_widget(
        Paragraph::new(log_lines).wrap(Wrap { trim: true }),
        inner,
    );
}

fn draw_editor(frame: &mut Frame, area: Rect, app: &mut App) {
    let dirty = if app.editor.is_dirty() { " *" } else { "" };
    let title = format!(" {}{} ", app.current_file_label(), dirty);
    let block = retro_block(&title, app.focus == Focus::Editor, "Edit", Some("F2 Save"));
    let inner = block.inner(area);
    app.geometry.editor_inner = inner;
    frame.render_widget(block, area);

    let line_number_width = app.editor_line_number_width();
    let text_cols = inner.width.saturating_sub(line_number_width + 1) as usize;
    let text_rows = inner.height as usize;
    app.editor.set_viewport(text_rows, text_cols);

    let mut lines = Vec::with_capacity(text_rows);
    let row_offset = app.editor.row_offset();
    let col_offset = app.editor.col_offset();

    for screen_row in 0..text_rows {
        let file_row = row_offset + screen_row;
        let mut spans = Vec::new();
        let full_width_selected = app
            .editor
            .selection_bounds()
            .is_some_and(|(start, end)| file_row > start.row && file_row < end.row);
        if let Some(line) = app.editor.lines().get(file_row) {
            let number = format!(
                "{:>width$} ",
                file_row + 1,
                width = line_number_width.saturating_sub(1) as usize
            );
            spans.push(Span::styled(
                number,
                Style::default().fg(DOS_YELLOW).bg(DOS_BLUE),
            ));

            spans.extend(render_editor_line(
                line,
                col_offset,
                text_cols,
                app.editor.selection_range_for_line(file_row),
                full_width_selected,
            ));
        } else {
            spans.push(Span::styled(
                "~".repeat(line_number_width as usize),
                Style::default().fg(DOS_DARK_GRAY).bg(DOS_BLUE),
            ));
        }
        lines.push(Line::from(spans));
    }

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(DOS_WHITE).bg(DOS_BLUE)),
        inner,
    );

    if app.focus == Focus::Editor {
        let cursor_x = inner.x
            + line_number_width
            + app.editor.cursor_col().saturating_sub(app.editor.col_offset()) as u16;
        let cursor_y = inner.y
            + app.editor.cursor_row().saturating_sub(app.editor.row_offset()) as u16;
        if cursor_x < inner.x + inner.width && cursor_y < inner.y + inner.height {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

fn draw_status(frame: &mut Frame, area: Rect, app: &App) {
    let base = Style::default()
        .fg(DOS_BLACK)
        .bg(DOS_GRAY)
        .add_modifier(Modifier::BOLD);
    let key = Style::default()
        .fg(DOS_RED)
        .bg(DOS_GRAY)
        .add_modifier(Modifier::BOLD);
    let position = format!(
        "Ln {}, Col {}",
        app.editor.cursor_row() + 1,
        app.editor.cursor_col() + 1
    );
    let selection = if app.editor.has_selection() { "  Sel" } else { "" };
    let select_mode = if app.selection_mode { "  SelMode" } else { "" };
    let suffix = format!("  {position}{selection}{select_mode} ");
    let mut line = Line::from(vec![
        Span::styled(" ", base),
        Span::styled("F1", key),
        Span::styled(" Help  ", base),
        Span::styled("F2", key),
        Span::styled(" Save  ", base),
        Span::styled("F3", key),
        Span::styled(" Open  ", base),
        Span::styled("F4", key),
        Span::styled(" Focus  ", base),
        Span::styled("F5", key),
        Span::styled(" Run  ", base),
        Span::styled("Ctrl+Sp", key),
        Span::styled(" Select  ", base),
        Span::styled("F9", key),
        Span::styled(" Build  ", base),
        Span::styled("F10", key),
        Span::styled(" Menu", base),
        Span::styled(suffix, base),
    ]);
    let width = line.width();
    if width < area.width as usize {
        line.spans
            .push(Span::styled(" ".repeat(area.width as usize - width), base));
    }
    frame.render_widget(Paragraph::new(line).style(base), area);
}

fn clamp_browser_width(total_width: u16, current: u16) -> u16 {
    if total_width <= 1 {
        return 1;
    }

    let editor_reserve = MIN_EDITOR_PANE_WIDTH.min(total_width.saturating_sub(1));
    let max_width = total_width.saturating_sub(editor_reserve).max(1);
    let min_width = MIN_BROWSER_PANE_WIDTH.min(max_width);
    current.clamp(min_width, max_width)
}

fn draw_help(frame: &mut Frame, area: Rect) {
    frame.render_widget(Clear, area);
    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(DOS_YELLOW).bg(DOS_CYAN))
        .style(Style::default().fg(DOS_BLACK).bg(DOS_GRAY));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let text = Text::from(vec![
        Line::from(vec![Span::styled(
            "trubo keys",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from("F2 Save     F3 Open selected file    F4 Cycle focus"),
        Line::from("F5 cargo run   Ctrl+Space toggle select mode   F9 cargo build"),
        Line::from("Enter opens the highlighted file or directory."),
        Line::from("Backspace goes to the parent directory."),
        Line::from("Ctrl+R Run  Ctrl+B Build  Ctrl+Q Quit"),
        Line::from("Ctrl+C Copy Ctrl+X Cut Ctrl+V Paste"),
        Line::from("Ctrl+Ins Copy  Shift+Ins Paste  Shift+Del Cut"),
        Line::from("Alt+X Delete line        Alt+U Duplicate line"),
        Line::from("Shift+Arrows/Home/End/Page extends selection."),
        Line::from("If Shift+Arrows fail, use Ctrl+Space and cursor keys."),
        Line::from("Menu: F10 opens, arrows move, Enter activates."),
        Line::from(""),
        Line::from("Mouse: click files to open, drag divider to resize,"),
        Line::from("click or drag inside the editor to move/select text."),
        Line::from(""),
        Line::from("Any file extension can be opened as text."),
        Line::from("Press any key to return."),
    ]);
    frame.render_widget(
        Paragraph::new(text)
            .style(Style::default().fg(DOS_BLACK).bg(DOS_GRAY))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true }),
        inner,
    );
}

fn draw_dialog(frame: &mut Frame, dialog: Dialog, area: Rect) {
    frame.render_widget(Clear, area);
    let title = match dialog {
        Dialog::About => " About trubo ",
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(DOS_YELLOW).bg(DOS_CYAN))
        .style(Style::default().fg(DOS_BLACK).bg(DOS_GRAY));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let text = match dialog {
        Dialog::About => Text::from(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "trubo 0.1",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from("Retro DOS-style terminal text editor"),
            Line::from("with a built-in file browser."),
            Line::from(""),
            Line::from("Press any key to return."),
        ]),
    };

    frame.render_widget(
        Paragraph::new(text)
            .style(Style::default().fg(DOS_BLACK).bg(DOS_GRAY))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        inner,
    );
}

fn retro_block<'a>(
    title: &'a str,
    active: bool,
    left: &'a str,
    right: Option<&'a str>,
) -> Block<'a> {
    let border = if active {
        Style::default()
            .fg(DOS_YELLOW)
            .bg(DOS_BLUE)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(DOS_WHITE).bg(DOS_BLUE)
    };

    let mut block = Block::default()
        .title(title)
        .title_style(Style::default().fg(DOS_YELLOW).bg(DOS_BLUE))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(border)
        .style(Style::default().bg(DOS_BLUE))
        .title_bottom(Line::from(Span::styled(
            left,
            Style::default().fg(DOS_YELLOW).bg(DOS_BLUE),
        )));

    if let Some(right) = right {
        block = block.title_bottom(
            Line::from(Span::styled(
                right,
                Style::default().fg(DOS_CYAN).bg(DOS_BLUE),
            ))
            .right_aligned(),
        );
    }

    block
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width.saturating_sub(4));
    let height = height.min(area.height.saturating_sub(2));
    Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    }
}

fn truncate(value: &str, width: u16) -> String {
    let width = width as usize;
    let mut result = value.chars().take(width).collect::<String>();
    if result.chars().count() == width && value.chars().count() > width && width > 1 {
        result.pop();
        result.push('>');
    }
    result
}

fn render_editor_line(
    line: &str,
    col_offset: usize,
    text_cols: usize,
    selection: Option<(usize, usize)>,
    full_width_selected: bool,
) -> Vec<Span<'static>> {
    let chars = line
        .chars()
        .skip(col_offset)
        .take(text_cols)
        .collect::<Vec<_>>();
    let mut spans = Vec::new();
    let mut run = String::new();
    let mut run_selected = None;

    for (screen_col, character) in chars.into_iter().enumerate() {
        let absolute_col = col_offset + screen_col;
        let selected = selection
            .map(|(start, end)| absolute_col >= start && absolute_col < end)
            .unwrap_or(false);

        if run_selected == Some(selected) || run_selected.is_none() {
            run.push(character);
            run_selected = Some(selected);
        } else {
            push_editor_run(&mut spans, &run, run_selected.unwrap_or(false));
            run.clear();
            run.push(character);
            run_selected = Some(selected);
        }
    }

    if !run.is_empty() {
        push_editor_run(&mut spans, &run, run_selected.unwrap_or(false));
    }

    if full_width_selected && text_cols > 0 {
        let rendered_cols = line.chars().skip(col_offset).take(text_cols).count();
        if rendered_cols < text_cols {
            let padding = " ".repeat(text_cols - rendered_cols);
            push_editor_run(&mut spans, &padding, true);
        }
    }

    spans
}

fn push_editor_run(spans: &mut Vec<Span<'static>>, run: &str, selected: bool) {
    let style = if selected {
        Style::default()
            .fg(DOS_BLUE)
            .bg(DOS_YELLOW)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(DOS_WHITE).bg(DOS_BLUE)
    };
    spans.push(Span::styled(run.to_string(), style));
}
