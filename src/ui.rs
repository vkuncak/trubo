use crate::app::{
    App, Dialog, Focus, Geometry, MENUS, MIN_BROWSER_PANE_WIDTH, MIN_EDITOR_PANE_WIDTH,
    MenuGeometry,
};
use crate::file_types::{DEFAULT_KEYWORDS, FileTypeSpec, comment_start_for_line, detect_file_type};
use std::path::Path;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
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
    browser_header_active_bg: Color,
    menu_brand_bg: Color,
    menu_brand_fg: Color,
    menu_bar_bg: Color,
    menu_bar_fg: Color,
    menu_hotkey_fg: Color,
    menu_active_bg: Color,
    menu_active_fg: Color,
    menu_active_hotkey_fg: Color,
    menu_selected_bg: Color,
    menu_selected_fg: Color,
    menu_selected_hotkey_fg: Color,
    dialog_border: Color,
    dialog_background: Color,
    status_bar_bg: Color,
    status_bar_fg: Color,
    status_hotkey_fg: Color,
    browser_selected_active_bg: Color,
    browser_selected_inactive_bg: Color,
    selected_fg: Color,
    browser_marked_fg: Color,
    editor_text_fg: Color,
    editor_text_bg: Color,
    editor_comment_fg: Color,
    editor_identifier_fg: Color,
    editor_line_number_fg: Color,
    editor_line_number_bg: Color,
    editor_current_line_bg: Color,
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
    browser_header_active_bg: Color::Rgb(255, 225, 150),
    menu_brand_bg: Color::Rgb(200, 200, 200),
    menu_brand_fg: Color::Rgb(255, 255, 85),
    menu_bar_bg: Color::Rgb(200, 200, 200),
    menu_bar_fg: Color::Rgb(0, 0, 0),
    menu_hotkey_fg: Color::Rgb(170, 0, 0),
    menu_active_bg: Color::Rgb(0, 170, 170),
    menu_active_fg: Color::Rgb(0, 0, 0),
    menu_active_hotkey_fg: Color::Rgb(170, 0, 0),
    menu_selected_bg: Color::Rgb(100, 255, 255),
    menu_selected_fg: Color::Rgb(0, 0, 0),
    menu_selected_hotkey_fg: Color::Rgb(170, 0, 0),
    dialog_border: Color::Rgb(0, 0, 0),
    dialog_background: Color::Rgb(210, 230, 255),
    status_bar_bg: Color::Rgb(200, 200, 200),
    status_bar_fg: Color::Rgb(0, 0, 0),
    status_hotkey_fg: Color::Rgb(170, 0, 0),
    browser_selected_active_bg: Color::Rgb(255, 195, 20),
    browser_selected_inactive_bg: Color::Rgb(210, 230, 255),
    selected_fg: Color::Rgb(0, 0, 0),
    browser_marked_fg: Color::Rgb(0, 0, 200),
    editor_text_fg: Color::Rgb(0, 0, 0),
    editor_text_bg: Color::Rgb(255, 255, 255),
    editor_comment_fg: Color::Rgb(0, 90, 0),
    editor_identifier_fg: Color::Rgb(0, 5, 0),
    editor_line_number_fg: Color::Rgb(0, 60, 0),
    editor_line_number_bg: Color::Rgb(200, 200, 200),
    editor_current_line_bg: Color::Rgb(234, 230, 169),
    editor_selection_bg: Color::Rgb(180, 240, 240),
};

const SECONDARY_BROWSER_GUTTER_WIDTH: u16 = 1;
const FILE_OPERATION_DIALOG_WIDTH: u16 = 72;
const FILE_CONFLICT_DIALOG_WIDTH: u16 = 76;
const DEFAULT_HEADER_PATH_WIDTH: usize = 40;
const SECONDARY_BROWSER_HEADER_BINDINGS: [(&str, &str); 4] = [
    ("F5", " Copy  "),
    ("F6", " Move  "),
    ("F7", " MkDir  "),
    ("F8", " Delete "),
];
const PRIMARY_HEADER_BINDINGS: [(&str, &str); 4] = [
    ("F1", " Help  "),
    ("Ctrl-B", " Side  "),
    ("F10", " Menu  "),
    ("Ctrl-Q", " Quit"),
];
const PRIMARY_HEADER_BINDINGS_WITH_DUAL: [(&str, &str); 5] = [
    ("F1", " Help  "),
    ("Ctrl-B", " Side  "),
    ("`", " Dual "),
    ("F10", " Menu  "),
    ("Ctrl-Q", " Quit"),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Plain,
    Identifier,
    Keyword,
    Comment,
    Title,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct RunStyle {
    token: TokenKind,
    selected: bool,
    current_line: bool,
}

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
        match dialog {
            Dialog::About => draw_dialog(frame, app, dialog, centered(root, 60, 10)),
            Dialog::SaveFile => draw_dialog(frame, app, dialog, centered(root, 72, 10)),
            Dialog::NewDirectory => draw_dialog(frame, app, dialog, centered(root, 76, 12)),
            Dialog::OpenFilePath => draw_dialog(frame, app, dialog, centered(root, 76, 5)),
            Dialog::RegexSearch => {
                draw_dialog(frame, app, dialog, anchored_search_area(app, root, 48, 5))
            }
            Dialog::BrowserIncrementalSearch => {
                draw_dialog(frame, app, dialog, anchored_file_operation_area(app, root, 52, 5))
            }
            Dialog::BrowserSelectionPattern => {
                draw_dialog(frame, app, dialog, anchored_file_operation_area(app, root, 60, 7))
            }
            Dialog::FileOperationName => {
                draw_dialog(
                    frame,
                    app,
                    dialog,
                    anchored_file_operation_area(app, root, FILE_OPERATION_DIALOG_WIDTH, 5),
                )
            }
            Dialog::ConfirmFileOperation => draw_dialog(
                frame,
                app,
                dialog,
                anchored_file_operation_area(
                    app,
                    root,
                    FILE_OPERATION_DIALOG_WIDTH,
                    confirm_file_operation_dialog_height(app),
                ),
            ),
            Dialog::ResolveFileConflict => draw_dialog(
                frame,
                app,
                dialog,
                anchored_file_operation_area(app, root, FILE_CONFLICT_DIALOG_WIDTH, 7),
            ),
        }
    }
}

fn draw_file_header(frame: &mut Frame, area: Rect, app: &App) {
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

    let mut right = Line::from(build_header_right_spans(app, base, key, &cursor_label));
    let right_width = right.width();
    let total_width = area.width as usize;

    let mut spans = Vec::new();
    if total_width > right_width {
        let left_width = total_width - right_width;
        spans.extend(build_header_left_spans(app, left_width as u16, base, key));
    }
    spans.append(&mut right.spans);

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(base),
        area,
    );
}

fn build_header_left_spans(app: &App, width: u16, base: Style, key: Style) -> Vec<Span<'static>> {
    if app.focus == Focus::BrowserSecondary {
        return fit_header_segments(
            binding_segments(&SECONDARY_BROWSER_HEADER_BINDINGS),
            width,
            base,
            key,
        );
    }

    let dirty = if app.editor.is_dirty() { " *" } else { "" };
    let read_only = if app.editor.is_read_only() { " [RO]" } else { "" };
    let file_label = compress_middle(&app.current_file_label(), DEFAULT_HEADER_PATH_WIDTH);
    let label = format!(
        " {} ({}){}{} ",
        file_label,
        app.editor.header_metric_label(),
        dirty,
        read_only,
    );
    fit_header_segments(
        vec![(label, false)],
        width,
        base,
        key,
    )
}

fn build_header_right_spans(app: &App, base: Style, key: Style, cursor_label: &str) -> Vec<Span<'static>> {
    let bindings: &[(&str, &str)] = if app.editor_only_mode {
        &PRIMARY_HEADER_BINDINGS
    } else {
        &PRIMARY_HEADER_BINDINGS_WITH_DUAL
    };
    let mut spans = build_binding_spans(bindings, base, key);
    spans.push(Span::styled(cursor_label.to_string(), base));
    spans
}

fn build_binding_spans(items: &[(&str, &str)], base: Style, key: Style) -> Vec<Span<'static>> {
    binding_segments(items)
        .into_iter()
        .map(|(text, is_key)| Span::styled(text, if is_key { key } else { base }))
        .collect()
}

fn binding_segments(items: &[(&str, &str)]) -> Vec<(String, bool)> {
    let mut segments = Vec::new();
    for (binding, label) in items {
        segments.push(((*binding).to_string(), true));
        segments.push(((*label).to_string(), false));
    }
    segments
}

fn fit_header_segments(
    segments: Vec<(String, bool)>,
    width: u16,
    base: Style,
    key: Style,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut remaining = width as usize;

    for (text, is_key) in segments {
        if remaining == 0 {
            break;
        }

        let char_count = text.chars().count();
        if char_count <= remaining {
            spans.push(Span::styled(text, if is_key { key } else { base }));
            remaining -= char_count;
        } else {
            spans.push(Span::styled(
                truncate(&text, remaining as u16),
                if is_key { key } else { base },
            ));
            remaining = 0;
        }
    }

    if remaining > 0 {
        spans.push(Span::styled(" ".repeat(remaining), base));
    }

    spans
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
        .unwrap_or(14) as u16;
    let height = menu.items.len() as u16;
    let max_x = app.geometry.root.x + app.geometry.root.width.saturating_sub(width);
    let area = Rect {
        x: bar.x.min(max_x),
        y: app.geometry.menu_area.y.saturating_add(1),
        width,
        height: height.min(app.geometry.root.height.saturating_sub(1)),
    };
    app.geometry.menu.dropdown = Some(area);

    frame.render_widget(Clear, area);
    let inner = area;
    frame.render_widget(
        Block::default().style(
            Style::default()
                .fg(CURRENT_THEME.menu_bar_fg)
                .bg(CURRENT_THEME.dialog_background),
        ),
        area,
    );

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
                        .fg(CURRENT_THEME.menu_selected_fg)
                        .bg(CURRENT_THEME.menu_selected_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(CURRENT_THEME.menu_bar_fg)
                        .bg(CURRENT_THEME.menu_bar_bg)
                };
                let hot_style = if active {
                    Style::default()
                        .fg(CURRENT_THEME.menu_selected_hotkey_fg)
                        .bg(CURRENT_THEME.menu_selected_bg)
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
    app.geometry.desktop_inner = desktop;
    app.geometry.browser_areas = [Rect::default(); 2];
    app.geometry.browser_inners = [Rect::default(); 2];
    app.geometry.browser_log_area = Rect::default();
    app.geometry.browser_log_divider_area = Rect::default();

    if app.editor_only_mode {
        app.geometry.editor_area = desktop;
        draw_editor(frame, desktop, app);
        return;
    }

    app.browser_pane_width = clamp_browser_width(
        desktop.width,
        app.browser_pane_width,
        app.visible_browser_count() as u16,
    );

    if app.secondary_browser_enabled {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(app.browser_pane_width),
                Constraint::Length(app.browser_pane_width),
                Constraint::Min(MIN_EDITOR_PANE_WIDTH),
            ])
            .split(desktop);

        app.geometry.browser_areas[0] = columns[0];
        let secondary_split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(SECONDARY_BROWSER_GUTTER_WIDTH.min(columns[1].width)),
                Constraint::Min(0),
            ])
            .split(columns[1]);
        frame.render_widget(
            Block::default().style(Style::default().bg(CURRENT_THEME.app_background)),
            secondary_split[0],
        );
        app.geometry.browser_areas[1] = secondary_split[1];
        app.geometry.editor_area = columns[2];
        draw_browser(frame, columns[0], app, 0, true);
        draw_browser(frame, secondary_split[1], app, 1, false);
        draw_editor(frame, columns[2], app);
    } else {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(app.browser_pane_width),
                Constraint::Min(MIN_EDITOR_PANE_WIDTH),
            ])
            .split(desktop);

        app.geometry.browser_areas[0] = columns[0];
        app.geometry.editor_area = columns[1];
        draw_browser(frame, columns[0], app, 0, true);
        draw_editor(frame, columns[1], app);
    }
}

fn draw_browser(frame: &mut Frame, area: Rect, app: &mut App, browser_index: usize, show_log: bool) {
    let list_panel = if show_log {
        if app.browser_log_height == 0 {
            app.browser_log_height = default_browser_log_height(area.height);
        }
        let log_height = clamp_browser_log_height(area.height, app.browser_log_height);
        app.browser_log_height = log_height;
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(log_height.min(area.height)),
            ])
            .split(area);
        app.geometry.browser_log_area = split[1];
        draw_browser_log(frame, split[1], app);
        split[0]
    } else {
        area
    };

    frame.render_widget(
        Block::default().style(Style::default().bg(CURRENT_THEME.panel_background)),
        list_panel,
    );
    let inner = list_panel;

    let header_lines = wrap_path_lines(&app.browser_label(browser_index), inner.width);
    let header_height = (header_lines.len() as u16).min(inner.height);
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height.max(1)),
            Constraint::Min(0),
        ])
        .split(inner);
    let list_area = sections.get(1).copied().unwrap_or(inner);
    let browser_focus = if browser_index == 0 {
        Focus::BrowserPrimary
    } else {
        Focus::BrowserSecondary
    };
    let is_active_browser = app.focus == browser_focus;
    let mut header_style = Style::default()
        .fg(CURRENT_THEME.panel_title_text)
        .bg(if is_active_browser {
            CURRENT_THEME.browser_header_active_bg
        } else {
            CURRENT_THEME.browser_header_bg
        });
    if is_active_browser {
        header_style = header_style.add_modifier(Modifier::BOLD);
    }

    frame.render_widget(
        Paragraph::new(Text::from(header_lines))
            .style(header_style),
        sections[0],
    );

    app.geometry.browser_inners[browser_index] = list_area;

    let entries = app.browser_entries(browser_index);

    if entries.is_empty() {
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
        return;
    }
    let selected = app.browser_selected_entry(browser_index);
    let selected_bg = if app.focus == if browser_index == 0 { Focus::BrowserPrimary } else { Focus::BrowserSecondary } {
        CURRENT_THEME.browser_selected_active_bg
    } else {
        CURRENT_THEME.browser_selected_inactive_bg
    };
    let start = selected.saturating_sub(height.saturating_sub(1));
    let items = app
        .browser_entries(browser_index)
        .iter()
        .enumerate()
        .skip(start)
        .take(height)
        .map(|(index, entry)| {
            let marked = app.browser_entry_is_selected(browser_index, entry);
            let style = if index == selected {
                Style::default()
                    .fg(if marked {
                        CURRENT_THEME.browser_marked_fg
                    } else {
                        CURRENT_THEME.selected_fg
                    })
                    .bg(selected_bg)
                    .add_modifier(Modifier::BOLD)
            } else if marked {
                Style::default()
                    .fg(CURRENT_THEME.browser_marked_fg)
                    .bg(CURRENT_THEME.panel_background)
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
}

fn draw_browser_log(frame: &mut Frame, area: Rect, app: &mut App) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    app.geometry.browser_log_divider_area = sections[0];

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

    let mut log_lines = app
        .log_lines_with_scroll(sections[1].height as usize, app.log_scroll)
        .into_iter()
        .map(|line| {
            Line::from(Span::styled(
                line,
                Style::default()
                    .fg(CURRENT_THEME.panel_text_primary)
                    .bg(CURRENT_THEME.panel_background),
            ))
        })
        .collect::<Vec<_>>();

    if log_lines.is_empty() {
        log_lines.push(Line::from(Span::styled(
            format!("Status: {}", app.status),
            Style::default()
                .fg(CURRENT_THEME.panel_text_primary)
                .bg(CURRENT_THEME.panel_background),
        )));
    }

    frame.render_widget(
        Paragraph::new(Text::from(log_lines))
            .style(Style::default().bg(CURRENT_THEME.panel_background))
            .wrap(Wrap { trim: false }),
        sections[1],
    );
}

fn clamp_browser_log_height(total_height: u16, preferred: u16) -> u16 {
    preferred.clamp(1, total_height)
}

fn default_browser_log_height(total_height: u16) -> u16 {
    total_height.max(1).div_ceil(3)
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
    let file_type = detect_file_type(app.editor.path(), app.editor.lines().first().map(String::as_str));
    let keywords = file_type
        .map(|spec| spec.keywords)
        .unwrap_or(DEFAULT_KEYWORDS);
    let row_offset = app.editor.row_offset();
    let mut segment_offset = app.editor.row_segment_offset();
    let mut file_row = row_offset;
    while lines.len() < text_rows {
        if let Some(line) = app.editor.lines().get(file_row) {
            let line_len = line.chars().count();
            let wrapped = wrapped_rows(line_len, text_cols);
            let token_kinds = tokenize_line(line, keywords, file_type);
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
                let current_line = app.highlight_current_line() && file_row == app.editor.cursor_row();
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
                        .bg(if current_line {
                            CURRENT_THEME.editor_current_line_bg
                        } else {
                            CURRENT_THEME.editor_line_number_bg
                        }),
                ));

                spans.extend(render_editor_segment(
                    line,
                    segment * text_cols,
                    text_cols,
                    selection,
                    full_width_selected,
                    current_line,
                    &token_kinds,
                ));

                if segment + 1 < wrapped {
                    wrap_marker_rows.push(lines.len());
                }
                lines.push(Line::from(spans));
            }

            for diagnostic in app.editor.diagnostics_for_row(file_row) {
                for (line_index, diagnostic_line) in diagnostic.message.lines().enumerate() {
                    if lines.len() >= text_rows {
                        break;
                    }

                    let mut spans = Vec::new();
                    spans.push(Span::styled(
                        " ".repeat(line_number_width as usize),
                        Style::default()
                            .fg(CURRENT_THEME.editor_line_number_fg)
                            .bg(CURRENT_THEME.editor_text_bg),
                    ));
                    let prefix = if line_index == 0 { "! " } else { "  " };
                    spans.push(Span::styled(
                        format!("{prefix}{diagnostic_line}"),
                        Style::default()
                            .fg(CURRENT_THEME.status_hotkey_fg)
                            .bg(CURRENT_THEME.editor_text_bg)
                            .add_modifier(Modifier::BOLD),
                    ));
                    lines.push(Line::from(spans));
                }
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
            visual_from_top += first_wrapped
                .saturating_sub(app.editor.row_segment_offset())
                + app.editor.diagnostic_visual_rows_for_row(row_offset);
            visual_from_top += (row_offset + 1..app.editor.cursor_row())
                .map(|row| {
                    let wrapped = app
                        .editor
                        .lines()
                        .get(row)
                        .map(|line| wrapped_rows(line.chars().count(), text_cols))
                        .unwrap_or(1);
                    wrapped + app.editor.diagnostic_visual_rows_for_row(row)
                })
                .sum::<usize>();
            visual_from_top += cursor_segment;
        }

        let cursor_y = inner.y + visual_from_top as u16;
        if cursor_x < inner.x + inner.width && cursor_y < inner.y + inner.height {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

fn clamp_browser_width(total_width: u16, current: u16, browser_count: u16) -> u16 {
    if total_width <= 1 {
        return 1;
    }

    let browser_count = browser_count.max(1);
    let editor_reserve = MIN_EDITOR_PANE_WIDTH.min(total_width.saturating_sub(browser_count));
    let max_width = total_width
        .saturating_sub(editor_reserve)
        .checked_div(browser_count)
        .unwrap_or(1)
        .max(1);
    let min_width = MIN_BROWSER_PANE_WIDTH.min(max_width);
    current.clamp(min_width, max_width)
}

fn draw_help(frame: &mut Frame, area: Rect) {
    frame.render_widget(Clear, area);
    let inner = area.inner(Margin {
        vertical: 0,
        horizontal: 1,
    });
    frame.render_widget(
        Block::default().style(
            Style::default()
                .fg(CURRENT_THEME.menu_bar_fg)
                .bg(CURRENT_THEME.dialog_background),
        ),
        area,
    );

    let text = Text::from(vec![
        Line::from(vec![Span::styled(
            "trubo keys",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from("Exit trubo with Ctrl+Q."),
        Line::from("Ctrl+B switches to editor-only mode."),
        Line::from("Use ` in a files pane to toggle dual-pane mode."),
        Line::from("See doc/GUIDE.md for the full guide."),
        Line::from(""),
        help_bindings_line(&[("F1", "Help"), ("F2", "Save"), ("F3", "Open selected file")]),
        help_bindings_line(&[("F4", "Cycle pane"), ("Tab", "Cycle pane"), ("Shift+Tab", "Cycle pane")]),
        help_bindings_line(&[("Ctrl+Left", "Files pane"), ("Ctrl+Right", "Editor pane"), ("F10", "Menu")]),
        help_bindings_line(&[("F5", "Copy entry"), ("F6", "Move entry"), ("F7", "New sub-directory")]),
        help_bindings_line(&[("F8", "Delete entry"), ("F9", "Build current file"), ("Ctrl+Q", "Quit")]),
        help_bindings_line(&[("Ctrl+S", "Save"), ("Ctrl+O", "Open file"), ("Ctrl+F", "Search")]),
        help_bindings_line(&[("Ctrl+L", "Redraw screen"), ("Ctrl+R", "Run current file"), ("Ctrl+B", "Editor only")]),
        help_bindings_line(&[("`", "Toggle dual pane")]),
        help_bindings_line(&[("Ctrl+T/Ins", "Mark/unmark entry")]),
        help_bindings_line(&[("Ctrl+Space", "Compute selected size")]),
        help_bindings_line(&[("Ctrl+C", "Copy"), ("Ctrl+X", "Cut"), ("Ctrl+V", "Paste"), ("Ctrl+Z", "Undo")]),
        help_bindings_line(&[("Ctrl+Y", "Redo")]),
        help_bindings_line(&[("Ctrl+Ins", "Copy"), ("Shift+Ins", "Paste"), ("Shift+Del", "Cut")]),
        help_bindings_line(&[("Ctrl+K", "Delete line")]),
        Line::from(""),
        help_section_line("Browser"),
        help_bindings_line(&[("Up/Down", "Move selection"), ("Home/End", "Jump to first/last")]),
        help_bindings_line(&[("Ctrl+T/Ins", "Mark/unmark entry"), ("F5/F6/F8", "Apply to marked entries")]),
        help_bindings_line(&[("Enter", "Open file or directory"), ("Backspace", "Parent directory"), ("R", "Refresh")]),
        help_section_line("Editor"),
        help_bindings_line(&[("Arrows", "Move cursor"), ("Home/End", "Line start/end"), ("PgUp/PgDn", "Scroll")]),
        help_bindings_line(&[("Ctrl+Home/Ctrl+PgUp", "Start of file"), ("Ctrl+End/Ctrl+PgDn", "End of file")]),
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
                    .bg(CURRENT_THEME.dialog_background),
            )
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true }),
        inner,
    );
}

fn help_bindings_line(items: &[(&str, &str)]) -> Line<'static> {
    let base = Style::default()
        .fg(CURRENT_THEME.menu_bar_fg)
        .bg(CURRENT_THEME.dialog_background);
    let key = Style::default()
        .fg(CURRENT_THEME.status_hotkey_fg)
        .bg(CURRENT_THEME.dialog_background)
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
            .bg(CURRENT_THEME.dialog_background)
            .add_modifier(Modifier::BOLD),
    )])
}

fn draw_dialog(frame: &mut Frame, app: &App, dialog: Dialog, area: Rect) {
    frame.render_widget(Clear, area);
    match dialog {
        Dialog::About => draw_about_dialog(frame, area),
        Dialog::SaveFile => draw_save_file_dialog(frame, app, area),
        Dialog::NewDirectory => draw_new_directory_dialog(frame, app, area),
        Dialog::OpenFilePath => draw_open_file_dialog(frame, app, area),
        Dialog::RegexSearch => draw_regex_search_dialog(frame, app, area),
        Dialog::BrowserIncrementalSearch => draw_browser_incremental_search_dialog(frame, app, area),
        Dialog::BrowserSelectionPattern => draw_browser_selection_pattern_dialog(frame, app, area),
        Dialog::FileOperationName => draw_file_operation_name_dialog(frame, app, area),
        Dialog::ConfirmFileOperation => draw_file_operation_dialog(frame, app, area),
        Dialog::ResolveFileConflict => draw_file_conflict_dialog(frame, app, area),
    }
}

fn draw_about_dialog(frame: &mut Frame, area: Rect) {
    let inner = draw_dialog_shell(frame, area);

    let text = Text::from(vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "About trubo",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
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
    ]);

    frame.render_widget(
        Paragraph::new(text)
            .style(
                Style::default()
                    .fg(CURRENT_THEME.menu_bar_fg)
                    .bg(CURRENT_THEME.dialog_background),
            )
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        inner,
    );
}

fn draw_save_file_dialog(frame: &mut Frame, app: &App, area: Rect) {
    let (path, file_name) = match app.editor.path() {
        Some(path) => {
            let directory = path
                .parent()
                .and_then(|parent| parent.to_str())
                .unwrap_or("")
                .to_string();
            let file_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Untitled")
                .to_string();
            (directory, file_name)
        }
        None => (String::new(), String::from("Untitled")),
    };
    let base = Style::default()
        .fg(CURRENT_THEME.status_bar_fg)
        .bg(CURRENT_THEME.dialog_background);
    let file_style = base.add_modifier(Modifier::BOLD);
    let key = Style::default()
        .fg(CURRENT_THEME.status_hotkey_fg)
        .bg(CURRENT_THEME.dialog_background)
        .add_modifier(Modifier::BOLD);

    let content_area = draw_dialog_shell(frame, area).inner(Margin {
        vertical: 0,
        horizontal: 1,
    });

    let text = Text::from(vec![
        Line::from(""),
        Line::from(vec![Span::styled(app.save_file_dialog_title(), key)]),
        Line::from(""),
        Line::from(vec![Span::styled(path, base)]),
        Line::from(vec![Span::styled(file_name, file_style)]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Y", key),
            Span::styled(app.save_file_dialog_yes_label(), base),
        ]),
        Line::from(vec![
            Span::styled("N", key),
            Span::styled(app.save_file_dialog_no_label(), base),
        ]),
        Line::from(vec![
            Span::styled("Esc", key),
            Span::styled(app.save_file_dialog_cancel_label(), base),
        ]),
    ]);

    frame.render_widget(
        Paragraph::new(text)
            .style(Style::default().bg(CURRENT_THEME.dialog_background))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true }),
        content_area,
    );
}

fn draw_new_directory_dialog(frame: &mut Frame, app: &App, area: Rect) {
    draw_text_input_dialog(
        frame,
        area,
        "New sub-directory",
        Some(("Parent:", app.pending_new_directory_parent().unwrap_or_default())),
        "Name:",
        app.pending_new_directory_name().unwrap_or_default(),
        app.pending_new_directory_name().unwrap_or_default().chars().count(),
        "Create",
    );
}

fn draw_open_file_dialog(frame: &mut Frame, app: &App, area: Rect) {
    let value = app.open_file_input().unwrap_or_default();

    let base = Style::default()
        .fg(CURRENT_THEME.status_bar_fg)
        .bg(CURRENT_THEME.dialog_background);
    let accent = base.add_modifier(Modifier::BOLD);
    let key = Style::default()
        .fg(CURRENT_THEME.status_hotkey_fg)
        .bg(CURRENT_THEME.dialog_background)
        .add_modifier(Modifier::BOLD);

    let content_area = draw_dialog_shell(frame, area).inner(Margin {
        vertical: 0,
        horizontal: 1,
    });

    let lines = vec![
        Line::from(vec![Span::styled("Open file", key)]),
        Line::from(vec![
            Span::styled("Path:", accent),
            Span::styled(" ", base),
            Span::styled(value.to_string(), base),
        ]),
        Line::from(vec![
            Span::styled("Examples:", accent),
            Span::styled(" src/main.rs  /tmp/log.txt  ~/notes/todo.txt", base),
        ]),
        Line::from(vec![
            Span::styled("Enter", key),
            Span::styled(" open  ", base),
            Span::styled("Tab", key),
            Span::styled(" complete  ", base),
            Span::styled("Down", key),
            Span::styled(" browser  ", base),
            Span::styled("Esc", key),
            Span::styled(" cancel", base),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .style(Style::default().bg(CURRENT_THEME.dialog_background))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true }),
        content_area,
    );

    let label_width = "Path: ".chars().count();
    let cursor_col = app.open_file_input_cursor().unwrap_or(value.chars().count());
    let row_offset = 1;
    let cursor_x = content_area
        .x
        .saturating_add(label_width as u16)
        .saturating_add(cursor_col as u16);
    let cursor_y = content_area.y.saturating_add(row_offset);
    if cursor_x < content_area.x.saturating_add(content_area.width)
        && cursor_y < content_area.y.saturating_add(content_area.height)
    {
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

fn draw_regex_search_dialog(frame: &mut Frame, app: &App, area: Rect) {
    let pattern = app.search_pattern();
    let base = Style::default()
        .fg(CURRENT_THEME.status_bar_fg)
        .bg(CURRENT_THEME.dialog_background);
    let accent = base.add_modifier(Modifier::BOLD);
    let key = Style::default()
        .fg(CURRENT_THEME.status_hotkey_fg)
        .bg(CURRENT_THEME.dialog_background)
        .add_modifier(Modifier::BOLD);

    let content_area = draw_compact_dialog_shell(frame, area);

    let lines = vec![
        Line::from(vec![Span::styled("Regular expression search", key)]),
        Line::from(vec![
            Span::styled("Pattern:", accent),
            Span::styled(" ", base),
            Span::styled(pattern.to_string(), base),
        ]),
        Line::from(vec![
            Span::styled("Enter", key),
            Span::styled(" find next  ", base),
            Span::styled("Esc", key),
            Span::styled(" cancel", base),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .style(Style::default().bg(CURRENT_THEME.dialog_background))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true }),
        content_area,
    );

    let cursor_x = content_area
        .x
        .saturating_add("Pattern: ".chars().count() as u16)
        .saturating_add(app.search_pattern_cursor() as u16);
    let cursor_y = content_area.y.saturating_add(1);
    if cursor_x < content_area.x.saturating_add(content_area.width)
        && cursor_y < content_area.y.saturating_add(content_area.height)
    {
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

fn draw_browser_incremental_search_dialog(frame: &mut Frame, app: &App, area: Rect) {
    let pattern = app.search_pattern();
    let base = Style::default()
        .fg(CURRENT_THEME.status_bar_fg)
        .bg(CURRENT_THEME.dialog_background);
    let accent = base.add_modifier(Modifier::BOLD);
    let key = Style::default()
        .fg(CURRENT_THEME.status_hotkey_fg)
        .bg(CURRENT_THEME.dialog_background)
        .add_modifier(Modifier::BOLD);

    let content_area = draw_compact_dialog_shell(frame, area);

    let lines = vec![
        Line::from(vec![Span::styled("Incremental file search", key)]),
        Line::from(vec![
            Span::styled("Pattern:", accent),
            Span::styled(" ", base),
            Span::styled(pattern.to_string(), base),
        ]),
        Line::from(vec![
            Span::styled("Type", key),
            Span::styled(" to jump  ", base),
            Span::styled("Enter", key),
            Span::styled(" keep  ", base),
            Span::styled("Esc", key),
            Span::styled(" cancel", base),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .style(Style::default().bg(CURRENT_THEME.dialog_background))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true }),
        content_area,
    );

    let cursor_x = content_area
        .x
        .saturating_add("Pattern: ".chars().count() as u16)
        .saturating_add(app.search_pattern_cursor() as u16);
    let cursor_y = content_area.y.saturating_add(1);
    if cursor_x < content_area.x.saturating_add(content_area.width)
        && cursor_y < content_area.y.saturating_add(content_area.height)
    {
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

fn draw_browser_selection_pattern_dialog(frame: &mut Frame, app: &App, area: Rect) {
    let pattern = app.search_pattern();
    let title = app.browser_selection_pattern_title();
    let base = Style::default()
        .fg(CURRENT_THEME.status_bar_fg)
        .bg(CURRENT_THEME.dialog_background);
    let accent = base.add_modifier(Modifier::BOLD);
    let key = Style::default()
        .fg(CURRENT_THEME.status_hotkey_fg)
        .bg(CURRENT_THEME.dialog_background)
        .add_modifier(Modifier::BOLD);

    let content_area = draw_compact_dialog_shell(frame, area);

    let lines = vec![
        Line::from(vec![Span::styled(title, key)]),
        Line::from(vec![
            Span::styled("Pattern:", accent),
            Span::styled(" ", base),
            Span::styled(pattern.to_string(), base),
        ]),
        Line::from(vec![
            Span::styled("Note:", accent),
            Span::styled(" regex matches substrings; . matches any character; use \\. for a literal dot", base),
        ]),
        Line::from(vec![
            Span::styled("Enter", key),
            Span::styled(" apply  ", base),
            Span::styled("Esc", key),
            Span::styled(" cancel", base),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .style(Style::default().bg(CURRENT_THEME.dialog_background))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true }),
        content_area,
    );

    let cursor_x = content_area
        .x
        .saturating_add("Pattern: ".chars().count() as u16)
        .saturating_add(app.search_pattern_cursor() as u16);
    let cursor_y = content_area.y.saturating_add(1);
    if cursor_x < content_area.x.saturating_add(content_area.width)
        && cursor_y < content_area.y.saturating_add(content_area.height)
    {
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

fn draw_file_operation_name_dialog(frame: &mut Frame, app: &App, area: Rect) {
    let name = app.pending_file_operation_name().unwrap_or_default();
    let title = app.pending_file_operation_prompt_title().unwrap_or("New file name");

    let base = Style::default()
        .fg(CURRENT_THEME.status_bar_fg)
        .bg(CURRENT_THEME.dialog_background);
    let accent = base.add_modifier(Modifier::BOLD);
    let key = Style::default()
        .fg(CURRENT_THEME.status_hotkey_fg)
        .bg(CURRENT_THEME.dialog_background)
        .add_modifier(Modifier::BOLD);

    let content_area = draw_compact_dialog_shell(frame, area);

    let lines = vec![
        Line::from(vec![Span::styled(title.to_string(), key)]),
        Line::from(vec![
            Span::styled("Name:", accent),
            Span::styled(" ", base),
            Span::styled(name.to_string(), base),
        ]),
        Line::from(vec![Span::styled("Enter", key), Span::styled(" apply  ", base), Span::styled("Esc", key), Span::styled(" cancel", base)]),
    ];

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .style(Style::default().bg(CURRENT_THEME.dialog_background))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true }),
        content_area,
    );

    let cursor_x = content_area
        .x
        .saturating_add("Name: ".chars().count() as u16)
        .saturating_add(app.pending_file_operation_name_cursor().unwrap_or(name.chars().count()) as u16);
    let cursor_y = content_area.y.saturating_add(1);
    if cursor_x < content_area.x.saturating_add(content_area.width)
        && cursor_y < content_area.y.saturating_add(content_area.height)
    {
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

fn draw_text_input_dialog(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    detail: Option<(&str, String)>,
    field_label: &str,
    field_value: &str,
    cursor_col: usize,
    enter_action: &str,
) {
    let base = Style::default()
        .fg(CURRENT_THEME.status_bar_fg)
        .bg(CURRENT_THEME.dialog_background);
    let accent = base.add_modifier(Modifier::BOLD);
    let key = Style::default()
        .fg(CURRENT_THEME.status_hotkey_fg)
        .bg(CURRENT_THEME.dialog_background)
        .add_modifier(Modifier::BOLD);

    let content_area = draw_dialog_shell(frame, area).inner(Margin {
        vertical: 0,
        horizontal: 1,
    });
    let has_detail = detail.is_some();

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(title.to_string(), key)]),
        Line::from(""),
    ];

    if let Some((detail_label, detail_value)) = detail {
        lines.push(Line::from(vec![Span::styled(detail_label.to_string(), accent)]));
        lines.push(Line::from(vec![Span::styled(detail_value, base)]));
        lines.push(Line::from(""));
    }

    lines.extend([
        Line::from(vec![
            Span::styled(field_label.to_string(), accent),
            Span::styled(" ", base),
            Span::styled(field_value.to_string(), base),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("Enter", key), Span::styled(format!(" = {enter_action}"), base)]),
        Line::from(vec![Span::styled("Esc", key), Span::styled(" = Cancel", base)]),
    ]);

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .style(Style::default().bg(CURRENT_THEME.dialog_background))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true }),
        content_area,
    );

    let field_row = if has_detail { 6 } else { 3 };
    let cursor_x = content_area
        .x
        .saturating_add(format!("{field_label} ").chars().count() as u16)
        .saturating_add(cursor_col as u16);
    let cursor_y = content_area.y.saturating_add(field_row);
    if cursor_x < content_area.x.saturating_add(content_area.width)
        && cursor_y < content_area.y.saturating_add(content_area.height)
    {
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

fn draw_file_operation_dialog(frame: &mut Frame, app: &App, area: Rect) {
    let title = app
        .pending_file_operation_title()
        .unwrap_or("Confirm file operation?");
    let (source, target) = app
        .pending_file_operation_paths()
        .unwrap_or_else(|| (String::new(), None));

    let base = Style::default()
        .fg(CURRENT_THEME.status_bar_fg)
        .bg(CURRENT_THEME.dialog_background);
    let accent = base.add_modifier(Modifier::BOLD);
    let key = Style::default()
        .fg(CURRENT_THEME.status_hotkey_fg)
        .bg(CURRENT_THEME.dialog_background)
        .add_modifier(Modifier::BOLD);

    let content_area = draw_compact_dialog_shell(frame, area);

    let source_name = file_name_for_display(&source);
    let mut lines = vec![Line::from(vec![Span::styled(title, key)])];

    if let Some(target) = target {
        lines.push(Line::from(vec![
            Span::styled(source_name, accent),
            Span::styled(" -> ", base),
            Span::styled(truncate(&target, content_area.width.saturating_sub(4)), base),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("Delete ", base),
            Span::styled(source_name, accent),
            Span::styled(" ?", base),
        ]));
    }

    lines.push(Line::from(vec![
        Span::styled("Y/Enter", key),
        Span::styled(" confirm  ", base),
        Span::styled("N/Esc", key),
        Span::styled(" cancel", base),
    ]));

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .style(Style::default().bg(CURRENT_THEME.dialog_background))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true }),
        content_area,
    );
}

fn draw_file_conflict_dialog(frame: &mut Frame, app: &App, area: Rect) {
    let title = app.pending_file_conflict_title().unwrap_or("Target exists");
    let (source, target) = app
        .pending_file_conflict_paths()
        .unwrap_or_else(|| (String::new(), String::new()));

    let base = Style::default()
        .fg(CURRENT_THEME.status_bar_fg)
        .bg(CURRENT_THEME.dialog_background);
    let accent = base.add_modifier(Modifier::BOLD);
    let key = Style::default()
        .fg(CURRENT_THEME.status_hotkey_fg)
        .bg(CURRENT_THEME.dialog_background)
        .add_modifier(Modifier::BOLD);

    let content_area = draw_compact_dialog_shell(frame, area);
    let lines = vec![
        Line::from(vec![Span::styled(title, key)]),
        Line::from(vec![
            Span::styled(file_name_for_display(&source), accent),
            Span::styled(" conflicts with ", base),
            Span::styled(truncate(&target, content_area.width.saturating_sub(16)), base),
        ]),
        Line::from(vec![
            Span::styled("O", key),
            Span::styled(" overwrite  ", base),
            Span::styled("S", key),
            Span::styled(" skip  ", base),
            Span::styled("R", key),
            Span::styled(" rename  ", base),
            Span::styled("Esc", key),
            Span::styled(" cancel", base),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .style(Style::default().bg(CURRENT_THEME.dialog_background))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true }),
        content_area,
    );
}

fn anchored_file_operation_area(app: &App, root: Rect, width: u16, height: u16) -> Rect {
    let Some(browser_index) = app.pending_file_operation_browser_index() else {
        return centered(root, width, height);
    };

    let list_area = app.geometry.browser_inners[browser_index];
    let entries = app.browser_entries(browser_index);
    if list_area.width == 0 || list_area.height == 0 || entries.is_empty() {
        return centered(root, width, height);
    }

    let selected = app.browser_selected_entry(browser_index).min(entries.len().saturating_sub(1));
    let height_rows = list_area.height as usize;
    let start = selected.saturating_sub(height_rows.saturating_sub(1));
    let row = selected.saturating_sub(start) as u16;
    let label_width = displayed_browser_label(&entries[selected].label, entries[selected].is_directory())
        .chars()
        .count() as u16;
    let anchor_x = list_area
        .x
        .saturating_add(label_width.min(list_area.width.saturating_sub(1)))
        .saturating_add(1);
    let preferred_y = list_area.y.saturating_add(row).saturating_sub(height / 2);

    placed_rect_near(root, anchor_x, preferred_y, width, height)
}

fn anchored_search_area(app: &App, root: Rect, width: u16, height: u16) -> Rect {
    let Some((cursor_x, cursor_y)) = editor_cursor_screen_position(app) else {
        return centered(root, width, height);
    };

    placed_rect_near(root, cursor_x.saturating_add(1), cursor_y.saturating_add(1), width, height)
}

fn confirm_file_operation_dialog_height(app: &App) -> u16 {
    let has_target = app
        .pending_file_operation_paths()
        .and_then(|(_, target)| target)
        .is_some();
    if has_target { 5 } else { 5 }
}

fn draw_compact_dialog_shell(frame: &mut Frame, area: Rect) -> Rect {
    draw_dialog_shell(frame, area)
}

fn editor_cursor_screen_position(app: &App) -> Option<(u16, u16)> {
    let inner = app.geometry.editor_inner;
    if inner.width == 0 || inner.height == 0 {
        return None;
    }

    let line_number_width = app.editor_line_number_width();
    let text_cols = inner.width.saturating_sub(line_number_width + 1).max(1) as usize;
    let row_offset = app.editor.row_offset();
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
        Some((cursor_x, cursor_y))
    } else {
        None
    }
}

fn draw_dialog_shell(frame: &mut Frame, area: Rect) -> Rect {
    frame.render_widget(
        Block::default().style(Style::default().bg(CURRENT_THEME.dialog_border)),
        area,
    );

    let content_area = area.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });

    frame.render_widget(
        Block::default().style(Style::default().bg(CURRENT_THEME.dialog_background)),
        content_area,
    );

    content_area
}

fn placed_rect_near(root: Rect, preferred_x: u16, preferred_y: u16, width: u16, height: u16) -> Rect {
    let width = width.min(root.width.saturating_sub(2)).max(1);
    let height = height.min(root.height.saturating_sub(1)).max(1);
    let max_x = root.x + root.width.saturating_sub(width);
    let max_y = root.y + root.height.saturating_sub(height);
    let x = if preferred_x.saturating_add(width) <= root.x.saturating_add(root.width) {
        preferred_x
    } else {
        preferred_x.saturating_sub(width.saturating_sub(2))
    }
    .clamp(root.x, max_x);

    Rect {
        x,
        y: preferred_y.clamp(root.y, max_y),
        width,
        height,
    }
}

fn displayed_browser_label(label: &str, is_directory: bool) -> String {
    if is_directory {
        format!("[D] {label}")
    } else {
        format!("    {label}")
    }
}

fn file_name_for_display(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string()
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

fn compress_middle(value: &str, max_chars: usize) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= max_chars || max_chars <= 3 {
        return value.to_string();
    }

    let reserved = max_chars - 3;
    let prefix_len = reserved / 2;
    let suffix_len = reserved - prefix_len;

    let prefix = chars.iter().take(prefix_len).collect::<String>();
    let suffix = chars[chars.len().saturating_sub(suffix_len)..]
        .iter()
        .collect::<String>();
    format!("{prefix}...{suffix}")
}

fn render_editor_segment(
    line: &str,
    segment_start: usize,
    text_cols: usize,
    selection: Option<(usize, usize)>,
    full_width_selected: bool,
    current_line: bool,
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
            current_line,
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
                    current_line,
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
                current_line,
            }),
        );
    }

    if (full_width_selected || current_line) && text_cols > 0 {
        let rendered_cols = line.chars().skip(segment_start).take(text_cols).count();
        if rendered_cols < text_cols {
            let padding = " ".repeat(text_cols - rendered_cols);
            push_editor_run(
                &mut spans,
                &padding,
                RunStyle {
                    token: TokenKind::Plain,
                    selected: full_width_selected,
                    current_line,
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

fn tokenize_line(
    line: &str,
    keywords: &[&str],
    file_type: Option<&FileTypeSpec>,
) -> Vec<TokenKind> {
    let chars = line.chars().collect::<Vec<_>>();
    let mut kinds = vec![TokenKind::Plain; chars.len()];

    if file_type.is_some_and(|spec| is_markdown_file(spec)) && is_markdown_heading(line) {
        kinds.fill(TokenKind::Title);
        return kinds;
    }

    let comment_start = file_type.and_then(|spec| comment_start_for_line(spec, line));
    let comment_start_idx = comment_start.map(|start| line[..start].chars().count());

    if let Some(start) = comment_start_idx {
        for token_kind in &mut kinds[start..] {
            *token_kind = TokenKind::Comment;
        }
    }

    let limit = comment_start_idx.unwrap_or(chars.len());
    let mut idx = 0;

    while idx < limit {
        if is_identifier_start(chars[idx]) {
            let start = idx;
            idx += 1;
            while idx < limit && is_identifier_continue(chars[idx]) {
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

fn is_markdown_file(file_type: &FileTypeSpec) -> bool {
    matches!(file_type.extension, "md" | "markdown")
}

fn is_markdown_heading(line: &str) -> bool {
    let trimmed = line.trim_start();
    let hashes = trimmed.chars().take_while(|character| *character == '#').count();
    hashes > 0 && trimmed.chars().nth(hashes) == Some(' ')
}

fn is_identifier_start(character: char) -> bool {
    character == '_' || character.is_ascii_alphabetic()
}

fn is_identifier_continue(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}

fn push_editor_run(spans: &mut Vec<Span<'static>>, run: &str, style: RunStyle) {
    let background = if style.selected {
        CURRENT_THEME.editor_selection_bg
    } else if style.current_line {
        CURRENT_THEME.editor_current_line_bg
    } else {
        CURRENT_THEME.editor_text_bg
    };

    let rendered_style = match style.token {
        TokenKind::Comment => Style::default()
            .fg(CURRENT_THEME.editor_comment_fg)
            .bg(background),
        TokenKind::Identifier => Style::default()
            .fg(CURRENT_THEME.editor_identifier_fg)
            .bg(background),
        TokenKind::Keyword => Style::default()
            .fg(CURRENT_THEME.editor_text_fg)
            .bg(background)
            .add_modifier(Modifier::BOLD),
        TokenKind::Title => Style::default()
            .fg(CURRENT_THEME.editor_text_fg)
            .bg(background)
            .add_modifier(Modifier::BOLD),
        TokenKind::Plain => Style::default()
            .fg(CURRENT_THEME.editor_text_fg)
            .bg(background),
    };

    spans.push(Span::styled(run.to_string(), rendered_style));
}

#[cfg(test)]
mod tests {
    use super::{TokenKind, is_markdown_heading, render_editor_segment, tokenize_line};
    use crate::file_types::file_type_for_extension;

    #[test]
    fn markdown_heading_lines_are_tokenized_as_titles() {
        let spec = file_type_for_extension("md").expect("expected markdown file type");
        let kinds = tokenize_line("## Heading", spec.keywords, Some(spec));

        assert!(kinds.iter().all(|kind| *kind == TokenKind::Title));
    }

    #[test]
    fn markdown_heading_detection_requires_space_after_hashes() {
        assert!(is_markdown_heading("### Title"));
        assert!(!is_markdown_heading("###Title"));
    }

    #[test]
    fn current_line_rendering_fills_empty_segment_width() {
        let spans = render_editor_segment("", 0, 4, None, false, true, &[]);

        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content.as_ref(), "    ");
    }
}
