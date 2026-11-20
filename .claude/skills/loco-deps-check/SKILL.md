---
name: loco-deps-check
description: Inspect a Loco app's framework stack for upstream bugs and issues that reach it. Loco is the major; its mid dependencies (Sea-ORM, sqlx, axum, tower, tower-http, tokio, lettre, tracing, validator, axum-test) are checked every run, its minor ones on request ("minor" or "all"). Reads the versions from the app's Cargo.lock, searches each crate's GitHub issues and releases, matches them against the app's code, and reports. Read-only. Use when asked to check Loco or its dependencies for bugs, issues or advisories.
---

# Loco dependency check

Run inside a Loco app (its root holds `Cargo.lock`). Read-only: it reports
and stops. Nothing is edited, upgraded or filed.

The project's list, `.claude/deps.md` (`/deps-check`), is read first where
it exists: what the app uses of Loco, its accepted advisories, its
workarounds.

## Arguments

- none: major and mid.
- `minor`: minor only.
- `all`: major, mid and minor.

## Tiers

| Tier | Crates |
|---|---|
| Major | `loco-rs` |
| Mid | `sea-orm`, `sea-orm-migration`, `sqlx`, `axum`, `tower`, `tower-http`, `tokio`, `lettre`, `tracing`, `validator`, `axum-test` |
| Minor | every other direct dependency of `loco-rs` at the locked version (step 1) |

Minor, by the Loco feature it serves (1.2.0; the locked version's own list
wins):

| Feature | Crates |
|---|---|
| queue | `redis`, `bb8`, `bb8-redis` |
| storage | `opendal` |
| cache | `moka` |
| scheduler | `tokio-cron-scheduler`, `english-to-cron` |
| auth | `jsonwebtoken`, `argon2` |
| views | `tera` |
| config | `serde_yaml`, `toml` |
| CLI, generators | `clap`, `loco-gen`, `duct`, `cargo-lock` |
| HTTP extras | `axum-extra`, `axum-client-ip`, `rustls`, `http`, `bytes` |
| logging | `tracing-appender`, `tracing-subscriber`, `backtrace_printer` |
| other | `uuid`, `ulid`, `rand`, `url`, `regex`, `chrono`, `serde_json`, `thiserror`, `futures-util`, `dashmap`, `notify`, `scraper` |

## Steps

### 1. Versions

Each crate's locked version:

```sh
awk -v n="sqlx" '$0=="name = \""n"\""{getline; gsub(/version = |"/,""); print; exit}' Cargo.lock
```

More than one version of a crate in the lock is a finding of its own:

```sh
grep -c '^name = "axum"$' Cargo.lock
```

Loco's own dependency list at the locked version, for the minor tier:

```sh
grep -oE '^\[dependencies\.[a-zA-Z0-9_-]+\]' ~/.cargo/registry/src/*/loco-rs-<version>/Cargo.toml
```

### 2. Relevance

Drop what the app does not compile or use, and say so in one line each:

- Loco's features: the `loco-rs` line in the app's `Cargo.toml`.
- Config: no `queue:` block means no redis or bb8; no cloud storage means no
  opendal; `cache:` off means no moka.
- Code: grep the app for the crate's name.

The app may also use a mid crate directly, beside Loco (an islands app on
this stack runs `sqlx::query!` on Loco's pool). Note both uses; issues reach
the app through either.

### 3. Repository

From the crate's own manifest, then the repository's current name (repos move:
sqlx's manifest says `launchbadge/sqlx`, the repo is `transact-rs/sqlx`):

```sh
grep -m1 '^repository' ~/.cargo/registry/src/*/sqlx-0.9.0/Cargo.toml
```

```sh
gh api repos/launchbadge/sqlx --jq .full_name
```

### 4. Upstream

The locked version's release date:

```sh
curl -s -A loco-deps-check https://crates.io/api/v1/crates/sqlx/0.9.0 | python3 -I -c "import json,sys;print(json.load(sys.stdin)['version']['created_at'])"
```

Newer releases, and their notes where a newer one exists:

```sh
gh release list --repo transact-rs/sqlx --limit 5
```

Open issues, most recently active first:

```sh
gh issue list --repo transact-rs/sqlx --state open --limit 40 --json number,title,labels,updatedAt
```

Issues closed since the locked release (fixed upstream, maybe not in the app's version):

```sh
gh issue list --repo transact-rs/sqlx --state closed --search "closed:>2026-05-21" --limit 40 --json number,title,closedAt
```

Narrow by what the app uses (database driver, API names):

```sh
gh issue list --repo transact-rs/sqlx --state open --search "sqlite" --limit 20 --json number,title,labels,updatedAt
```

Open any issue whose title might apply and read its body before judging it.

### 5. Advisories

Whichever is installed:

```sh
cargo deny check advisories
```

```sh
cargo audit
```

### 6. Match to the app

For each candidate issue, find the API or setting it touches and grep the app
for it. Read the crate's source in `~/.cargo/registry` at the locked version
where the claim needs it. An issue counts only with evidence: the file and
line in the app that reaches it, or the config that turns it on.

## Report

One table per tier:

| Crate (version) | Issue | Affects the app? | Evidence | Whose |
|---|---|---|---|---|

- **Affects the app?:** yes, no, or not checked, with the reason.
- **Evidence:** a markdown link to the app's file and line, or the config key.
- **Whose:** Loco, the crate upstream, or the app (its own glue or use).

Then:

- **Fixed upstream, not yet in the app:** the newer release that has it.
- **Workarounds already in the app:** the code that covers an open issue (the list's, and any new one found).
- **Not checked:** what was skipped and why.

Rules:

- Report only. Never edit, upgrade, comment or file an issue.
- Quote an issue only after reading it.
- Say plainly what is unverified.
- A bug inside a crate belongs to that crate, not to Loco, even when it
  reaches the app through Loco.
