use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileTypeSpec {
    pub extension: &'static str,
    pub keywords: &'static [&'static str],
    pub comment_line_pattern: Option<&'static str>,
    pub run: Option<ToolInvocation>,
    pub build: Option<ToolInvocation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolInvocation {
    Cargo {
        subcommand: &'static str,
    },
    Program {
        program: &'static str,
        args: &'static [&'static str],
        pass_file_path: bool,
    },
}

const BASH_KEYWORDS: &[&str] = &[
    "break", "case", "continue", "coproc", "declare", "do", "done", "elif", "else",
    "esac", "export", "fi", "for", "function", "if", "in", "local", "readonly",
    "return", "select", "shift", "then", "time", "typeset", "unset", "until", "while",
];

const PYTHON_KEYWORDS: &[&str] = &[
    "and", "as", "assert", "async", "await", "break", "case", "class", "continue",
    "def", "del", "elif", "else", "except", "False", "finally", "for", "from",
    "global", "if", "import", "in", "is", "lambda", "match", "None", "nonlocal",
    "not", "or", "pass", "raise", "return", "True", "try", "while", "with", "yield",
];

const TYPESCRIPT_KEYWORDS: &[&str] = &[
    "abstract", "any", "as", "asserts", "async", "await", "boolean", "break", "case",
    "catch", "class", "const", "continue", "debugger", "declare", "default", "delete",
    "do", "else", "enum", "export", "extends", "false", "finally", "for", "from",
    "function", "get", "if", "implements", "import", "in", "infer", "instanceof",
    "interface", "is", "keyof", "let", "module", "namespace", "new", "null", "number",
    "object", "of", "override", "package", "private", "protected", "public", "readonly",
    "return", "set", "static", "string", "super", "switch", "symbol", "this", "throw",
    "true", "try", "type", "typeof", "undefined", "unique", "unknown", "using", "var",
    "void", "while", "with", "yield",
];

const OCAML_KEYWORDS: &[&str] = &[
    "and", "as", "assert", "begin", "class", "constraint", "do", "done", "downto",
    "else", "end", "exception", "external", "false", "for", "fun", "function", "functor",
    "if", "in", "include", "inherit", "initializer", "land", "lazy", "let", "lor",
    "lsl", "lsr", "lxor", "match", "method", "mod", "module", "mutable", "new", "nonrec",
    "object", "of", "open", "or", "private", "rec", "sig", "struct", "then", "to", "true",
    "try", "type", "val", "virtual", "when", "while", "with",
];

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

const FILE_TYPES: &[FileTypeSpec] = &[
    FileTypeSpec {
        extension: "rs",
        keywords: RUST_KEYWORDS,
        comment_line_pattern: Some("//"),
        run: Some(ToolInvocation::Cargo { subcommand: "run" }),
        build: Some(ToolInvocation::Cargo {
            subcommand: "build",
        }),
    },
    FileTypeSpec {
        extension: "sh",
        keywords: BASH_KEYWORDS,
        comment_line_pattern: Some("#"),
        run: Some(ToolInvocation::Program {
            program: "bash",
            args: &[],
            pass_file_path: true,
        }),
        build: Some(ToolInvocation::Program {
            program: "bash",
            args: &["-n"],
            pass_file_path: true,
        }),
    },
    FileTypeSpec {
        extension: "bash",
        keywords: BASH_KEYWORDS,
        comment_line_pattern: Some("#"),
        run: Some(ToolInvocation::Program {
            program: "bash",
            args: &[],
            pass_file_path: true,
        }),
        build: Some(ToolInvocation::Program {
            program: "bash",
            args: &["-n"],
            pass_file_path: true,
        }),
    },
    FileTypeSpec {
        extension: "py",
        keywords: PYTHON_KEYWORDS,
        comment_line_pattern: Some("#"),
        run: Some(ToolInvocation::Program {
            program: "python3",
            args: &[],
            pass_file_path: true,
        }),
        build: Some(ToolInvocation::Program {
            program: "python3",
            args: &["-m", "py_compile"],
            pass_file_path: true,
        }),
    },
    FileTypeSpec {
        extension: "scala",
        keywords: SCALA3_ALL_KEYWORDS,
        comment_line_pattern: Some("//"),
        run: Some(ToolInvocation::Program {
            program: "scala",
            args: &[],
            pass_file_path: true,
        }),
        build: Some(ToolInvocation::Program {
            program: "scalac",
            args: &[],
            pass_file_path: true,
        }),
    },
    FileTypeSpec {
        extension: "ts",
        keywords: TYPESCRIPT_KEYWORDS,
        comment_line_pattern: Some("//"),
        run: Some(ToolInvocation::Program {
            program: "tsx",
            args: &[],
            pass_file_path: true,
        }),
        build: Some(ToolInvocation::Program {
            program: "tsc",
            args: &["--noEmit"],
            pass_file_path: true,
        }),
    },
    FileTypeSpec {
        extension: "tsx",
        keywords: TYPESCRIPT_KEYWORDS,
        comment_line_pattern: Some("//"),
        run: Some(ToolInvocation::Program {
            program: "tsx",
            args: &[],
            pass_file_path: true,
        }),
        build: Some(ToolInvocation::Program {
            program: "tsc",
            args: &["--noEmit"],
            pass_file_path: true,
        }),
    },
    FileTypeSpec {
        extension: "lean",
        keywords: LEAN_KEYWORDS,
        comment_line_pattern: Some("--"),
        run: Some(ToolInvocation::Program {
            program: "lean",
            args: &[],
            pass_file_path: true,
        }),
        build: Some(ToolInvocation::Program {
            program: "lean",
            args: &[],
            pass_file_path: true,
        }),
    },
    FileTypeSpec {
        extension: "ml",
        keywords: OCAML_KEYWORDS,
        comment_line_pattern: None,
        run: Some(ToolInvocation::Program {
            program: "ocaml",
            args: &[],
            pass_file_path: true,
        }),
        build: Some(ToolInvocation::Program {
            program: "ocamlc",
            args: &["-c"],
            pass_file_path: true,
        }),
    },
    FileTypeSpec {
        extension: "mli",
        keywords: OCAML_KEYWORDS,
        comment_line_pattern: None,
        run: None,
        build: Some(ToolInvocation::Program {
            program: "ocamlc",
            args: &["-c"],
            pass_file_path: true,
        }),
    },
    FileTypeSpec {
        extension: "md",
        keywords: DEFAULT_KEYWORDS,
        comment_line_pattern: None,
        run: None,
        build: None,
    },
    FileTypeSpec {
        extension: "markdown",
        keywords: DEFAULT_KEYWORDS,
        comment_line_pattern: None,
        run: None,
        build: None,
    },
];

pub const DEFAULT_KEYWORDS: &[&str] = &[];
static COMMENT_REGEXES: OnceLock<Vec<Option<Regex>>> = OnceLock::new();

pub fn file_type_for_extension(extension: &str) -> Option<&'static FileTypeSpec> {
    FILE_TYPES.iter().find(|spec| spec.extension == extension)
}

pub fn comment_start_for_line(file_type: &FileTypeSpec, line: &str) -> Option<usize> {
    let regex = comment_regex_for(file_type)?;
    regex.find(line).map(|matched| matched.start())
}

pub fn detect_file_type(path: Option<&Path>, first_line: Option<&str>) -> Option<&'static FileTypeSpec> {
    path.and_then(file_type_for_path)
        .or_else(|| first_line.and_then(file_type_for_shebang))
}

pub fn file_type_for_path(path: &Path) -> Option<&'static FileTypeSpec> {
    let extension = path.extension()?.to_str()?;
    file_type_for_extension(extension)
}

pub fn file_type_for_shebang(line: &str) -> Option<&'static FileTypeSpec> {
    let line = line.trim();
    if !line.starts_with("#!") {
        return None;
    }

    let interpreter = parse_shebang_interpreter(line)?;

    if matches_interpreter(interpreter, &["bash", "sh", "zsh", "ksh", "dash"]) {
        return file_type_for_extension("sh");
    }

    if matches_interpreter(interpreter, &["python", "python3", "pypy", "pypy3"]) {
        return file_type_for_extension("py");
    }

    if matches_interpreter(interpreter, &["tsx", "ts-node", "deno", "bun"]) {
        return file_type_for_extension("ts");
    }

    if matches_interpreter(interpreter, &["lean", "lake"]) {
        return file_type_for_extension("lean");
    }

    if matches_interpreter(interpreter, &["ocaml"]) {
        return file_type_for_extension("ml");
    }

    None
}

fn parse_shebang_interpreter(line: &str) -> Option<&str> {
    let shebang = line.trim()[2..].trim();
    let mut parts = shebang.split_whitespace();
    let command = parts.next()?;
    let command_name = basename(command);

    if command_name == "env" {
        return parse_env_interpreter(parts);
    }

    Some(command_name)
}

fn parse_env_interpreter<'a>(parts: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let allow_options = true;

    for part in parts {
        if allow_options && part.starts_with('-') {
            continue;
        }

        if allow_options && part.contains('=') {
            continue;
        }

        return Some(basename(part));
    }

    None
}

fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn matches_interpreter(interpreter: &str, needles: &[&str]) -> bool {
    let interpreter = interpreter.to_ascii_lowercase();
    needles.iter().any(|needle| {
        interpreter == *needle
            || interpreter
                .strip_prefix(needle)
                .is_some_and(|suffix| suffix.starts_with('.'))
    })
}

fn comment_regex_for(file_type: &FileTypeSpec) -> Option<&Regex> {
    let regexes = COMMENT_REGEXES.get_or_init(|| {
        FILE_TYPES
            .iter()
            .map(|spec| {
                spec.comment_line_pattern.map(|pattern| {
                    Regex::new(pattern).expect("invalid comment regex in file type table")
                })
            })
            .collect()
    });

    let index = FILE_TYPES
        .iter()
        .position(|spec| spec.extension == file_type.extension)?;
    regexes[index].as_ref()
}

#[cfg(test)]
mod tests {
    use super::{comment_start_for_line, detect_file_type, file_type_for_extension, file_type_for_shebang};
    use std::path::Path;

    #[test]
    fn detects_bin_bash_shebang() {
        let spec = file_type_for_shebang("#!/bin/bash").expect("expected bash detection");
        assert_eq!(spec.extension, "sh");
    }

    #[test]
    fn detects_env_python_with_flags() {
        let spec = file_type_for_shebang("#!/usr/bin/env -S python3 -u")
            .expect("expected python detection");
        assert_eq!(spec.extension, "py");
    }

    #[test]
    fn detects_direct_python_versioned_binary() {
        let spec = file_type_for_shebang("#!/usr/local/bin/python3.12")
            .expect("expected python detection");
        assert_eq!(spec.extension, "py");
    }

    #[test]
    fn detects_env_lean_shebang() {
        let spec = file_type_for_shebang("#!/usr/bin/env lean")
            .expect("expected lean detection");
        assert_eq!(spec.extension, "lean");
    }

    #[test]
    fn falls_back_to_shebang_when_extension_is_unknown() {
        let path = Path::new("script.custom");
        let spec = detect_file_type(Some(path), Some("#!/bin/bash"))
            .expect("expected shebang fallback");
        assert_eq!(spec.extension, "sh");
    }

    #[test]
    fn detects_markdown_by_extension() {
        let spec = detect_file_type(Some(Path::new("README.md")), None)
            .expect("expected markdown detection");
        assert_eq!(spec.extension, "md");
    }

    #[test]
    fn finds_hash_comments_for_bash() {
        let spec = file_type_for_extension("sh").expect("expected shell file type");
        let start = comment_start_for_line(spec, "echo test # trailing comment");
        assert_eq!(start, Some(10));
    }

    #[test]
    fn finds_double_dash_comments_for_lean() {
        let spec = file_type_for_extension("lean").expect("expected lean file type");
        let start = comment_start_for_line(spec, "theorem demo := by -- explanation");
        assert_eq!(start, Some(19));
    }

    #[test]
    fn ocaml_has_no_line_comment_pattern() {
        let spec = file_type_for_extension("ml").expect("expected ocaml file type");
        let start = comment_start_for_line(spec, "let x = 1 (* not handled");
        assert_eq!(start, None);
    }
}