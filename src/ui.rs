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

struct Theme {
    app_background: Color,
    panel_background: Color,
    panel_border_inactive: Color,
    panel_border_active: Color,
    panel_text_primary: Color,
    panel_text_secondary: Color,
    panel_text_muted: Color,
    panel_title_text: Color,
    menu_brand_bg: Color,
    menu_brand_fg: Color,
    menu_bar_bg: Color,
    menu_bar_fg: Color,
    menu_hotkey_fg: Color,
    menu_active_bg: Color,
    menu_active_fg: Color,
    menu_active_hotkey_fg: Color,
    status_bar_bg: Color,
    status_bar_fg: Color,
    status_hotkey_fg: Color,
    selected_bg: Color,
    selected_fg: Color,
    editor_text_fg: Color,
    editor_text_bg: Color,
    editor_identifier_fg: Color,
    editor_line_number_fg: Color,
    editor_line_number_bg: Color,
    editor_selection_bg: Color,
    editor_selection_fg: Color,
}

const CURRENT_THEME: Theme = Theme {
    app_background: Color::Rgb(200, 200, 200),
    panel_background: Color::Rgb(255, 255, 255),
    panel_border_inactive: Color::Rgb(250, 250, 250),
    panel_border_active: Color::Rgb(55, 155, 0),
    panel_text_primary: Color::Rgb(200, 200, 200),
    panel_text_secondary: Color::Rgb(0, 20, 20),
    panel_text_muted: Color::Rgb(85, 85, 85),
    panel_title_text: Color::Rgb(0, 0, 80),
    menu_brand_bg: Color::Rgb(200, 200, 200),
    menu_brand_fg: Color::Rgb(255, 255, 85),
    menu_bar_bg: Color::Rgb(200, 200, 200),
    menu_bar_fg: Color::Rgb(0, 0, 0),
    menu_hotkey_fg: Color::Rgb(170, 0, 0),
    menu_active_bg: Color::Rgb(0, 170, 170),
    menu_active_fg: Color::Rgb(0, 0, 0),
    menu_active_hotkey_fg: Color::Rgb(170, 0, 0),
    status_bar_bg: Color::Rgb(200, 200, 200),
    status_bar_fg: Color::Rgb(0, 0, 0),
    status_hotkey_fg: Color::Rgb(170, 0, 0),
    selected_bg: Color::Rgb(0, 170, 170),
    selected_fg: Color::Rgb(0, 0, 0),
    editor_text_fg: Color::Rgb(0, 0, 0),
    editor_text_bg: Color::Rgb(255, 255, 255),
    editor_identifier_fg: Color::Rgb(0, 5, 0),
    editor_line_number_fg: Color::Rgb(0, 60, 0),
    editor_line_number_bg: Color::Rgb(200, 200, 200),
    editor_selection_bg: Color::Rgb(0, 210, 230),
    editor_selection_fg: Color::Rgb(0, 0, 0),
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Plain,
    Identifier,
    Keyword,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RunStyle {
    Normal,
    Identifier,
    Keyword,
    Selected,
}

const KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false",
    "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut",
    "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait",
    "true", "type", "unsafe", "use", "where", "while",
];

pub fn draw(frame: &mut Frame, app: &mut App) {
    let root = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(CURRENT_THEME.app_background)),
        root,
    );

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(8)])
        .split(root);

    app.geometry = Geometry {
        root,
        menu_area: vertical[0],
        ..Geometry::default()
    };

    if app.menu_open {
        draw_menu(frame, vertical[0], app);
    } else {
        draw_file_header(frame, vertical[0], app);
    }
    draw_desktop(frame, vertical[1], app);

    if app.menu_open {
        draw_menu_dropdown(frame, app);
    }

    if app.help_open {
        draw_help(frame, centered(root, 72, 15));
    } else if let Some(dialog) = app.dialog {
        draw_dialog(frame, dialog, centered(root, 60, 10));
    }
}

fn draw_file_header(frame: &mut Frame, area: Rect, app: &App) {
    let dirty = if app.editor.is_dirty() { " *" } else { "" };
    let label = format!(" {}{} ", app.current_file_label(), dirty);
    let base = Style::default()
        .fg(CURRENT_THEME.status_bar_fg)
        .bg(CURRENT_THEME.status_bar_bg)
        .add_modifier(Modifier::BOLD);
    let key = Style::default()
        .fg(CURRENT_THEME.status_hotkey_fg)
        .bg(CURRENT_THEME.status_bar_bg)
        .add_modifier(Modifier::BOLD);

    let mut right = Line::from(vec![
        Span::styled("F1", key),
        Span::styled(" Help  ", base),
        Span::styled("F10", key),
        Span::styled(" Menu ", base),
    ]);
    let right_width = right.width();
    let total_width = area.width as usize;

    let mut spans = Vec::new();
    if total_width > right_width {
        let left_width = total_width - right_width;
        let left = truncate(&label, left_width as u16);
        let rendered_left = left.chars().count();
        spans.push(Span::styled(left, base));
        if rendered_left < left_width {
            spans.push(Span::styled(" ".repeat(left_width - rendered_left), base));
        }
    }
    spans.append(&mut right.spans);

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(base),
        area,
    );
}

fn draw_menu(frame: &mut Frame, area: Rect, app: &mut App) {
    app.geometry.menu = MenuGeometry::default();
    let mut spans = vec![Span::styled(
        " trubo ",
        Style::default()
            .fg(CURRENT_THEME.menu_brand_fg)
            .bg(CURRENT_THEME.menu_brand_bg)
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
                .fg(CURRENT_THEME.menu_active_fg)
                .bg(CURRENT_THEME.menu_active_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(CURRENT_THEME.menu_bar_fg)
                .bg(CURRENT_THEME.menu_bar_bg)
        };
        let hot_style = if active {
            Style::default()
                .fg(CURRENT_THEME.menu_active_hotkey_fg)
                .bg(CURRENT_THEME.menu_active_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(CURRENT_THEME.menu_hotkey_fg)
                .bg(CURRENT_THEME.menu_bar_bg)
                .add_modifier(Modifier::BOLD)
        };

        spans.push(Span::styled(" ", style));
        spans.push(Span::styled(hot, hot_style));
        spans.push(Span::styled(rest, style));
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(
            Style::default()
                .fg(CURRENT_THEME.menu_bar_fg)
                .bg(CURRENT_THEME.menu_bar_bg),
        ),
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
        .border_style(
            Style::default()
                .fg(CURRENT_THEME.panel_border_active)
                .bg(CURRENT_THEME.menu_active_bg),
        )
        .style(
            Style::default()
                .fg(CURRENT_THEME.menu_bar_fg)
                .bg(CURRENT_THEME.menu_bar_bg),
        );
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
                    Style::default()
                        .fg(CURRENT_THEME.panel_text_muted)
                        .bg(CURRENT_THEME.menu_bar_bg),
                ))
            } else {
                let active = index == app.active_menu_item;
                let style = if active {
                    Style::default()
                        .fg(CURRENT_THEME.editor_text_fg)
                        .bg(CURRENT_THEME.editor_text_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(CURRENT_THEME.menu_bar_fg)
                        .bg(CURRENT_THEME.menu_bar_bg)
                };
                let hot_style = if active {
                    Style::default()
                        .fg(CURRENT_THEME.status_hotkey_fg)
                        .bg(CURRENT_THEME.editor_text_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(CURRENT_THEME.menu_hotkey_fg)
                        .bg(CURRENT_THEME.menu_bar_bg)
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
        Paragraph::new(lines).style(
            Style::default()
                .fg(CURRENT_THEME.menu_bar_fg)
                .bg(CURRENT_THEME.menu_bar_bg),
        ),
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
        Some("Enter Open"),
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
            .style(
                Style::default()
                    .fg(CURRENT_THEME.panel_text_primary)
                    .bg(CURRENT_THEME.panel_background),
            )
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
                    .fg(CURRENT_THEME.selected_fg)
                    .bg(CURRENT_THEME.selected_bg)
                    .add_modifier(Modifier::BOLD)
            } else if entry.is_directory() {
                Style::default()
                    .fg(CURRENT_THEME.panel_title_text)
                    .bg(CURRENT_THEME.panel_background)
            } else {
                Style::default()
                    .fg(CURRENT_THEME.panel_text_primary)
                    .bg(CURRENT_THEME.panel_background)
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

    frame.render_widget(
        List::new(items).style(Style::default().bg(CURRENT_THEME.panel_background)),
        inner,
    );

    draw_browser_log(frame, split[1], app);
}

fn draw_browser_log(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Log ")
        .title_style(
            Style::default()
                .fg(CURRENT_THEME.panel_title_text)
                .bg(CURRENT_THEME.panel_background),
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(
            Style::default()
                .fg(CURRENT_THEME.panel_border_inactive)
                .bg(CURRENT_THEME.panel_background),
        )
        .style(Style::default().bg(CURRENT_THEME.panel_background));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let log_lines = Text::from(vec![
        Line::from(Span::styled(
            truncate(&app.status, inner.width),
            Style::default()
                .fg(CURRENT_THEME.panel_text_primary)
                .bg(CURRENT_THEME.panel_background),
        )),
        Line::from(Span::styled(
            truncate(&format!("Dir: {}", app.browser_label()), inner.width),
            Style::default()
                .fg(CURRENT_THEME.panel_text_secondary)
                .bg(CURRENT_THEME.panel_background),
        )),
    ]);

    frame.render_widget(
        Paragraph::new(log_lines).wrap(Wrap { trim: true }),
        inner,
    );
}

fn draw_editor(frame: &mut Frame, area: Rect, app: &mut App) {
    let border = if app.focus == Focus::Editor {
        Style::default()
            .fg(CURRENT_THEME.panel_border_active)
            .bg(CURRENT_THEME.panel_background)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(CURRENT_THEME.panel_border_inactive)
            .bg(CURRENT_THEME.panel_background)
    };
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_type(BorderType::Double)
        .border_style(border)
        .style(Style::default().bg(CURRENT_THEME.panel_background));
    let inner = block.inner(area);
    app.geometry.editor_inner = inner;
    frame.render_widget(block, area);

    let line_number_width = app.editor_line_number_width();
    let text_cols = inner.width.saturating_sub(line_number_width + 1).max(1) as usize;
    let text_rows = inner.height as usize;
    app.editor.set_viewport(text_rows, text_cols);

    let mut lines = Vec::with_capacity(text_rows);
    let mut wrap_marker_rows = Vec::new();
    let row_offset = app.editor.row_offset();
    let mut segment_offset = app.editor.row_segment_offset();
    let mut file_row = row_offset;
    while lines.len() < text_rows {
        if let Some(line) = app.editor.lines().get(file_row) {
            let line_len = line.chars().count();
            let wrapped = wrapped_rows(line_len, text_cols);
            let token_kinds = tokenize_line(line);
            let full_width_selected = app
                .editor
                .selection_bounds()
                .is_some_and(|(start, end)| file_row > start.row && file_row < end.row);
            let selection = app.editor.selection_range_for_line(file_row);

            for segment in segment_offset..wrapped {
                if lines.len() >= text_rows {
                    break;
                }

                let mut spans = Vec::new();
                let number = if segment == 0 {
                    format!(
                        "{:>width$} ",
                        file_row + 1,
                        width = line_number_width.saturating_sub(1) as usize
                    )
                } else {
                    " ".repeat(line_number_width as usize)
                };
                spans.push(Span::styled(
                    number,
                    Style::default()
                        .fg(CURRENT_THEME.editor_line_number_fg)
                        .bg(CURRENT_THEME.editor_line_number_bg),
                ));

                spans.extend(render_editor_segment(
                    line,
                    segment * text_cols,
                    text_cols,
                    selection,
                    full_width_selected,
                    &token_kinds,
                ));

                if segment + 1 < wrapped {
                    wrap_marker_rows.push(lines.len());
                }
                lines.push(Line::from(spans));
            }

            segment_offset = 0;
            file_row += 1;
        } else {
            let mut spans = Vec::new();
            spans.push(Span::styled(
                " ".repeat(line_number_width as usize),
                Style::default()
                    .fg(CURRENT_THEME.panel_text_muted)
                    .bg(CURRENT_THEME.editor_text_bg),
            ));
            spans.push(Span::styled(
                " ".repeat(text_cols),
                Style::default()
                    .fg(CURRENT_THEME.editor_text_fg)
                    .bg(CURRENT_THEME.editor_text_bg),
            ));
            lines.push(Line::from(spans));
            file_row += 1;
        }
    }

    frame.render_widget(
        Paragraph::new(lines).style(
            Style::default()
                .fg(CURRENT_THEME.editor_text_fg)
                .bg(CURRENT_THEME.editor_text_bg),
        ),
        inner,
    );

    if area.width > 0 {
        let border_x = area.x + area.width.saturating_sub(1);
        for screen_row in wrap_marker_rows {
            let marker_area = Rect {
                x: border_x,
                y: inner.y + screen_row as u16,
                width: 1,
                height: 1,
            };
            frame.render_widget(
                Paragraph::new("↩").style(border),
                marker_area,
            );
        }
    }

    if app.focus == Focus::Editor {
        let cursor_segment = app.editor.cursor_col() / text_cols;
        let cursor_x = inner.x + line_number_width + (app.editor.cursor_col() % text_cols) as u16;
        let mut visual_from_top = 0usize;
        if app.editor.cursor_row() == row_offset {
            visual_from_top = cursor_segment.saturating_sub(app.editor.row_segment_offset());
        } else if app.editor.cursor_row() > row_offset {
            let first_wrapped = app
                .editor
                .lines()
                .get(row_offset)
                .map(|line| wrapped_rows(line.chars().count(), text_cols))
                .unwrap_or(1);
            visual_from_top += first_wrapped.saturating_sub(app.editor.row_segment_offset());
            visual_from_top += app
                .editor
                .lines()
                .iter()
                .enumerate()
                .skip(row_offset + 1)
                .take(app.editor.cursor_row().saturating_sub(row_offset + 1))
                .map(|(_, line)| wrapped_rows(line.chars().count(), text_cols))
                .sum::<usize>();
            visual_from_top += cursor_segment;
        }

        let cursor_y = inner.y + visual_from_top as u16;
        if cursor_x < inner.x + inner.width && cursor_y < inner.y + inner.height {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
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
        .border_style(
            Style::default()
                .fg(CURRENT_THEME.panel_border_active)
                .bg(CURRENT_THEME.menu_active_bg),
        )
        .style(
            Style::default()
                .fg(CURRENT_THEME.menu_bar_fg)
                .bg(CURRENT_THEME.menu_bar_bg),
        );
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
            .style(
                Style::default()
                    .fg(CURRENT_THEME.menu_bar_fg)
                    .bg(CURRENT_THEME.menu_bar_bg),
            )
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true }),
        inner,
    );
}

fn draw_dialog(frame: &mut Frame, dialog: Dialog, area: Rect) {
    frame.render_widget(Clear, area);
    let title = match dialog {
        Dialog::About => " About trubo ",
        Dialog::ConfirmExit { .. } => " Confirm Exit ",
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(
            Style::default()
                .fg(CURRENT_THEME.panel_border_active)
                .bg(CURRENT_THEME.menu_active_bg),
        )
        .style(
            Style::default()
                .fg(CURRENT_THEME.menu_bar_fg)
                .bg(CURRENT_THEME.menu_bar_bg),
        );
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
        Dialog::ConfirmExit { dirty, selection } => {
            let reason = match (dirty, selection) {
                (true, true) => "Unsaved changes and a selection are active.",
                (true, false) => "Unsaved changes are present.",
                (false, true) => "A text selection is active.",
                (false, false) => "",
            };

            Text::from(vec![
                Line::from(""),
                Line::from(vec![Span::styled(
                    "Exit trubo now?",
                    Style::default().add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
                Line::from(reason),
                Line::from(""),
                Line::from("Y / Enter = Exit"),
                Line::from("N / Esc = Stay in editor"),
            ])
        }
    };

    frame.render_widget(
        Paragraph::new(text)
            .style(
                Style::default()
                    .fg(CURRENT_THEME.menu_bar_fg)
                    .bg(CURRENT_THEME.menu_bar_bg),
            )
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        inner,
    );
}

fn retro_block<'a>(
    title: &'a str,
    active: bool,
    left: Option<&'a str>,
    right: Option<&'a str>,
) -> Block<'a> {
    let border = if active {
        Style::default()
            .fg(CURRENT_THEME.panel_border_active)
            .bg(CURRENT_THEME.panel_background)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(CURRENT_THEME.panel_border_inactive)
            .bg(CURRENT_THEME.panel_background)
    };

    let mut block = Block::default()
        .title(title)
        .title_style(
            Style::default()
                .fg(CURRENT_THEME.panel_title_text)
                .bg(CURRENT_THEME.panel_background),
        )
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
        .border_type(BorderType::Double)
        .border_style(border)
        .style(Style::default().bg(CURRENT_THEME.panel_background));

    if let Some(left) = left {
        block = block.title_bottom(Line::from(Span::styled(
            left,
            Style::default()
                .fg(CURRENT_THEME.panel_title_text)
                .bg(CURRENT_THEME.panel_background),
        )));
    }

    if let Some(right) = right {
        block = block.title_bottom(
            Line::from(Span::styled(
                right,
                Style::default()
                    .fg(CURRENT_THEME.panel_text_secondary)
                    .bg(CURRENT_THEME.panel_background),
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

fn render_editor_segment(
    line: &str,
    segment_start: usize,
    text_cols: usize,
    selection: Option<(usize, usize)>,
    full_width_selected: bool,
    token_kinds: &[TokenKind],
) -> Vec<Span<'static>> {
    let chars = line
        .chars()
        .skip(segment_start)
        .take(text_cols)
        .collect::<Vec<_>>();
    let mut spans = Vec::new();
    let mut run = String::new();
    let mut run_style: Option<RunStyle> = None;

    for (screen_col, character) in chars.into_iter().enumerate() {
        let absolute_col = segment_start + screen_col;
        let selected = selection
            .map(|(start, end)| absolute_col >= start && absolute_col < end)
            .unwrap_or(false);
        let token = token_kinds
            .get(absolute_col)
            .copied()
            .unwrap_or(TokenKind::Plain);
        let style = if selected {
            RunStyle::Selected
        } else if token == TokenKind::Keyword {
            RunStyle::Keyword
        } else if token == TokenKind::Identifier {
            RunStyle::Identifier
        } else {
            RunStyle::Normal
        };

        if run_style == Some(style) || run_style.is_none() {
            run.push(character);
            run_style = Some(style);
        } else {
            push_editor_run(&mut spans, &run, run_style.unwrap_or(RunStyle::Normal));
            run.clear();
            run.push(character);
            run_style = Some(style);
        }
    }

    if !run.is_empty() {
        push_editor_run(&mut spans, &run, run_style.unwrap_or(RunStyle::Normal));
    }

    if full_width_selected && text_cols > 0 {
        let rendered_cols = line.chars().skip(segment_start).take(text_cols).count();
        if rendered_cols < text_cols {
            let padding = " ".repeat(text_cols - rendered_cols);
            push_editor_run(&mut spans, &padding, RunStyle::Selected);
        }
    }

    spans
}

fn wrapped_rows(line_len: usize, text_cols: usize) -> usize {
    if text_cols == 0 {
        return 1;
    }
    line_len.max(1).div_ceil(text_cols)
}

fn tokenize_line(line: &str) -> Vec<TokenKind> {
    let chars = line.chars().collect::<Vec<_>>();
    let mut kinds = vec![TokenKind::Plain; chars.len()];
    let mut idx = 0;

    while idx < chars.len() {
        if is_identifier_start(chars[idx]) {
            let start = idx;
            idx += 1;
            while idx < chars.len() && is_identifier_continue(chars[idx]) {
                idx += 1;
            }
            let identifier = chars[start..idx].iter().collect::<String>();
            let kind = if KEYWORDS.contains(&identifier.as_str()) {
                TokenKind::Keyword
            } else {
                TokenKind::Identifier
            };
            for token_kind in &mut kinds[start..idx] {
                *token_kind = kind;
            }
        } else {
            idx += 1;
        }
    }

    kinds
}

fn is_identifier_start(character: char) -> bool {
    character == '_' || character.is_ascii_alphabetic()
}

fn is_identifier_continue(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}

fn push_editor_run(spans: &mut Vec<Span<'static>>, run: &str, style: RunStyle) {
    let style = match style {
        RunStyle::Selected => Style::default()
            .fg(CURRENT_THEME.editor_selection_fg)
            .bg(CURRENT_THEME.editor_selection_bg)
            .add_modifier(Modifier::BOLD),
        RunStyle::Identifier => Style::default()
            .fg(CURRENT_THEME.editor_identifier_fg)
            .bg(CURRENT_THEME.editor_text_bg),
        RunStyle::Keyword => Style::default()
            .fg(CURRENT_THEME.editor_text_fg)
            .bg(CURRENT_THEME.editor_text_bg)
            .add_modifier(Modifier::BOLD),
        RunStyle::Normal => Style::default()
            .fg(CURRENT_THEME.editor_text_fg)
            .bg(CURRENT_THEME.editor_text_bg),
    };
    spans.push(Span::styled(run.to_string(), style));
}
