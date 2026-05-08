use std::path::Path;

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
    panel_text_muted: Color,
    panel_title_text: Color,
    browser_header_bg: Color,
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
    browser_selected_active_bg: Color,
    browser_selected_inactive_bg: Color,
    selected_fg: Color,
    editor_text_fg: Color,
    editor_text_bg: Color,
    editor_identifier_fg: Color,
    editor_line_number_fg: Color,
    editor_line_number_bg: Color,
    editor_selection_bg: Color,
}

const CURRENT_THEME: Theme = Theme {
    app_background: Color::Rgb(200, 200, 200),
    panel_background: Color::Rgb(225, 255, 225),
    panel_border_inactive: Color::Rgb(250, 250, 250),
    panel_border_active: Color::Rgb(55, 155, 0),
    panel_text_primary: Color::Rgb(0, 0, 0),
    panel_text_muted: Color::Rgb(85, 85, 85),
    panel_title_text: Color::Rgb(0, 0, 80),
    browser_header_bg: Color::Rgb(210, 230, 255),
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
    browser_selected_active_bg: Color::Rgb(255, 195, 20),
    browser_selected_inactive_bg: Color::Rgb(180, 240, 240),
    selected_fg: Color::Rgb(0, 0, 0),
    editor_text_fg: Color::Rgb(0, 0, 0),
    editor_text_bg: Color::Rgb(255, 255, 255),
    editor_identifier_fg: Color::Rgb(0, 5, 0),
    editor_line_number_fg: Color::Rgb(0, 60, 0),
    editor_line_number_bg: Color::Rgb(200, 200, 200),
    editor_selection_bg: Color::Rgb(180, 240, 240),
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Plain,
    Identifier,
    Keyword,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct RunStyle {
    token: TokenKind,
    selected: bool,
}

const RUST_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false",
    "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut",
    "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait",
    "true", "type", "unsafe", "use", "where", "while",
];

const SCALA3_ALL_KEYWORDS: &[&str] = &[
    "abstract", "case", "catch", "class", "def", "do", "else", "enum", "export",
    "extends", "false", "final", "finally", "for", "given", "if", "implicit",
    "import", "lazy", "match", "new", "null", "object", "override", "package",
    "private", "protected", "return", "sealed", "super", "then", "throw", "trait",
    "true", "try", "type", "val", "var", "while", "with", "yield", ":", "=",
    "<-", "=>", "<:", ">:", "#", "@", "=>>", "?=>", "as", "derives", "end",
    "extension", "infix", "inline", "opaque", "open", "transparent", "using", "|",
    "*", "+", "-",
];

const LEAN_KEYWORDS: &[&str] = &[
    "import", "prelude",
    "open", "as", "renaming", "replacing", "hiding", "exposing",
    "export",
    "namespace", "section",
    "parameter", "parameters", "variable", "variables", "universe",
    "universes", "include", "omit",
    "protected", "private", "noncomputable", "meta", "mutual",
    "theory",
    "definition", "def", "constant", "constants", "lemma", "theorem", "example",
    "axiom", "axioms",
    "inductive", "structure", "class", "extends",
    "begin", "end", "match", "calc", "this", "with", "have",
    "show", "suffices", "by", "in", "at", "let", "forall", "Pi", "fun",
    "exists", "if", "dif", "then", "else",
    "assume", "from", "to", "do",
    "using", "using_well_founded",
    "instance", "attribute",
    "precedence",
    "infix", "infixl", "infixr", "notation", "postfix", "prefix",
    "reserve", "local",
    "set_option",
    "run_command",
    "alias", "declare_trace", "add_key_equivalence", "aliases",
    "register_simp_ext",
    "help", "print", "eval", "check",
];

const DEFAULT_KEYWORDS: &[&str] = &[];

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
        draw_help(frame, centered(root, 84, 28));
    } else if let Some(dialog) = app.dialog {
        draw_dialog(frame, dialog, centered(root, 60, 10));
    }
}

fn draw_file_header(frame: &mut Frame, area: Rect, app: &App) {
    let dirty = if app.editor.is_dirty() { " *" } else { "" };
    let label = format!(" {}{} ", app.current_file_label(), dirty);
    let cursor_line = app.editor.cursor_row() + 1;
    let cursor_col = app.editor.cursor_col() + 1;
    let line_width = cursor_line.to_string().len().max(4);
    let col_width = cursor_col.to_string().len().max(3);
    let cursor_label = format!(" {:>line_width$}:{:>col_width$} ", cursor_line, cursor_col,);
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
        Span::styled("F4", key),
        Span::styled(" Pane  ", base),
        Span::styled("F10", key),
        Span::styled(" Menu", base),
        Span::styled(cursor_label, base),
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
        horizontal: 0,
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

    frame.render_widget(
        Block::default().style(Style::default().bg(CURRENT_THEME.panel_background)),
        split[0],
    );
    let inner = split[0];

    let header_lines = wrap_path_lines(&app.browser_label(), inner.width);
    let header_height = (header_lines.len() as u16).min(inner.height);
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height.max(1)),
            Constraint::Min(0),
        ])
        .split(inner);
    let list_area = sections.get(1).copied().unwrap_or(inner);

    frame.render_widget(
        Paragraph::new(Text::from(header_lines))
            .style(
                Style::default()
                    .fg(CURRENT_THEME.panel_title_text)
                    .bg(CURRENT_THEME.browser_header_bg)
                    .add_modifier(Modifier::BOLD),
            ),
        sections[0],
    );

    app.geometry.browser_inner = list_area;

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
            list_area,
        );
        return;
    }

    let height = list_area.height as usize;
    if height == 0 {
        draw_browser_log(frame, split[1], app);
        return;
    }
    let selected = app.selected_entry;
    let selected_bg = if app.focus == Focus::Browser {
        CURRENT_THEME.browser_selected_active_bg
    } else {
        CURRENT_THEME.browser_selected_inactive_bg
    };
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
                    .bg(selected_bg)
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
                truncate(&label, list_area.width),
                style,
            )))
        })
        .collect::<Vec<_>>();

    frame.render_widget(
        List::new(items).style(Style::default().bg(CURRENT_THEME.panel_background)),
        list_area,
    );

    draw_browser_log(frame, split[1], app);
}

fn draw_browser_log(frame: &mut Frame, area: Rect, app: &App) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    frame.render_widget(
        Paragraph::new(" Log ").style(
            Style::default()
                .fg(CURRENT_THEME.status_bar_fg)
                .bg(CURRENT_THEME.status_bar_bg)
                .add_modifier(Modifier::BOLD),
        ),
        sections[0],
    );

    frame.render_widget(
        Block::default().style(Style::default().bg(CURRENT_THEME.panel_background)),
        sections[1],
    );

    let log_lines = vec![
        Line::from(Span::styled(
            format!("Status: {}", app.status),
            Style::default()
                .fg(CURRENT_THEME.panel_text_primary)
                .bg(CURRENT_THEME.panel_background),
        )),
    ];

    frame.render_widget(
        Paragraph::new(Text::from(log_lines))
            .style(Style::default().bg(CURRENT_THEME.panel_background))
            .wrap(Wrap { trim: false }),
        sections[1],
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
    let keywords = keywords_for_path(app.editor.path());
    let row_offset = app.editor.row_offset();
    let mut segment_offset = app.editor.row_segment_offset();
    let mut file_row = row_offset;
    while lines.len() < text_rows {
        if let Some(line) = app.editor.lines().get(file_row) {
            let line_len = line.chars().count();
            let wrapped = wrapped_rows(line_len, text_cols);
            let token_kinds = tokenize_line(line, keywords);
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
        help_bindings_line(&[("F1", "Help"), ("F2", "Save"), ("F3", "Open selected file")]),
        help_bindings_line(&[("F4", "Cycle pane"), ("Tab", "Cycle pane"), ("Shift+Tab", "Cycle pane")]),
        help_bindings_line(&[("Ctrl+Left", "Files pane"), ("Ctrl+Right", "Editor pane"), ("F10", "Menu")]),
        help_bindings_line(&[("F5", "cargo run"), ("F9", "cargo build"), ("Ctrl+Q", "Quit")]),
        help_bindings_line(&[("Ctrl+S", "Save"), ("Ctrl+O", "Open selected file"), ("Ctrl+F", "Cycle pane")]),
        help_bindings_line(&[("Ctrl+L", "Redraw screen"), ("Ctrl+R", "Run"), ("Ctrl+B", "Build")]),
        help_bindings_line(&[("Ctrl+Space", "Toggle select mode")]),
        help_bindings_line(&[("Ctrl+C", "Copy"), ("Ctrl+X", "Cut"), ("Ctrl+V", "Paste"), ("Ctrl+Z", "Undo")]),
        help_bindings_line(&[("Ctrl+Ins", "Copy"), ("Shift+Ins", "Paste"), ("Shift+Del", "Cut")]),
        help_bindings_line(&[("Alt+X", "Delete line"), ("Alt+U", "Duplicate line")]),
        Line::from(""),
        help_section_line("Browser"),
        help_bindings_line(&[("Up/Down", "Move selection"), ("Home/End", "Jump to first/last")]),
        help_bindings_line(&[("Enter", "Open file or directory"), ("Backspace", "Parent directory"), ("R", "Refresh")]),
        help_section_line("Editor"),
        help_bindings_line(&[("Arrows", "Move cursor"), ("Home/End", "Line start/end"), ("PgUp/PgDn", "Scroll")]),
        help_bindings_line(&[("Shift+Arrows/Home/End/Pg", "Extend selection"), ("Backspace/Delete", "Delete text")]),
        help_bindings_line(&[("Enter", "New line"), ("Typing", "Insert text")]),
        help_section_line("Menus and dialogs"),
        help_bindings_line(&[("F10/Esc", "Close menu"), ("Arrows", "Move in menu"), ("Enter", "Activate")]),
        help_bindings_line(&[("Home/End", "First/last menu"), ("Menu hotkeys", "Jump by highlighted letter")]),
        help_bindings_line(&[("Y/Enter", "Confirm exit"), ("N/Esc", "Cancel exit"), ("Any key", "Close help/about")]),
        Line::from(""),
        Line::from("Mouse: click files to open, drag divider to resize,"),
        Line::from("click or drag inside the editor to move/select text."),
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

fn help_bindings_line(items: &[(&str, &str)]) -> Line<'static> {
    let base = Style::default()
        .fg(CURRENT_THEME.menu_bar_fg)
        .bg(CURRENT_THEME.menu_bar_bg);
    let key = Style::default()
        .fg(CURRENT_THEME.status_hotkey_fg)
        .bg(CURRENT_THEME.menu_bar_bg)
        .add_modifier(Modifier::BOLD);

    let mut spans = Vec::new();
    for (index, (binding, description)) in items.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled("   ", base));
        }
        spans.push(Span::styled((*binding).to_string(), key));
        spans.push(Span::styled(format!(" {}", description), base));
    }

    Line::from(spans)
}

fn help_section_line(title: &str) -> Line<'static> {
    Line::from(vec![Span::styled(
        title.to_string(),
        Style::default()
            .fg(CURRENT_THEME.menu_bar_fg)
            .bg(CURRENT_THEME.menu_bar_bg)
            .add_modifier(Modifier::BOLD),
    )])
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
        let style = RunStyle {
            token,
            selected,
        };

        if run_style == Some(style) || run_style.is_none() {
            run.push(character);
            run_style = Some(style);
        } else {
            push_editor_run(
                &mut spans,
                &run,
                run_style.unwrap_or(RunStyle {
                    token: TokenKind::Plain,
                    selected: false,
                }),
            );
            run.clear();
            run.push(character);
            run_style = Some(style);
        }
    }

    if !run.is_empty() {
        push_editor_run(
            &mut spans,
            &run,
            run_style.unwrap_or(RunStyle {
                token: TokenKind::Plain,
                selected: false,
            }),
        );
    }

    if full_width_selected && text_cols > 0 {
        let rendered_cols = line.chars().skip(segment_start).take(text_cols).count();
        if rendered_cols < text_cols {
            let padding = " ".repeat(text_cols - rendered_cols);
            push_editor_run(
                &mut spans,
                &padding,
                RunStyle {
                    token: TokenKind::Plain,
                    selected: true,
                },
            );
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

fn keywords_for_path(path: Option<&Path>) -> &'static [&'static str] {
    match path
        .and_then(|path| path.extension())
        .and_then(|extension| extension.to_str())
    {
        Some("rs") => RUST_KEYWORDS,
        Some("scala") => SCALA3_ALL_KEYWORDS,
        Some("lean") => LEAN_KEYWORDS,
        _ => DEFAULT_KEYWORDS,
    }
}

fn wrap_path_lines(path: &str, width: u16) -> Vec<Line<'static>> {
    if width == 0 {
        return vec![Line::from(String::new())];
    }

    let width = width as usize;
    if path.is_empty() {
        return vec![Line::from(String::new())];
    }

    let mut pieces = Vec::new();
    let is_absolute = path.starts_with('/');
    let segments = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    if is_absolute {
        if segments.is_empty() {
            pieces.push("/".to_string());
        } else {
            pieces.push(format!("/{}", segments[0]));
            for segment in segments.iter().skip(1) {
                pieces.push(format!("/{segment}"));
            }
        }
    } else if let Some((first, rest)) = segments.split_first() {
        pieces.push((*first).to_string());
        for segment in rest {
            pieces.push(format!("/{segment}"));
        }
    }

    let mut lines = Vec::new();
    let mut current = String::new();

    for piece in pieces {
        let piece_len = piece.chars().count();
        let current_len = current.chars().count();

        if current.is_empty() && piece_len <= width {
            current = piece;
            continue;
        }

        if !current.is_empty() && current_len + piece_len <= width {
            current.push_str(&piece);
            continue;
        }

        if !current.is_empty() {
            lines.push(Line::from(std::mem::take(&mut current)));
        }

        if piece_len <= width {
            current = piece;
            continue;
        }

        let mut chunk = String::new();
        for character in piece.chars() {
            chunk.push(character);
            if chunk.chars().count() == width {
                lines.push(Line::from(std::mem::take(&mut chunk)));
            }
        }
        current = chunk;
    }

    if !current.is_empty() {
        lines.push(Line::from(current));
    }

    if lines.is_empty() {
        vec![Line::from(String::new())]
    } else {
        lines
    }
}

fn tokenize_line(line: &str, keywords: &[&str]) -> Vec<TokenKind> {
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
            let kind = if keywords.contains(&identifier.as_str()) {
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
    let mut rendered_style = match style.token {
        TokenKind::Identifier => Style::default()
            .fg(CURRENT_THEME.editor_identifier_fg)
            .bg(CURRENT_THEME.editor_text_bg),
        TokenKind::Keyword => Style::default()
            .fg(CURRENT_THEME.editor_text_fg)
            .bg(CURRENT_THEME.editor_text_bg)
            .add_modifier(Modifier::BOLD),
        TokenKind::Plain => Style::default()
            .fg(CURRENT_THEME.editor_text_fg)
            .bg(CURRENT_THEME.editor_text_bg),
    };

    if style.selected {
        rendered_style = rendered_style.bg(CURRENT_THEME.editor_selection_bg);
    }

    spans.push(Span::styled(run.to_string(), rendered_style));
}
