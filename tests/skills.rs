//! The Loco skill's API indexes (`.claude/skills/loco/api-index.md`,
//! `sea-orm-index.md`) list one release's public names each, and
//! `SKILL.md` says which. A `Cargo.lock` past them makes the indexes miss
//! what a newer release added, so the line and the lock must agree; the
//! skill says what to do when they can't.
//!
//! The skills and docs are the template's, read by whoever clones it, so
//! they say "the maintainer", never a person's name; a person's own rules
//! live in their own `~/.claude/CLAUDE.md`.

use std::{
    fs,
    path::{Path, PathBuf},
};

/// A name the skills and docs once used for the maintainer, kept in two
/// halves so this file doesn't hold it.
const NAMES: [[&str; 2]; 1] = [["Dav", "or"]];

/// Every Markdown file under `dir`, at any depth.
fn markdown(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).unwrap_or_else(|_| panic!("{}", dir.display())) {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            found.extend(markdown(&path));
        } else if path.extension().is_some_and(|ext| ext == "md") {
            found.push(path);
        }
    }
    found
}

/// The version `Cargo.lock` holds for `package`.
fn locked(lock: &str, package: &str) -> String {
    let lines: Vec<&str> = lock.lines().collect();
    lines
        .iter()
        .position(|line| *line == format!("name = \"{package}\""))
        .and_then(|at| lines.get(at + 1))
        .and_then(|line| line.strip_prefix("version = \""))
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or_else(|| panic!("Cargo.lock: no version for {package}"))
        .to_owned()
}

#[test]
fn the_loco_skills_indexes_name_the_locked_versions() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lock = fs::read_to_string(root.join("Cargo.lock")).expect("Cargo.lock");
    let skill = root.join(".claude/skills/loco/SKILL.md");
    let text = fs::read_to_string(&skill).expect(".claude/skills/loco/SKILL.md");
    for package in ["loco-rs", "sea-orm"] {
        let version = locked(&lock, package);
        assert!(
            text.contains(&format!("{package} {version}'s")),
            "Cargo.lock holds {package} {version}, which {} doesn't name: regenerate the indexes for it, or write it there with the indexes' own release beside it",
            skill.display()
        );
    }
}

#[test]
fn the_skills_and_docs_name_no_one() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = markdown(&root.join(".claude"));
    files.extend(markdown(&root.join("docs")));
    files.extend(
        fs::read_dir(root)
            .expect("the repo root")
            .map(|entry| entry.expect("a directory entry").path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "md")),
    );
    let names: Vec<String> = NAMES.iter().map(|halves| halves.concat()).collect();
    let mut found = Vec::new();
    for file in &files {
        let text = fs::read_to_string(file).unwrap_or_else(|_| panic!("{}", file.display()));
        for (at, line) in text.lines().enumerate() {
            if names.iter().any(|name| line.contains(name.as_str())) {
                found.push(format!("{}:{}", file.display(), at + 1));
            }
        }
    }
    assert!(files.len() > 10, "too few files read: {}", files.len());
    assert!(
        found.is_empty(),
        "a person named where the maintainer is meant: {found:?}"
    );
}
