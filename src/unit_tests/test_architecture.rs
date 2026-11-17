//! Architecture guard: the dependency rule between `src/shared`, `src/discrete` and
//! `src/continuous`.
//!
//! - In `src/shared/`, every `crate::` path starts with `crate::shared`.
//! - In `src/continuous/`, every `crate::` path starts with `crate::shared` or
//!   `crate::continuous`. Root re-exports such as `crate::Position` are forbidden.
//! - In `src/discrete/`, no `crate::` path starts with `crate::continuous`.
//!
//! The check is a small hand-written scanner: comments and the contents of string and character
//! literals are blanked out, then every `crate::` path is reduced to its first segment. A grouped
//! import `use crate::{a::b, c::{d, e}}` yields one first segment per top-level item (`a`, `c`).
//! `super::` paths are not checked.
use std::{
    fs,
    path::{Path, PathBuf},
};

/// The three source trees and the first segments allowed after `crate::` in each of them.
#[derive(Clone, Copy, Debug)]
enum Tree {
    Shared,
    Discrete,
    Continuous,
}

impl Tree {
    fn dir_name(self) -> &'static str {
        match self {
            Tree::Shared => "shared",
            Tree::Discrete => "discrete",
            Tree::Continuous => "continuous",
        }
    }

    fn allows(self, head: &str) -> bool {
        match self {
            Tree::Shared => head == "shared",
            Tree::Continuous => head == "shared" || head == "continuous",
            Tree::Discrete => head != "continuous",
        }
    }
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Replace comments (line, doc and nested block comments) and the contents of string, raw string
/// and character literals with spaces. Newlines are kept so that line numbers are preserved, and
/// lifetimes (`'a`) are left untouched.
///
/// @ai-generated
fn blank_comments_and_literals(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let blank = |c: char| if c == '\n' { '\n' } else { ' ' };
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        // Line comment, including `///` and `//!`.
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                out.push(' ');
                i += 1;
            }
            continue;
        }
        // Block comment, possibly nested.
        if c == '/' && next == Some('*') {
            let mut depth = 0;
            while i < chars.len() {
                if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                    depth += 1;
                    out.push_str("  ");
                    i += 2;
                } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    out.push_str("  ");
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    out.push(blank(chars[i]));
                    i += 1;
                }
            }
            continue;
        }
        // Raw string `r"..."`, `r#"..."#`, `br"..."`: no escapes, ends with `"` and as many `#`.
        let starts_word = i == 0
            || !is_ident_char(chars[i - 1])
            || (chars[i - 1] == 'b' && (i == 1 || !is_ident_char(chars[i - 2])));
        if c == 'r' && starts_word {
            let mut j = i + 1;
            while chars.get(j) == Some(&'#') {
                j += 1;
            }
            if chars.get(j) == Some(&'"') {
                let hashes = j - i - 1;
                out.push_str(&" ".repeat(j - i + 1));
                i = j + 1;
                while i < chars.len() {
                    let closes =
                        chars[i] == '"' && (1..=hashes).all(|h| chars.get(i + h) == Some(&'#'));
                    if closes {
                        out.push_str(&" ".repeat(hashes + 1));
                        i += hashes + 1;
                        break;
                    }
                    out.push(blank(chars[i]));
                    i += 1;
                }
                continue;
            }
        }
        // String literal with escapes.
        if c == '"' {
            out.push(' ');
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' {
                    out.push(' ');
                    i += 1;
                }
                if i < chars.len() {
                    out.push(blank(chars[i]));
                    i += 1;
                }
            }
            out.push(' ');
            i += 1;
            continue;
        }
        // Character literal: `'x'` or `'\…'`. Anything else starting with `'` is a lifetime.
        if c == '\'' {
            if next == Some('\\') {
                // Skip the escaped character, which may itself be a quote: `'\''`.
                let mut j = i + 3;
                while j < chars.len() && chars[j] != '\'' {
                    j += 1;
                }
                out.push_str(&" ".repeat(j + 1 - i));
                i = j + 1;
                continue;
            }
            if next.is_some() && chars.get(i + 2) == Some(&'\'') {
                out.push_str("   ");
                i += 3;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

fn skip_whitespace(chars: &[char], mut i: usize) -> usize {
    while i < chars.len() && chars[i].is_whitespace() {
        i += 1;
    }
    i
}

fn read_ident(chars: &[char], mut i: usize) -> (String, usize) {
    let start = i;
    while i < chars.len() && is_ident_char(chars[i]) {
        i += 1;
    }
    (chars[start..i].iter().collect(), i)
}

/// The first segment after `crate::` of every `crate::` path in `code` (already blanked by
/// [`blank_comments_and_literals`]), with the line where that segment is. A grouped import
/// yields the first segment of each of its top-level items; a glob `crate::*` yields `*`.
///
/// @ai-generated
fn crate_path_heads(code: &str) -> Vec<(usize, String)> {
    let chars: Vec<char> = code.chars().collect();
    let line_at = |offset: usize| chars[..offset].iter().filter(|&&c| c == '\n').count() + 1;
    let mut heads = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let is_crate_keyword = chars[i..].starts_with(&['c', 'r', 'a', 't', 'e'])
            && (i == 0 || !is_ident_char(chars[i - 1]))
            && !chars.get(i + 5).is_some_and(|&c| is_ident_char(c));
        if !is_crate_keyword {
            i += 1;
            continue;
        }
        let j = skip_whitespace(&chars, i + 5);
        if !chars[j..].starts_with(&[':', ':']) {
            i += 5;
            continue;
        }
        let k = skip_whitespace(&chars, j + 2);
        match chars.get(k) {
            Some('{') => {
                // Top-level items of the group are at brace depth 1.
                let mut depth = 1;
                let mut item_head: Option<(usize, String)> = None;
                let mut m = k + 1;
                while m < chars.len() && depth > 0 {
                    match chars[m] {
                        '{' => depth += 1,
                        c @ ('}' | ',') if depth == 1 => {
                            heads.extend(item_head.take());
                            if c == '}' {
                                depth = 0;
                            }
                        }
                        '}' => depth -= 1,
                        '*' if depth == 1 && item_head.is_none() => {
                            item_head = Some((line_at(m), "*".into()));
                        }
                        c if depth == 1 && item_head.is_none() && is_ident_char(c) => {
                            let (ident, end) = read_ident(&chars, m);
                            item_head = Some((line_at(m), ident));
                            m = end;
                            continue;
                        }
                        _ => {}
                    }
                    m += 1;
                }
            }
            Some('*') => heads.push((line_at(k), "*".into())),
            Some(&c) if is_ident_char(c) => heads.push((line_at(k), read_ident(&chars, k).0)),
            _ => {}
        }
        i += 5;
    }
    heads
}

/// The forbidden `crate::` heads of `src` for `tree`, as `(line, head)`.
fn violations(tree: Tree, src: &str) -> Vec<(usize, String)> {
    crate_path_heads(&blank_comments_and_literals(src))
        .into_iter()
        .filter(|(_, head)| !tree.allows(head))
        .collect()
}

fn rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot read {dir:?}: {e}")) {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
}

/// Check every `.rs` file of `src/<tree>` and report all the violations at once.
///
/// @ai-generated
fn assert_tree_respects_the_rule(tree: Tree) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(tree.dir_name());
    let mut files = Vec::new();
    rust_files(&dir, &mut files);
    assert!(!files.is_empty(), "no Rust file found in {dir:?}");
    let mut report = Vec::new();
    for file in files {
        let src = fs::read_to_string(&file).unwrap();
        for (line, head) in violations(tree, &src) {
            let relative = file
                .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                .unwrap_or(&file);
            report.push(format!("{}:{line}: crate::{head}", relative.display()));
        }
    }
    assert!(
        report.is_empty(),
        "forbidden `crate::` paths in src/{}:\n{}",
        tree.dir_name(),
        report.join("\n")
    );
}

#[test]
fn shared_only_references_shared() {
    assert_tree_respects_the_rule(Tree::Shared);
}

#[test]
fn continuous_only_references_shared_and_continuous() {
    assert_tree_respects_the_rule(Tree::Continuous);
}

#[test]
fn discrete_never_references_continuous() {
    assert_tree_respects_the_rule(Tree::Discrete);
}

// ---------------------------------------------------------------------------
// Self-tests of the guard
// ---------------------------------------------------------------------------

fn heads(src: &str) -> Vec<String> {
    crate_path_heads(&blank_comments_and_literals(src))
        .into_iter()
        .map(|(_, head)| head)
        .collect()
}

#[test]
fn guard_extracts_heads_of_nested_multi_line_groups_and_inline_paths() {
    let src = r#"
use crate::{
    shared::{map::{MapSpec, ValidMap}, Position},
    continuous :: geometry::*,
    self,
};
use crate::*;
pub(crate) fn f() -> usize {
    let spec = crate::shared::map::MapSpec::parse("S0 X").unwrap();
    crate :: continuous::helper(&spec)
}
"#;
    assert_eq!(
        heads(src),
        ["shared", "continuous", "self", "*", "shared", "continuous"]
    );
}

#[test]
fn guard_accepts_a_compliant_continuous_file() {
    let src = r##"
//! Unlike `crate::core::World`, this module only uses `crate::shared`.
use super::{geometry::*, lasers::Beam};
use crate::{
    continuous::ContinuousError,
    shared::{event::WorldEvent, map::ValidMap},
};
/* A block comment with crate::World, /* nested: crate::Action */ still a comment. */
pub(crate) struct S<'a> {
    name: &'a str,
}
fn f<'a>(s: &'a str) -> &'a str {
    let quote = '"';
    let escaped = '\'';
    let text = "crate::discrete::World \" crate::Position";
    let raw = r#"crate::World "quoted" crate::Action"#;
    let map = crate::shared::map::MapSpec::parse(raw);
    super::super::helper(s)
}
"##;
    assert!(violations(Tree::Continuous, src).is_empty());
}

#[test]
fn guard_reports_root_re_exports_and_discrete_items_in_continuous() {
    let src = r#"
use crate::{
    shared::Grid,
    discrete::{
        World,
        Action,
    },
};
fn f() {
    let p = crate::Position::new2d(0, 0);
}
"#;
    assert_eq!(
        violations(Tree::Continuous, src),
        [(4, "discrete".to_string()), (10, "Position".to_string())]
    );
}

#[test]
fn guard_sees_code_after_a_nested_comment_and_quote_char() {
    // If nested comments or the `'"'` char literal were mishandled, the `use` would be swallowed.
    let src = "/* outer /* inner */ still outer */ use crate::core::World;\n\
               let q = '\"'; use crate::Action;";
    assert_eq!(
        violations(Tree::Continuous, src),
        [(1, "core".to_string()), (2, "Action".to_string())]
    );
}

#[test]
fn guard_reports_shared_depending_on_another_tree() {
    let src = "use crate::shared::map::MapSpec;\nuse crate::continuous::ContinuousWorld;\n\
               fn f() { crate::discrete::World::get_level(1); }";
    assert_eq!(
        violations(Tree::Shared, src),
        [(2, "continuous".to_string()), (3, "discrete".to_string())]
    );
}

#[test]
fn guard_reports_discrete_depending_on_continuous_only() {
    let src = "use crate::{Position, shared::Grid, discrete::World};\n\
               use crate::{\n    continuous::ContinuousWorld,\n};\n\
               fn f() { let _ = crate :: continuous :: PhysicsParams::default(); }";
    assert_eq!(
        violations(Tree::Discrete, src),
        [(3, "continuous".to_string()), (5, "continuous".to_string())]
    );
}
