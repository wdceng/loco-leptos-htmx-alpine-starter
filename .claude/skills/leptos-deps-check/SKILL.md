---
name: leptos-deps-check
description: Inspect a Leptos app's framework stack for upstream bugs and issues that reach it. Leptos (every crate of leptos-rs/leptos) is the major; cargo-leptos, wasm-bindgen and wasm-split are the mid, checked every run; Leptos's other direct dependencies are the minor, on request ("minor" or "all"). Reads the versions from the app's Cargo.lock and its git pins, searches the GitHub issues, releases, the backport branch and the next major's tag, matches them against the app's code, and reports. Read-only. Use when asked to check Leptos or its dependencies for bugs, issues, unreleased fixes or what the next major fixes.
---

# Leptos dependency check

Run inside a Leptos app (its root holds `Cargo.lock`). Read-only: it reports
and stops. Nothing is edited, upgraded or filed.

axum, tokio and tower belong to `loco-deps-check`; htmx to `htmx-deps-check`.
Not checked twice here.

The project's list, `.claude/deps.md` (`/deps-check`), is read first where
it exists: what the app uses of Leptos, its pins and workarounds.

## Arguments

- none: major and mid.
- `minor`: minor only.
- `all`: major, mid and minor.

## Tiers

| Tier | Crates | Repository |
|---|---|---|
| Major | `leptos`, `leptos_macro`, `leptos_dom`, `leptos_server`, `leptos_axum`, `leptos_integration_utils`, `leptos_router`, `leptos_meta`, `leptos_config`, `tachys`, `reactive_graph`, `reactive_stores`, `server_fn`, `any_spawner`, `hydration_context`, `either_of` | `leptos-rs/leptos` (one repo, one search) |
| Mid | `cargo-leptos` (the installed tool) | `leptos-rs/cargo-leptos` |
| Mid | `wasm-bindgen`, `wasm-bindgen-futures`, `js-sys`, `web-sys` | `wasm-bindgen/wasm-bindgen` |
| Mid | `wasm_split_helpers`, `wasm_split_macros` (only with `--split` or `#[lazy]`) | `WorldSEnder/wasm-split-prototype` |
| Minor | every other direct dependency of `leptos` at the locked version (step 1) | each crate's own |

## Steps

### 1. Versions

Each crate's locked version. Leptos's crates have their own numbers
(leptos 0.8.22 ships tachys 0.2.19 and reactive_graph 0.2.16):

```sh
awk -v n="tachys" '$0=="name = \""n"\""{getline; gsub(/version = |"/,""); print; exit}' Cargo.lock
```

A git pin replaces the published crate. The check runs against the pin:

```sh
grep -n -A3 '^\[patch.crates-io\]' Cargo.toml
```

```sh
grep -A2 '^name = "leptos"$' Cargo.lock
```

The installed cargo-leptos, with the git rev when built from git:

```sh
cargo install --list | grep -A1 '^cargo-leptos'
```

Leptos's own dependency list at the locked version, for the minor tier:

```sh
grep -oE '^\[dependencies\.[a-zA-Z0-9_-]+\]' ~/.cargo/registry/src/*/leptos-<version>/Cargo.toml
```

### 2. Relevance

The app's Leptos profile decides which issues can reach it. Count the uses:

```sh
for p in 'island(lazy' '#[island' '#[lazy' '#[server' 'Suspense' 'Resource::' '<For ' '<Title' 'leptos_router' 'leptos_meta' 'reactive_stores' 'Owner::new' 'AutoReload' 'HydrationScripts' 'render_app_to_stream' 'leptos_routes' '<textarea'; do printf '%-22s %s\n' "$p" "$(grep -rF "$p" src crates --include=*.rs 2>/dev/null | wc -l)"; done
```

```sh
grep -n 'features = \|split\|hash-files\|lib-profile' Cargo.toml README.md
```

Drop the issues on features the app does not use, in one line each.

### 3. Upstream

The locked version's release date:

```sh
curl -s -A leptos-deps-check https://crates.io/api/v1/crates/leptos/0.8.22 | python3 -I -c "import json,sys;print(json.load(sys.stdin)['version']['created_at'])"
```

Releases, the 0.x line and the next major's tags:

```sh
gh release list --repo leptos-rs/leptos --limit 6
```

Open issues, most recently active first:

```sh
gh issue list --repo leptos-rs/leptos --state open --limit 200 --json number,title,updatedAt --jq '.[] | "\(.number) \(.updatedAt[0:10]) \(.title)"'
```

Issues closed since the locked release:

```sh
gh issue list --repo leptos-rs/leptos --state closed --search "closed:>=2026-10-05" --limit 60 --json number,title,closedAt,stateReason --jq '.[] | "\(.number) \(.closedAt[0:10]) \(.stateReason) \(.title)"'
```

Open any issue whose title might apply and read its body and comments before
judging it:

```sh
gh issue view 4976 --repo leptos-rs/leptos --json body,comments
```

The same three searches for the mid repos, narrowed by what the app uses
(`--search "split"`, `--search "hash"`).

cargo-leptos built from git: what `main` has that the installed rev does not
(the rev from step 1):

```sh
gh api repos/leptos-rs/cargo-leptos/compare/0fb3de86...main --jq '.commits[] | "\(.sha[0:8]) \(.commit.author.date[0:10]) \(.commit.message | split("\n")[0])"'
```

### 4. Where each fix is

For each fix that matters, find its PR and merge commit:

```sh
gh pr list --repo leptos-rs/leptos --state merged --search "4917" --json number,title,mergeCommit --jq '.[] | "\(.number) \(.mergeCommit.oid[0:8]) \(.title)"'
```

In a tag? `behind` or `identical` means yes, `diverged` or `ahead` means no:

```sh
gh api repos/leptos-rs/leptos/compare/v0.9.0-rc1...abf091ec --jq .status
```

The backport branch carries its own copies of the fixes, with other hashes.
Read its log and match by title:

```sh
gh api repos/leptos-rs/leptos/branches --paginate --jq '.[].name' | grep latest
```

```sh
gh api "repos/leptos-rs/leptos/commits?sha=leptos_0.8_latest&per_page=40" --jq '.[] | "\(.sha[0:8]) \(.commit.author.date[0:10]) \(.commit.message | split("\n")[0])"'
```

A backport is released only if the crate it touches has a published version
newer than the fix commit:

```sh
curl -s -A leptos-deps-check https://crates.io/api/v1/crates/reactive_graph | python3 -I -c "import json,sys;[print(v['num'],v['created_at'][:10]) for v in json.load(sys.stdin)['versions'][:5]]"
```

A file at a tag or branch, to confirm a fix is there:

```sh
gh api "repos/leptos-rs/leptos/contents/leptos/src/hydration/island_script.js?ref=v0.8.23" --jq .content | base64 -d
```

### 5. Next major

For every fix that is only in the next major's tag, ask two things:

- **Fixes:** which of the app's open problems and pins can go.
- **Breaks:** what the app does around today's bug that the fix makes wrong.
  Example: on 0.8 the app escapes `<textarea>` children itself; 0.9 escapes
  them too, so the text would be escaped twice.

### 6. Advisories

```sh
cargo audit
```

An advisory on a crate that Loco or Tauri brings in is reported once, under
its own skill (`loco-deps-check`, `tauri-deps-check`), not here.

### 7. Match to the app

For each candidate issue, find the API or setting it touches and grep the app
for it. Read the crate's source in `~/.cargo/registry` at the locked version
(or the pin's checkout in `~/.cargo/git/checkouts`) where the claim needs it.
An issue counts only with evidence: the file and line in the app that
reaches it, or the config that turns it on.

## Report

One table per tier:

| Crate (version) | Issue | Affects the app? | Evidence | Whose |
|---|---|---|---|---|

- **Affects the app?:** yes, no, or not checked, with the reason.
- **Evidence:** a markdown link to the app's file and line, or the config key.
- **Whose:** Leptos, the crate upstream, a browser engine, or the app (its
  own glue or use).

Then:

- **Fixed upstream, not yet in the app:** on the backport branch (released or
  not), or only in the next major's tag.
- **Pins and workarounds already in the app:** the code or pin that covers an
  open issue, and the release that lets it go.
- **On the next major:** what it fixes and what it breaks (step 5).
- **Not checked:** what was skipped and why.

Rules:

- Report only. Never edit, upgrade, comment or file an issue.
- Quote an issue only after reading it.
- Say plainly what is unverified.
- A bug inside a crate belongs to that crate; a browser bug (WebKit,
  Chromium) to the browser, even when it reaches the app through Leptos.
