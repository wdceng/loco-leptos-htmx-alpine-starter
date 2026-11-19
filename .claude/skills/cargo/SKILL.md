---
name: cargo
description: Use when the maintainer asks to check the crates for upgrades (/cargo): every direct dependency of the current project's Cargo workspaces, its available major, minor and patch releases, and which are safe to take now and which are not, with the reason. Read-only: a table and the edits each safe one needs, nothing updated.
---

# Check the crates for upgrades

Read-only. No `cargo update`, no edit of a `Cargo.toml` or `Cargo.lock`: the report hands the maintainer the plan, and the session that edits the repo makes the changes on the maintainer's go.

The `*-deps-check` skills look for upstream bugs in a framework's stack (`/loco-deps-check`, `/leptos-deps-check`, `/tauri-deps-check`, `/htmx-deps-check`, `/alpine-deps-check`); this one looks for the upgrades across every crate. A framework's own upgrade (Loco, Leptos, Tauri, Sea-ORM) is named here and judged there.

## The levels

The maintainer's words beside Cargo's:

| Level | Cargo's | What changes |
|---|---|---|
| **major** | `1.x` → `2.0`, and for `0.x` a new minor, `0.9` → `0.10` | may break: code changes expected |
| **mid** | `1.4` → `1.5` | new features, compatible by semver's promise |
| **minor** | `1.4.2` → `1.4.3` | fixes only |

## Sources

- **The workspaces:** every `Cargo.toml` with a `[workspace]` or none above it in the project: the root, and the ones of their own (`src-tauri/`, `e2e/`, `migration/`), each with its own `Cargo.lock`.
- **The rules:** the project's `docs/prerequisites.md`, Updates (the starter's: monthly; a release at least two weeks old; security fixes at once; one major upgrade at a time, each its own commit; the versions within a major together), and its CLAUDE.md and the maintainer's rules (every dependency carries its three-number version, the one `Cargo.lock` holds, so an update edits both, never `cargo update` alone; `sqlx` stays at the version Sea-ORM depends on).
- **The project's list,** `.claude/deps.md` (`/deps-check`): the accepted advisories and the workarounds a release would end.
- **A testing period:** if the maintainer has called one, every verdict is "wait".

## Reading

The direct dependencies, each workspace, its members too:

```bash
cargo outdated --workspace --root-deps-only --format json --manifest-path <workspace>/Cargo.toml
```

`cargo outdated` resolves in a copy of its own and leaves the project's lock alone. `Compat` is the newest release the written requirement already allows (a mid or minor), `Latest` the newest of all (a major where they differ).

For each crate with a newer release, from crates.io (`https://crates.io/api/v1/crates/<name>/versions`, a `User-Agent` of the skill's name):

- the release's date, and whether it is yanked;
- its `rust_version` against `rustc -V`;
- its release notes or CHANGELOG, opened before quoting, for "breaking", a removed feature or an MSRV raise in a mid or minor.

Advisories, each workspace's lock:

```bash
cargo audit --file <workspace>/Cargo.lock
```

What holds a crate back, where an upgrade cannot resolve:

```bash
cargo tree -i <crate>@<version> --manifest-path <workspace>/Cargo.toml
```

A crate in the lock twice (`grep -c '^name = "<crate>"$' Cargo.lock`) is a finding of its own: two copies, and which crate keeps the old one.

## Verdicts

| Mark | Meaning |
|---|---|
| ✅ safe | a minor or mid, at least two weeks old, not yanked, its MSRV met, its notes with no breaking change |
| 🔒 now | an advisory's fix: take it outside the month |
| ⏳ wait | under two weeks old, or a testing period is on |
| ⚠️ major | a major: its own commit, its own plan, one at a time; the notes' breaking changes listed |
| 📌 held | pinned by the project's rule or by another crate (sqlx by Sea-ORM; a framework's crate by the framework), with what holds it |
| ❌ not now | a mid or minor whose notes say breaking, a raised MSRV the toolchain lacks, or one that resolves only with a held crate moved |

Crates that move together are judged together: a framework's family (`leptos`, `leptos_axum`, `leptos_meta`; `tauri`, `tauri-build`, `tauri-plugin-*`; `sea-orm`, `sea-orm-migration`), a crate and its `-macros` or `-derive`.

## Report

- One table a workspace: `Crate | Now | Safe to (mid/minor) | Latest (major) | Released | Verdict | Why`.
- Then the safe ones as the plan the editing session follows: for each, the line in `Cargo.toml` (and `migration/Cargo.toml`) with its new three-number version, then `cargo update -p <crate> --precise <version>` in that workspace, then the checks (`cargo fmt --check`, clippy on both halves, the suite). The minors and mids in one commit, a major in its own.
- Then the majors and the held ones, each in one line with what it would take, and the `*-deps-check` skill to run before a framework's.
- Then the duplicates and the advisories.

## Outstanding and verdict

The report ends with what is still open, then one verdict.

| Crate | State | Waits for | Earliest date |
|---|---|---|---|
| a release under two weeks old | ⏳ | its two weeks | the day it turns two weeks old |
| a held crate | 📌 | what frees it (Sea-ORM moving its sqlx, a framework's next release) | that release, or none known |
| a major | ⚠️ | its own plan and commit | the maintainer's call |
| a mid or minor that is not safe | ❌ | the fix its notes or MSRV need | the release or toolchain that brings it |
| a duplicate in the lock | ⚠️ | the crate that keeps the old copy | its next release, or none known |

Then the verdict, one line, with the count of each kind (safe now, waiting, held, majors, not now):

- 🟢 **all in line:** nothing to take until next month.
- 🟡 **update when convenient:** safe mids and minors waiting, none urgent.
- 🔴 **act now:** an advisory's fix, or a yanked release in the lock.
