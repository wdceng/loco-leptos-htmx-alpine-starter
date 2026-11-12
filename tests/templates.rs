//! The pages' templates, read as source. cargo-leptos's hot reload parses
//! the `view!` markup with a parser of its own, which ends a tag at a bare
//! `>`: an attribute's value compared with `<` or `>` outside braces
//! (`disabled=quantity >= MAX`) prints the rest of its line into the page.
//! The compiled macro reads such a value right, so no rendered page in the
//! other tests can show it; this test reads the source instead: a value
//! that is more than a name, a literal or a call goes in braces.

use std::{
    fs,
    path::{Path, PathBuf},
};

/// Every `.rs` file under `dir`, in a stable order.
fn sources(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            found.extend(sources(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
    found
}

/// Whether `line` holds a template attribute whose value, outside braces,
/// goes on with a `<` or `>` operator: `name=value >= other`. An attribute
/// is told from Rust code by its `=` with no space on either side, which
/// rustfmt never leaves in code; a tag's own `>` right after a value
/// (`href=x>`) or on a line of its own is no operator.
fn unbraced_comparison(line: &str) -> bool {
    let code = line.trim_start();
    if code.starts_with("//") {
        return false;
    }
    let bytes = code.as_bytes();
    code.match_indices('=').any(|(at, _)| {
        let named = at > 0
            && (bytes[at - 1].is_ascii_alphanumeric() || matches!(bytes[at - 1], b'-' | b'_'));
        let valued = bytes
            .get(at + 1)
            .is_some_and(|next| !matches!(next, b' ' | b'{' | b'"' | b'=' | b'>'));
        if !(named && valued) {
            return false;
        }
        let Some((_, after)) = code[at + 1..].split_once(char::is_whitespace) else {
            return false;
        };
        let after = after.trim_start();
        after.starts_with("<=")
            || after.starts_with(">=")
            || after.starts_with("< ")
            || after.starts_with("> ")
    })
}

#[test]
fn the_check_tells_a_comparison_from_markup() {
    for bad in [
        "    disabled=quantity >= MAX",
        "    disabled=quantity <= 1",
        "<button disabled=count > 9 class=X>",
        "    hidden=width < 600",
    ] {
        assert!(unbraced_comparison(bad), "missed: {bad}");
    }
    for good in [
        "    disabled={quantity >= MAX}",
        "    class=ROW",
        "<a href=link>Text</a>",
        "    aria-label=if count == 0 {",
        "    aria-label=move || if open.get() { \"a\" } else { \"b\" }",
        "    value=(quantity + 1).min(MAX).to_string()",
        "    let less = quantity.saturating_sub(1).max(1);",
        "    if count > 99 {",
        "// disabled=quantity >= MAX in a comment",
    ] {
        assert!(!unbraced_comparison(good), "flagged: {good}");
    }
}

#[test]
fn no_template_compares_outside_braces() {
    let offenders: Vec<String> = sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"))
        .iter()
        .flat_map(|path| {
            let text =
                fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            text.lines()
                .enumerate()
                .filter(|(_, line)| unbraced_comparison(line))
                .map(|(number, line)| format!("{}:{}: {}", path.display(), number + 1, line.trim()))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "put these values in braces, `name={{…}}` (hot reload ends the tag at a bare `>`):\n{}",
        offenders.join("\n")
    );
}
