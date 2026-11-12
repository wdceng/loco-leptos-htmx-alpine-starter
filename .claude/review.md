# The starter's list for /review

The project's own names for the global `review` skill's passes. A new project copies this file and changes the names.

## 1 One-job drift

- Register: [docs/one-job.md](../docs/one-job.md). Guard file: `tests/one_job.rs`.
- Helpers a diff should use: `render_page`, `Notice`, `FIELD`, `TITLE`, `paths::assets`, `sql::pool`, `settings::is_local`, `hashing`, `Page::answer_with`.

## 3 Bugs

- Two requests at once: `open = true`, never a toggle; `hx-sync="this:drop"` on a form.
- Writes: an app query is `sqlx::query!` on `sql::pool`; writes that belong together are one `sql::write`.
- Language: English everywhere but the text a visitor reads.
- Links: a file in `public/` only through `paths::assets`.
- Islands only in `src/islands.rs`; every cargo-leptos command with `--split`.
- Shell scripts: `e2e/run.sh`, `src-tauri/*.sh`, `tools/`.

## 4 Security

- Security doc: [docs/security.md](../docs/security.md).
- Permission checks: the extractors `Signed` and `Admin` (`src/controllers/signed.rs`). Cross-site check: `cross_site::refused` (`sec-fetch-site`) on every post.
- Rate limits: `site_bucket`, `auth_bucket`, `files_bucket`.

## 5 Leaks

- Scheduling in an island: a `Job` makes its callback once; never `set_timeout`, `set_interval`, `request_animation_frame` or `debounce` (the root owner keeps their callbacks for the tab's life; `HELPERS` in `src/islands.rs` refuses them).
- An island htmx removes is freed by `$$dispose` in `public/js/htmx-islands.js`.
- Scripts that start processes: `run.sh`, `screens.sh`, `links.sh`.

## 6 htmx and Alpine under the CSP

- Hand-written helpers: the two in `public/js/`.
- `:inherited` example: the shell's `hx-boost:inherited` (`src/views/layout.rs`).

## 7 Accessibility

- Focus after a swap: `Navigation`. Tap target: `min-h-11`.
- A refused form: `PageMeta::refused` and `aria-describedby`.

## 8 Performance

- No bundle budget yet: the `hydrate` build's dependencies and `web-sys` features are the measure.
- Lazy islands: `#[island(lazy)]`.

## 9 Async

- Doc: the Conventions in [docs/architecture.md](../docs/architecture.md).
- Password hash: `hashing` alone (Argon2, four at once).
- Spawns are made at boot: `maintenance.rs`, `rate_limit.rs`, `deploy_checks.rs`. A check at boot may block.
- One transaction: `sql::write`.

## 10 Depth and props

- `.into_any()` example: `src/views/layout.rs`.

## 11 Policy and pages

- Config test: `tests/config.rs`; it also holds `config/dev-server.yaml` to `config/staging.yaml`.
- `/llms.txt`: `src/controllers/llms.rs`.

## 12 Docs

- Feature docs: `docs/architecture.md` and the doc the feature's row names. Decisions: `docs/decisions.md`.
- Parallel deploy docs: `docs/dev-server.md`, `docs/staging.md`, `docs/production.md`.

## 13 Tests

- Browser tests: the `e2e` skill; written, not run.
- The app's lane: `app` alone (no iOS lane).

## 15 Axum

- Uploads: `uploads.rs`, with its own body limit.

## 16 A framework's own tool

- Loco's scheduler beside the nightly restart (`src/maintenance.rs`); its cache (`ctx.cache`) beside a `LazyLock`; its storage (`ctx.storage`) beside the uploads on disk (`src/uploads.rs`). No background workers: work beside a request is spawned at boot.

## 17 One fact drawn twice

- One attribute switches: `group-data-*`, `aria-busy` with `busy:`, as `BUTTON` does.
- Swap list: the page's own swap; no `hx-select-oob` yet.
- Back to the server's state at a failure: `BusyForm`'s `htmx:finally:request`.

## Dated incidents

- 2026-11-11: Leptos's `debounce` left a closure with the root owner each keystroke; `HELPERS` in `src/islands.rs` now refuses it.
