//! Architecture guard: the dependency rule between `src/shared`, `src/discrete` and
//! `src/continuous`.
//!
//! - In `src/shared/`, every `crate::` path starts with `crate::shared`.
//! - In `src/continuous/`, every `crate::` path starts with `crate::shared` or
//!   `crate::continuous`. Root re-exports such as `crate::Position` are forbidden.
//! - In `src/discrete/`, no `crate::` path starts with `crate::continuous`.
//!
//! Each file is parsed with `syn`, so comments and literals are never mistaken for code. The
//! guard checks the first segment after `crate` in every path and `use` tree (one per top-level
//! item of a grouped import), and in the tokens of macro invocations. A `super::` path that climbs
//! out of the tree up to the crate root is checked like a `crate::` path: in
//! `src/continuous/world.rs`, `super::super::Position` is `crate::Position`.
//!
//! The test files wired with `#[path]` from `src/unit_tests/` are not under these directories, so
//! they are not checked: tests may use any part of the crate.
use std::{
    fs,
    path::{Path, PathBuf},
};

use proc_macro2::{TokenStream, TokenTree};
use syn::{
    ItemMod, ItemUse, Macro, UseTree,
    visit::{self, Visit},
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

/// Collects the first segment after the crate root of every path of a file.
///
/// @ai-generated
struct HeadCollector {
    /// The module path of the code being visited, from the crate root (e.g. `["continuous",
    /// "world"]`), extended by the inline `mod` blocks entered so far.
    module: Vec<String>,
    /// `(head, path)` for every path that reaches the crate root.
    heads: Vec<(String, String)>,
}

impl HeadCollector {
    /// Record the path `segments` if it reaches the crate root: either it starts with `crate`, or
    /// it starts with as many `super` as the depth of the current module (or more).
    ///
    /// @ai-generated
    fn record(&mut self, segments: &[String]) {
        let n_root = match segments.first().map(String::as_str) {
            Some("crate") => 1,
            Some("super") => {
                let n_super = segments.iter().take_while(|s| *s == "super").count();
                if n_super < self.module.len() {
                    return;
                }
                n_super
            }
            _ => return,
        };
        // A bare `crate` (e.g. `pub(crate)`) names the root itself, and the items of a group
        // such as `use crate::{...}` are recorded one by one.
        if let Some(head) = segments.get(n_root) {
            self.heads.push((head.clone(), segments.join("::")));
        }
    }

    /// Walk a `use` tree under the path `prefix`.
    ///
    /// @ai-generated
    fn use_tree(&mut self, prefix: &mut Vec<String>, tree: &UseTree) {
        match tree {
            UseTree::Path(path) => {
                prefix.push(path.ident.to_string());
                self.use_tree(prefix, &path.tree);
                prefix.pop();
            }
            UseTree::Name(name) => self.leaf(prefix, name.ident.to_string()),
            UseTree::Rename(rename) => self.leaf(prefix, rename.ident.to_string()),
            UseTree::Glob(_) => self.leaf(prefix, "*".to_string()),
            UseTree::Group(group) => {
                for item in &group.items {
                    self.use_tree(prefix, item);
                }
            }
        }
    }

    fn leaf(&mut self, prefix: &mut Vec<String>, name: String) {
        prefix.push(name);
        self.record(prefix);
        prefix.pop();
    }

    /// Record every `crate :: ident` sequence in the tokens of a macro invocation.
    ///
    /// @ai-generated
    fn tokens(&mut self, tokens: TokenStream) {
        let tokens: Vec<TokenTree> = tokens.into_iter().collect();
        for (i, token) in tokens.iter().enumerate() {
            match token {
                TokenTree::Group(group) => self.tokens(group.stream()),
                TokenTree::Ident(ident) if ident == "crate" => {
                    if let [
                        TokenTree::Punct(a),
                        TokenTree::Punct(b),
                        TokenTree::Ident(head),
                        ..,
                    ] = &tokens[i + 1..]
                        && a.as_char() == ':'
                        && b.as_char() == ':'
                    {
                        self.heads
                            .push((head.to_string(), format!("crate::{head} (in a macro)")));
                    }
                }
                _ => {}
            }
        }
    }
}

impl<'ast> Visit<'ast> for HeadCollector {
    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        self.use_tree(&mut Vec::new(), &item.tree);
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        if path.leading_colon.is_none() {
            let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
            self.record(&segments);
        }
        visit::visit_path(self, path);
    }

    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        if item.content.is_some() {
            self.module.push(item.ident.to_string());
            visit::visit_item_mod(self, item);
            self.module.pop();
        }
    }

    fn visit_macro(&mut self, mac: &'ast Macro) {
        self.tokens(mac.tokens.clone());
        visit::visit_macro(self, mac);
    }
}

/// The `(head, path)` of every path of `src` that reaches the crate root, for a file whose module
/// path is `module` (e.g. `["continuous", "world"]` for `src/continuous/world.rs`).
///
/// @ai-generated
fn root_heads(module: &[&str], src: &str) -> Vec<(String, String)> {
    let file = syn::parse_file(src).unwrap_or_else(|e| panic!("cannot parse: {e}"));
    let mut collector = HeadCollector {
        module: module.iter().map(|s| s.to_string()).collect(),
        heads: Vec::new(),
    };
    collector.visit_file(&file);
    collector.heads
}

/// The paths of `src` that break the rule of `tree`.
fn violations(tree: Tree, module: &[&str], src: &str) -> Vec<String> {
    root_heads(module, src)
        .into_iter()
        .filter(|(head, _)| !tree.allows(head))
        .map(|(_, path)| path)
        .collect()
}

fn rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
}

/// The module path of `file`, relative to `src/`: `src/shared/map/mod.rs` is `["shared", "map"]`.
///
/// @ai-generated
fn module_path(src_dir: &Path, file: &Path) -> Vec<String> {
    let relative = file.strip_prefix(src_dir).unwrap().with_extension("");
    let mut module: Vec<String> = relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    if module.last().is_some_and(|last| last == "mod") {
        module.pop();
    }
    module
}

/// Assert that no file under `src/<tree>/` breaks the rule of `tree`, listing every violation.
///
/// @ai-generated
fn assert_tree_respects_the_rule(tree: Tree) {
    let src_dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));
    let mut files = Vec::new();
    rust_files(&src_dir.join(tree.dir_name()), &mut files);
    files.sort();
    assert!(
        !files.is_empty(),
        "no Rust file under src/{}",
        tree.dir_name()
    );
    let mut report = Vec::new();
    for file in &files {
        let module = module_path(src_dir, file);
        let module: Vec<&str> = module.iter().map(String::as_str).collect();
        let src = fs::read_to_string(file).unwrap();
        for path in violations(tree, &module, &src) {
            report.push(format!("{}: {path}", file.display()));
        }
    }
    assert!(
        report.is_empty(),
        "dependency rule broken in src/{}:\n{}",
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

const CONTINUOUS_WORLD: &[&str] = &["continuous", "world"];

#[test]
fn guard_reports_root_re_exports_in_groups_paths_and_macros_but_not_in_comments() {
    let src = r#"
//! Unlike `crate::discrete::World`, this module only uses `crate::shared`.
use super::{geometry::*, lasers::Beam};
use crate::{
    continuous::ContinuousError,
    shared::{event::WorldEvent, map::ValidMap},
    discrete::{
        World,
        Action,
    },
};
/* A block comment with crate::World, /* nested: crate::Action */ still a comment. */
pub(crate) fn f() {
    let text = "crate::discrete::World";
    let map = crate::shared::map::MapSpec::parse(text);
    let p = crate::Position::new2d(0, 0);
    let v = vec![crate::Action::Stay];
}
"#;
    assert_eq!(
        violations(Tree::Continuous, CONTINUOUS_WORLD, src),
        [
            "crate::discrete::World",
            "crate::discrete::Action",
            "crate::Position::new2d",
            "crate::Action (in a macro)",
        ]
    );
}

#[test]
fn guard_reports_super_paths_that_climb_to_the_crate_root() {
    // In `continuous::world`, one `super` stays in `continuous`, two reach the crate root.
    let src = "
use super::geometry::centre;
use super::super::shared::Position;
use super::super::{continuous::ContinuousError, Action};
mod inner {
    // In `continuous::world::inner`, two `super` only reach `continuous`.
    use super::super::geometry::norm;
    fn f() { super::super::super::World::get_level(1); }
}
";
    assert_eq!(
        violations(Tree::Continuous, CONTINUOUS_WORLD, src),
        [
            "super::super::Action",
            "super::super::super::World::get_level"
        ]
    );
}

#[test]
fn guard_reports_shared_and_discrete_depending_on_another_tree() {
    let src = "use crate::shared::map::MapSpec;\nuse crate::continuous::ContinuousWorld;\n\
               fn f() { crate::discrete::World::get_level(1); }";
    assert_eq!(
        violations(Tree::Shared, &["shared", "map"], src),
        [
            "crate::continuous::ContinuousWorld",
            "crate::discrete::World::get_level"
        ]
    );
    assert_eq!(
        violations(Tree::Discrete, &["discrete", "world"], src),
        ["crate::continuous::ContinuousWorld"]
    );
}
