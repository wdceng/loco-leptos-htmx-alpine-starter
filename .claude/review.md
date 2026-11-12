# The starter's names for /review

| Pass | Starter |
|---|---|
| 1 | [docs/one-job.md](../docs/one-job.md), `tests/one_job.rs`; helpers `render_page`, `Notice`, `FIELD`, `TITLE`, `paths::assets`, `sql::pool`, `settings::is_local`, `hashing`, `Page::answer_with` |
| 3 | `hx-sync="this:drop"`, `open = true`; `sqlx::query!` on `sql::pool`, `sql::write`; English; `paths::assets`; `src/islands.rs`, `--split`; `e2e/run.sh`, `src-tauri/*.sh`, `tools/` |
| 4 | [docs/security.md](../docs/security.md); `Signed`, `Admin` (`src/controllers/signed.rs`); `cross_site::refused`; `site_bucket`, `auth_bucket`, `files_bucket` |
| 5 | `Job`, `HELPERS` (`src/islands.rs`); `$$dispose` (`public/js/htmx-islands.js`); `run.sh`, `screens.sh`, `links.sh` |
| 6 | `public/js/htmx-islands.js`, `public/js/safe-area.js`; `hx-boost:inherited` (`src/views/layout.rs`) |
| 7 | `Navigation`; `min-h-11`; `PageMeta::refused` |
| 8 | no WASM budget: the `hydrate` build's dependencies and `web-sys` features; `#[island(lazy)]` |
| 9 | [docs/architecture.md](../docs/architecture.md), Conventions; `hashing`; `sql::write`; spawns at boot: `maintenance.rs`, `rate_limit.rs`, `deploy_checks.rs` |
| 10 | `.into_any()` in `src/views/layout.rs` |
| 11 | `tests/config.rs`, `config/dev-server.yaml` beside `config/staging.yaml`; `src/controllers/llms.rs` |
| 12 | `docs/architecture.md`; `docs/dev-server.md`, `docs/staging.md`, `docs/production.md`; `docs/decisions.md` |
| 13 | the `e2e` skill; `app` (no iOS lane) |
| 15 | `uploads.rs` and its limit |
| 16 | `src/maintenance.rs`, `LazyLock`, `src/uploads.rs`; no background workers |
| 17 | `group-data-*`, `aria-busy` with `busy:`, `BUTTON`; `BusyForm`'s `htmx:finally:request`; no `hx-select-oob` yet |

## Incidents

- 2026-11-11: Leptos's `debounce` kept a closure with the root owner each keystroke; `HELPERS` refuses it.
