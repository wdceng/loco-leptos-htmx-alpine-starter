//! The Loco skill's API indexes (`.claude/skills/loco/api-index.md`,
//! `sea-orm-index.md`) list one release's public names each, and
//! `SKILL.md` says which. A `Cargo.lock` past them makes the indexes miss
//! what a newer release added, so the line and the lock must agree; the
//! skill says what to do when they can't.

use std::{fs, path::Path};

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
