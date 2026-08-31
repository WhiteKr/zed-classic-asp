#![allow(dead_code)]

//! A workspace holding several site folders: each file must resolve
//! root-absolute includes against its own site root, not the workspace root.

use std::fs;
use std::path::{Path, PathBuf};

#[path = "../src/builtins.rs"]
mod builtins;
#[path = "../src/parse.rs"]
mod parse;
#[path = "../src/state.rs"]
mod state;

use state::State;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("asp-ls-test-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn root_absolute_includes_resolve_against_the_nearest_site_root() {
    let root = scratch("web-root");
    // Capitalized on purpose: real codebases mix `global.asa` and `Global.asa`.
    write(&root.join("site/Global.asa"), "");
    write(&root.join("site/includes/CommFun.asp"), "<% Sub Noop\nEnd Sub %>");
    write(
        &root.join("site/pages/index.asp"),
        r#"<!--#include virtual="/includes/CommFun.asp"-->"#,
    );
    // No marker anywhere above it: falls back to the workspace root.
    write(&root.join("loose/includes/Other.asp"), "");
    write(
        &root.join("loose/index.asp"),
        r#"<!--#include virtual="/includes/Other.asp"-->"#,
    );

    let mut state = State::new(State::canon(&root), None);
    state.scan_workspace();

    let page = State::canon(&root.join("site/pages/index.asp"));
    let resolved = state.index[&page].includes[0].resolved.as_ref();
    assert_eq!(
        resolved,
        Some(&State::canon(&root.join("site/includes/CommFun.asp"))),
        "virtual include should resolve under site/, not the workspace root"
    );

    let loose = State::canon(&root.join("loose/index.asp"));
    assert!(
        state.index[&loose].includes[0].resolved.is_none(),
        "without a marker the web root stays the workspace root, so /includes/Other.asp misses"
    );

    fs::remove_dir_all(&root).unwrap();
}
