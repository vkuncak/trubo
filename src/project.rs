use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectEntryKind {
    Parent,
    Directory,
    File,
}

#[derive(Debug, Clone)]
pub struct ProjectEntry {
    pub path: PathBuf,
    pub label: String,
    pub kind: ProjectEntryKind,
}

impl ProjectEntry {
    pub fn is_directory(&self) -> bool {
        matches!(
            self.kind,
            ProjectEntryKind::Parent | ProjectEntryKind::Directory
        )
    }
}

pub fn list_directory(dir: &Path) -> Vec<ProjectEntry> {
    let mut directories = Vec::new();
    let mut files = Vec::new();

    if let Some(parent) = dir.parent() {
        directories.push(ProjectEntry {
            path: parent.to_path_buf(),
            label: "..".to_string(),
            kind: ProjectEntryKind::Parent,
        });
    }

    let Ok(entries) = fs::read_dir(dir) else {
        return directories;
    };

    let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            directories.push(ProjectEntry {
                label: format!("{}/", file_name(&path)),
                path,
                kind: ProjectEntryKind::Directory,
            });
        } else if file_type.is_file() {
            files.push(ProjectEntry {
                label: file_name(&path),
                path,
                kind: ProjectEntryKind::File,
            });
        }
    }

    directories.sort_by(|a, b| a.label.cmp(&b.label));
    files.sort_by(|a, b| a.label.cmp(&b.label));
    directories.extend(files);
    directories
}

pub fn directory_subtree_lines_with_size(
    dir: &Path,
    max_entries: usize,
    known_size: Option<u64>,
) -> Vec<String> {
    let mut lines = vec![match known_size {
        Some(size) => format!("{}/ ({})", file_name(dir), format_byte_size(size)),
        None => format!("{}/", file_name(dir)),
    }];
    let mut remaining = max_entries.saturating_sub(1);

    let truncated = append_subtree(dir, "", &mut remaining, &mut lines);
    if truncated {
        lines.push("...".to_string());
    }

    lines
}

pub fn subtree_size(path: &Path) -> u64 {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return 0;
    };

    if metadata.is_file() {
        return metadata.len();
    }

    if !metadata.is_dir() {
        return 0;
    }

    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };

    entries
        .filter_map(Result::ok)
        .map(|entry| subtree_size(&entry.path()))
        .sum()
}

pub fn format_byte_size(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];

    let mut value = size as f64;
    let mut unit_index = 0usize;
    while value >= 1024.0 && unit_index + 1 < UNITS.len() {
        value /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{size} {}", UNITS[unit_index])
    } else {
        format!("{value:.1} {}", UNITS[unit_index])
    }
}

fn append_subtree(
    dir: &Path,
    prefix: &str,
    remaining: &mut usize,
    lines: &mut Vec<String>,
) -> bool {
    let entries = child_entries(dir);

    for (index, entry) in entries.iter().enumerate() {
        if *remaining == 0 {
            return true;
        }

        *remaining -= 1;
        let is_last = index + 1 == entries.len();
        let branch = if is_last { "└── " } else { "├── " };
        lines.push(format!("{prefix}{branch}{}", entry.label));

        if entry.kind == ProjectEntryKind::Directory {
            let child_prefix = format!("{prefix}{}", if is_last { "    " } else { "│   " });
            if append_subtree(&entry.path, &child_prefix, remaining, lines) {
                return true;
            }
        }
    }

    false
}

fn child_entries(dir: &Path) -> Vec<ProjectEntry> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut directories = Vec::new();
    let mut files = Vec::new();

    let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            directories.push(ProjectEntry {
                label: format!("{}/", file_name(&path)),
                path,
                kind: ProjectEntryKind::Directory,
            });
        } else if file_type.is_file() {
            files.push(ProjectEntry {
                label: file_name(&path),
                path,
                kind: ProjectEntryKind::File,
            });
        }
    }

    directories.sort_by(|a, b| a.label.cmp(&b.label));
    files.sort_by(|a, b| a.label.cmp(&b.label));
    directories.extend(files);
    directories
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| path.display().to_string())
}
