use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::app::read_to_string;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone, Copy)]
enum Movement {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp(usize),
    PageDown(usize),
}

#[derive(Debug, Clone)]
struct UndoState {
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    row_offset: usize,
    row_segment_offset: usize,
    col_offset: usize,
    selection_anchor: Option<Position>,
    dirty: bool,
}

#[derive(Debug, Clone)]
pub struct Editor {
    path: Option<PathBuf>,
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    row_offset: usize,
    row_segment_offset: usize,
    col_offset: usize,
    viewport_rows: usize,
    viewport_cols: usize,
    selection_anchor: Option<Position>,
    undo_state: Option<UndoState>,
    dirty: bool,
}

impl Editor {
    pub fn scratch() -> Self {
        Self {
            path: None,
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            row_offset: 0,
            row_segment_offset: 0,
            col_offset: 0,
            viewport_rows: 18,
            viewport_cols: 72,
            selection_anchor: None,
            undo_state: None,
            dirty: false,
        }
    }

    pub fn open(path: &Path) -> io::Result<Self> {
        let content = read_to_string(path)?;
        let mut lines: Vec<String> = content.lines().map(ToString::to_string).collect();
        if content.ends_with('\n') || lines.is_empty() {
            lines.push(String::new());
        }

        Ok(Self {
            path: Some(path.to_path_buf()),
            lines,
            cursor_row: 0,
            cursor_col: 0,
            row_offset: 0,
            row_segment_offset: 0,
            col_offset: 0,
            viewport_rows: 18,
            viewport_cols: 72,
            selection_anchor: None,
            undo_state: None,
            dirty: false,
        })
    }

    pub fn save(&mut self) -> io::Result<()> {
        let Some(path) = &self.path else {
            return Err(io::Error::other("scratch buffer has no path"));
        };

        fs::write(path, self.lines.join("\n"))?;
        self.dirty = false;
        Ok(())
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn cursor_row(&self) -> usize {
        self.cursor_row
    }

    pub fn cursor_col(&self) -> usize {
        self.cursor_col
    }

    pub fn row_offset(&self) -> usize {
        self.row_offset
    }

    pub fn row_segment_offset(&self) -> usize {
        self.row_segment_offset
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn has_selection(&self) -> bool {
        self.selection_bounds().is_some()
    }

    pub fn set_cursor(&mut self, row: usize, col: usize) {
        let position = self.clamp_position(Position { row, col });
        self.set_cursor_position(position);
        self.clear_selection();
        self.keep_cursor_visible();
    }

    pub fn begin_selection(&mut self) {
        self.selection_anchor = Some(self.current_position());
    }

    pub fn select_to(&mut self, row: usize, col: usize) {
        if self.selection_anchor.is_none() {
            self.begin_selection();
        }
        let position = self.clamp_position(Position { row, col });
        self.set_cursor_position(position);
        if self.selection_anchor == Some(self.current_position()) {
            self.clear_selection();
        }
        self.keep_cursor_visible();
    }

    pub fn selection_bounds(&self) -> Option<(Position, Position)> {
        let anchor = self.selection_anchor?;
        let cursor = self.current_position();
        if anchor == cursor {
            return None;
        }

        let start = anchor.min(cursor);
        let end = anchor.max(cursor);
        Some((start, end))
    }

    pub fn selection_range_for_line(&self, row: usize) -> Option<(usize, usize)> {
        let (start, end) = self.selection_bounds()?;
        if row < start.row || row > end.row {
            return None;
        }

        let line_len = self.line_len(row);
        let (from, to) = if start.row == end.row {
            (start.col, end.col)
        } else if row == start.row {
            (start.col, line_len)
        } else if row == end.row {
            (0, end.col)
        } else {
            (0, line_len)
        };

        let from = from.min(line_len);
        let to = to.min(line_len);
        (from < to).then_some((from, to))
    }

    pub fn selected_text(&self) -> Option<String> {
        let (start, end) = self.selection_bounds()?;

        if start.row == end.row {
            return self
                .lines
                .get(start.row)
                .map(|line| slice_chars(line, start.col, end.col));
        }

        let mut text = String::new();
        let first_line = self.lines.get(start.row)?;
        text.push_str(&slice_chars(
            first_line,
            start.col,
            self.line_len(start.row),
        ));

        for row in start.row + 1..end.row {
            text.push('\n');
            if let Some(line) = self.lines.get(row) {
                text.push_str(line);
            }
        }

        text.push('\n');
        let last_line = self.lines.get(end.row)?;
        text.push_str(&slice_chars(last_line, 0, end.col));
        Some(text)
    }

    pub fn cut_selection(&mut self) -> Option<String> {
        let text = self.selected_text()?;
        self.delete_selection();
        Some(text)
    }

    pub fn move_left(&mut self) {
        self.move_cursor(Movement::Left, false);
    }

    pub fn move_right(&mut self) {
        self.move_cursor(Movement::Right, false);
    }

    pub fn move_up(&mut self) {
        self.move_cursor(Movement::Up, false);
    }

    pub fn move_down(&mut self) {
        self.move_cursor(Movement::Down, false);
    }

    pub fn home(&mut self) {
        self.move_cursor(Movement::Home, false);
    }

    pub fn end(&mut self) {
        self.move_cursor(Movement::End, false);
    }

    pub fn page_up(&mut self, rows: usize) {
        self.move_cursor(Movement::PageUp(rows), false);
    }

    pub fn page_down(&mut self, rows: usize) {
        self.move_cursor(Movement::PageDown(rows), false);
    }

    pub fn extend_left(&mut self) {
        self.move_cursor(Movement::Left, true);
    }

    pub fn extend_right(&mut self) {
        self.move_cursor(Movement::Right, true);
    }

    pub fn extend_up(&mut self) {
        self.move_cursor(Movement::Up, true);
    }

    pub fn extend_down(&mut self) {
        self.move_cursor(Movement::Down, true);
    }

    pub fn extend_home(&mut self) {
        self.move_cursor(Movement::Home, true);
    }

    pub fn extend_end(&mut self) {
        self.move_cursor(Movement::End, true);
    }

    pub fn extend_page_up(&mut self, rows: usize) {
        self.move_cursor(Movement::PageUp(rows), true);
    }

    pub fn extend_page_down(&mut self, rows: usize) {
        self.move_cursor(Movement::PageDown(rows), true);
    }

    pub fn insert_char(&mut self, character: char) {
        self.capture_undo_state();
        self.delete_selection();
        let cursor_col = self.cursor_col;
        let byte_idx = char_to_byte(&self.lines[self.cursor_row], cursor_col);
        self.lines[self.cursor_row].insert(byte_idx, character);
        self.cursor_col += 1;
        self.dirty = true;
        self.keep_cursor_visible();
    }

    pub fn insert_text(&mut self, text: &str) {
        let text = normalize_newlines(text);
        if text.is_empty() {
            return;
        }

        self.capture_undo_state();
        self.delete_selection();
        let parts = text.split('\n').collect::<Vec<_>>();
        if parts.len() == 1 {
            let byte_idx = char_to_byte(&self.lines[self.cursor_row], self.cursor_col);
            self.lines[self.cursor_row].insert_str(byte_idx, parts[0]);
            self.cursor_col += parts[0].chars().count();
        } else {
            let byte_idx = char_to_byte(&self.lines[self.cursor_row], self.cursor_col);
            let suffix = self.lines[self.cursor_row].split_off(byte_idx);
            self.lines[self.cursor_row].push_str(parts[0]);

            let mut insert_row = self.cursor_row + 1;
            for part in &parts[1..parts.len() - 1] {
                self.lines.insert(insert_row, (*part).to_string());
                insert_row += 1;
            }

            let last_part = parts.last().copied().unwrap_or_default();
            self.lines
                .insert(insert_row, format!("{last_part}{suffix}"));
            self.cursor_row = insert_row;
            self.cursor_col = last_part.chars().count();
        }

        self.dirty = true;
        self.keep_cursor_visible();
    }

    pub fn insert_newline(&mut self) {
        self.capture_undo_state();
        self.delete_selection();
        let cursor_col = self.cursor_col;
        let byte_idx = char_to_byte(&self.lines[self.cursor_row], cursor_col);
        let remainder = self.lines[self.cursor_row].split_off(byte_idx);
        self.cursor_row += 1;
        self.cursor_col = 0;
        self.lines.insert(self.cursor_row, remainder);
        self.dirty = true;
        self.keep_cursor_visible();
    }

    pub fn backspace(&mut self) {
        if self.selection_bounds().is_none() && self.cursor_col == 0 && self.cursor_row == 0 {
            self.keep_cursor_visible();
            return;
        }

        self.capture_undo_state();
        if self.delete_selection() {
            return;
        }

        if self.cursor_col > 0 {
            let remove_at = char_to_byte(&self.lines[self.cursor_row], self.cursor_col - 1);
            self.lines[self.cursor_row].remove(remove_at);
            self.cursor_col -= 1;
            self.dirty = true;
        } else if self.cursor_row > 0 {
            let current = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.line_len(self.cursor_row);
            self.lines[self.cursor_row].push_str(&current);
            self.dirty = true;
        }
        self.keep_cursor_visible();
    }

    pub fn delete(&mut self) {
        if self.selection_bounds().is_none()
            && self.cursor_col >= self.line_len(self.cursor_row)
            && self.cursor_row + 1 >= self.lines.len()
        {
            self.keep_cursor_visible();
            return;
        }

        self.capture_undo_state();
        if self.delete_selection() {
            return;
        }

        if self.cursor_col < self.line_len(self.cursor_row) {
            let remove_at = char_to_byte(&self.lines[self.cursor_row], self.cursor_col);
            self.lines[self.cursor_row].remove(remove_at);
            self.dirty = true;
        } else if self.cursor_row + 1 < self.lines.len() {
            let next = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].push_str(&next);
            self.dirty = true;
        }
        self.keep_cursor_visible();
    }

    pub fn delete_line(&mut self) {
        self.capture_undo_state();
        self.clear_selection();
        if self.lines.len() == 1 {
            self.lines[0].clear();
            self.cursor_col = 0;
        } else {
            self.lines.remove(self.cursor_row);
            self.cursor_row = self.cursor_row.min(self.lines.len() - 1);
            self.clamp_col();
        }
        self.dirty = true;
        self.keep_cursor_visible();
    }

    pub fn duplicate_line(&mut self) {
        self.capture_undo_state();
        self.clear_selection();
        let line = self.lines[self.cursor_row].clone();
        self.lines.insert(self.cursor_row + 1, line);
        self.cursor_row += 1;
        self.dirty = true;
        self.keep_cursor_visible();
    }

    pub fn undo(&mut self) -> bool {
        let Some(state) = self.undo_state.take() else {
            return false;
        };

        self.lines = state.lines;
        self.cursor_row = state.cursor_row;
        self.cursor_col = state.cursor_col;
        self.row_offset = state.row_offset;
        self.row_segment_offset = state.row_segment_offset;
        self.col_offset = state.col_offset;
        self.selection_anchor = state.selection_anchor;
        self.dirty = state.dirty;
        self.keep_cursor_visible();
        true
    }

    pub fn set_viewport(&mut self, rows: usize, cols: usize) {
        self.viewport_rows = rows;
        self.viewport_cols = cols;

        let cols = cols.max(1);
        let cursor_abs = self.abs_visual_row(self.cursor_row, self.cursor_col / cols, cols);
        let mut top_abs = self.abs_visual_row(self.row_offset, self.row_segment_offset, cols);

        if cursor_abs < top_abs {
            top_abs = cursor_abs;
        } else if rows > 0 && cursor_abs >= top_abs + rows {
            top_abs = cursor_abs.saturating_sub(rows - 1);
        }

        let (row_offset, row_segment_offset) = self.visual_row_to_offset(top_abs, cols);
        self.row_offset = row_offset;
        self.row_segment_offset = row_segment_offset;

        // Soft-wrap mode keeps rendering anchored at column 0.
        self.col_offset = 0;
    }

    fn move_cursor(&mut self, movement: Movement, selecting: bool) {
        if selecting {
            if self.selection_anchor.is_none() {
                self.begin_selection();
            }
        } else {
            self.clear_selection();
        }

        match movement {
            Movement::Left => self.step_left(),
            Movement::Right => self.step_right(),
            Movement::Up => self.step_up(),
            Movement::Down => self.step_down(),
            Movement::Home => self.cursor_col = 0,
            Movement::End => self.cursor_col = self.line_len(self.cursor_row),
            Movement::PageUp(rows) => {
                for _ in 0..rows {
                    self.step_up();
                }
            }
            Movement::PageDown(rows) => {
                for _ in 0..rows {
                    self.step_down();
                }
            }
        }

        if selecting && self.selection_anchor == Some(self.current_position()) {
            self.clear_selection();
        }
        self.keep_cursor_visible();
    }

    fn step_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.line_len(self.cursor_row);
        }
    }

    fn step_right(&mut self) {
        let len = self.line_len(self.cursor_row);
        if self.cursor_col < len {
            self.cursor_col += 1;
        } else if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }

    fn step_up(&mut self) {
        let cols = self.viewport_cols.max(1);
        let visual_segment = self.cursor_col / cols;
        let visual_col = self.cursor_col % cols;

        if visual_segment > 0 {
            self.cursor_col = (visual_segment - 1) * cols + visual_col;
            self.clamp_col();
            return;
        }

        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            let prev_len = self.line_len(self.cursor_row);
            let prev_rows = wrapped_rows(prev_len, cols);
            let prev_segment_start = (prev_rows.saturating_sub(1)) * cols;
            self.cursor_col = (prev_segment_start + visual_col).min(prev_len);
        }
    }

    fn step_down(&mut self) {
        let cols = self.viewport_cols.max(1);
        let visual_segment = self.cursor_col / cols;
        let visual_col = self.cursor_col % cols;
        let line_len = self.line_len(self.cursor_row);
        let visual_rows = wrapped_rows(line_len, cols);

        if visual_segment + 1 < visual_rows {
            self.cursor_col = ((visual_segment + 1) * cols + visual_col).min(line_len);
            return;
        }

        if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            let next_len = self.line_len(self.cursor_row);
            self.cursor_col = visual_col.min(next_len);
        }
    }

    fn delete_selection(&mut self) -> bool {
        let Some((start, end)) = self.selection_bounds() else {
            return false;
        };

        if start.row == end.row {
            let start_byte = char_to_byte(&self.lines[start.row], start.col);
            let end_byte = char_to_byte(&self.lines[end.row], end.col);
            self.lines[start.row].replace_range(start_byte..end_byte, "");
        } else {
            let start_byte = char_to_byte(&self.lines[start.row], start.col);
            let end_byte = char_to_byte(&self.lines[end.row], end.col);
            let suffix = self.lines[end.row][end_byte..].to_string();
            self.lines[start.row].truncate(start_byte);
            self.lines[start.row].push_str(&suffix);
            self.lines.drain(start.row + 1..=end.row);
        }

        self.set_cursor_position(start);
        self.clear_selection();
        self.dirty = true;
        self.keep_cursor_visible();
        true
    }

    fn clear_selection(&mut self) {
        self.selection_anchor = None;
    }

    fn capture_undo_state(&mut self) {
        self.undo_state = Some(UndoState {
            lines: self.lines.clone(),
            cursor_row: self.cursor_row,
            cursor_col: self.cursor_col,
            row_offset: self.row_offset,
            row_segment_offset: self.row_segment_offset,
            col_offset: self.col_offset,
            selection_anchor: self.selection_anchor,
            dirty: self.dirty,
        });
    }

    fn current_position(&self) -> Position {
        Position {
            row: self.cursor_row,
            col: self.cursor_col,
        }
    }

    fn set_cursor_position(&mut self, position: Position) {
        self.cursor_row = position.row;
        self.cursor_col = position.col;
    }

    fn clamp_position(&self, position: Position) -> Position {
        let row = position.row.min(self.lines.len().saturating_sub(1));
        let col = position.col.min(self.line_len(row));
        Position { row, col }
    }

    fn keep_cursor_visible(&mut self) {
        self.set_viewport(self.viewport_rows, self.viewport_cols);
    }

    fn clamp_col(&mut self) {
        self.cursor_col = self.cursor_col.min(self.line_len(self.cursor_row));
    }

    fn line_len(&self, row: usize) -> usize {
        self.lines
            .get(row)
            .map(|line| line.chars().count())
            .unwrap_or(0)
    }

    fn abs_visual_row(&self, row: usize, segment: usize, cols: usize) -> usize {
        let mut abs = 0;
        for idx in 0..row.min(self.lines.len()) {
            abs += wrapped_rows(self.line_len(idx), cols);
        }

        let max_segment = wrapped_rows(self.line_len(row), cols).saturating_sub(1);
        abs + segment.min(max_segment)
    }

    fn visual_row_to_offset(&self, mut abs: usize, cols: usize) -> (usize, usize) {
        for row in 0..self.lines.len() {
            let rows = wrapped_rows(self.line_len(row), cols);
            if abs < rows {
                return (row, abs);
            }
            abs -= rows;
        }

        let last_row = self.lines.len().saturating_sub(1);
        let last_segment = wrapped_rows(self.line_len(last_row), cols).saturating_sub(1);
        (last_row, last_segment)
    }
}

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn slice_chars(text: &str, start: usize, end: usize) -> String {
    let start = char_to_byte(text, start);
    let end = char_to_byte(text, end);
    text[start..end].to_string()
}

fn char_to_byte(text: &str, char_idx: usize) -> usize {
    text.char_indices()
        .nth(char_idx)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len())
}

fn wrapped_rows(line_len: usize, cols: usize) -> usize {
    if cols == 0 {
        return 1;
    }
    line_len.max(1).div_ceil(cols)
}

#[cfg(test)]
mod tests {
    use super::Editor;

    #[test]
    fn selects_and_cuts_across_lines() {
        let mut editor = Editor::scratch();
        editor.insert_text("alpha\nbeta\ngamma");
        editor.set_cursor(0, 2);
        editor.begin_selection();
        editor.select_to(2, 2);

        assert_eq!(editor.selected_text().as_deref(), Some("pha\nbeta\nga"));
        assert_eq!(editor.cut_selection().as_deref(), Some("pha\nbeta\nga"));
        assert_eq!(editor.lines(), &["almma".to_string()]);
    }

    #[test]
    fn paste_replaces_selection() {
        let mut editor = Editor::scratch();
        editor.insert_text("hello world");
        editor.set_cursor(0, 6);
        editor.begin_selection();
        editor.select_to(0, 11);
        editor.insert_text("rust");

        assert_eq!(editor.lines(), &["hello rust".to_string()]);
        assert!(!editor.has_selection());
    }

    #[test]
    fn mouse_cursor_uses_current_viewport_height() {
        let mut editor = Editor::scratch();
        editor.insert_text(
            &(0..60)
                .map(|line| format!("line {line}"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
        editor.set_cursor(0, 0);
        editor.set_viewport(40, 72);

        editor.set_cursor(30, 0);

        assert_eq!(editor.cursor_row(), 30);
        assert_eq!(editor.row_offset(), 0);
    }

    #[test]
    fn down_moves_within_wrapped_line_before_next_file_line() {
        let mut editor = Editor::scratch();
        editor.insert_text("abcdefghij\nxy");
        editor.set_viewport(10, 4);
        editor.set_cursor(0, 1);

        editor.move_down();
        assert_eq!(editor.cursor_row(), 0);
        assert_eq!(editor.cursor_col(), 5);

        editor.move_down();
        assert_eq!(editor.cursor_row(), 0);
        assert_eq!(editor.cursor_col(), 9);

        editor.move_down();
        assert_eq!(editor.cursor_row(), 1);
        assert_eq!(editor.cursor_col(), 1);
    }

    #[test]
    fn up_moves_within_wrapped_line_before_previous_file_line() {
        let mut editor = Editor::scratch();
        editor.insert_text("abcd\nabcdefghij");
        editor.set_viewport(10, 4);
        editor.set_cursor(1, 9);

        editor.move_up();
        assert_eq!(editor.cursor_row(), 1);
        assert_eq!(editor.cursor_col(), 5);

        editor.move_up();
        assert_eq!(editor.cursor_row(), 1);
        assert_eq!(editor.cursor_col(), 1);

        editor.move_up();
        assert_eq!(editor.cursor_row(), 0);
        assert_eq!(editor.cursor_col(), 1);
    }

    #[test]
    fn page_down_moves_by_wrapped_visual_rows() {
        let mut editor = Editor::scratch();
        editor.insert_text("abcdefghij\nxy");
        editor.set_viewport(10, 4);
        editor.set_cursor(0, 1);

        editor.page_down(2);
        assert_eq!(editor.cursor_row(), 0);
        assert_eq!(editor.cursor_col(), 9);

        editor.page_down(1);
        assert_eq!(editor.cursor_row(), 1);
        assert_eq!(editor.cursor_col(), 1);
    }

    #[test]
    fn page_up_moves_by_wrapped_visual_rows() {
        let mut editor = Editor::scratch();
        editor.insert_text("abcd\nabcdefghij");
        editor.set_viewport(10, 4);
        editor.set_cursor(1, 9);

        editor.page_up(2);
        assert_eq!(editor.cursor_row(), 1);
        assert_eq!(editor.cursor_col(), 1);

        editor.page_up(1);
        assert_eq!(editor.cursor_row(), 0);
        assert_eq!(editor.cursor_col(), 1);
    }

    #[test]
    fn page_down_updates_wrapped_viewport_offset() {
        let mut editor = Editor::scratch();
        editor.insert_text("abcdefghijklmnopqrstuvwxyz");
        editor.set_viewport(3, 4);
        editor.set_cursor(0, 0);

        editor.page_down(3);

        assert_eq!(editor.cursor_row(), 0);
        assert_eq!(editor.cursor_col(), 12);
        assert_eq!(editor.row_offset(), 0);
        assert_eq!(editor.row_segment_offset(), 1);
    }

    #[test]
    fn extend_up_creates_selection_across_visual_rows() {
        let mut editor = Editor::scratch();
        editor.insert_text("abcd\nabcdefghij");
        editor.set_viewport(10, 4);
        editor.set_cursor(1, 9);

        editor.extend_up();

        assert_eq!(editor.cursor_row(), 1);
        assert_eq!(editor.cursor_col(), 5);
        assert_eq!(editor.selected_text().as_deref(), Some("fghi"));
    }

    #[test]
    fn extend_down_creates_selection_across_visual_rows() {
        let mut editor = Editor::scratch();
        editor.insert_text("abcdefghij\nxy");
        editor.set_viewport(10, 4);
        editor.set_cursor(0, 1);

        editor.extend_down();

        assert_eq!(editor.cursor_row(), 0);
        assert_eq!(editor.cursor_col(), 5);
        assert_eq!(editor.selected_text().as_deref(), Some("bcde"));
    }

    #[test]
    fn undo_reverts_last_insert() {
        let mut editor = Editor::scratch();
        editor.insert_text("abc");

        assert!(editor.undo());
        assert_eq!(editor.lines(), &["".to_string()]);
        assert_eq!(editor.cursor_row(), 0);
        assert_eq!(editor.cursor_col(), 0);
    }

    #[test]
    fn undo_reverts_last_delete_only() {
        let mut editor = Editor::scratch();
        editor.insert_text("abcd");
        editor.backspace();

        assert!(editor.undo());
        assert_eq!(editor.lines(), &["abcd".to_string()]);

        // Single-step undo: no additional history beyond the last edit.
        assert!(!editor.undo());
    }
}
