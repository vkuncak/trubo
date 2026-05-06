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
    pub fn is_file(&self) -> bool {
        self.kind == ProjectEntryKind::File
    }

    pub fn is_directory(&self) -> bool {
        matches!(
            self.kind,
            ProjectEntryKind::Parent | ProjectEntryKind::Directory
        )
    }
}

pub fn list_project_dir(root: &Path, dir: &Path) -> Vec<ProjectEntry> {
    let mut directories = Vec::new();
    let mut files = Vec::new();

    if dir != root {
        let parent = dir.parent().unwrap_or(root).to_path_buf();
        directories.push(ProjectEntry {
            path: parent,
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
            if should_skip_dir(&path) {
                continue;
            }
            directories.push(ProjectEntry {
                label: format!("{}/", file_name(&path)),
                path,
                kind: ProjectEntryKind::Directory,
            });
        } else if is_editable_project_file(&path) {
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

pub fn is_editable_project_file(path: &Path) -> bool {
    if path
        .extension()
        .is_some_and(|extension| matches!(extension.to_str(), Some("rs" | "toml" | "lock")))
    {
        return true;
    }

    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, "Cargo.toml" | "Cargo.lock"))
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| path.display().to_string())
}

fn should_skip_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            matches!(
                name,
                ".git" | ".idea" | ".vscode" | "target" | "node_modules" | ".direnv"
            )
        })
}
