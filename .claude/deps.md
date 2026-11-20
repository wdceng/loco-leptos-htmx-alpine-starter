# Dependency check: the starter's own list

Read by `/deps-check`. Three workspaces, each with its own lock: the root (`Cargo.lock`), the app (`src-tauri/Cargo.lock`), the browser tests (`e2e/Cargo.lock`). The update rules: `docs/prerequisites.md`, "Updates".

## Stacks

- `/loco-deps-check`: Loco with SQLite alone (no `db-postgres`, no `compression`), its mailer, sessions of the app's own (no JWT mounted).
- `/leptos-deps-check`: Leptos islands, `--split`, `leptos_axum`; cargo-leptos.
- `/htmx-deps-check`: `public/vendor/htmx-*.min.js`, with `hx-alpine-compat` and `hx-preload`.
- `/alpine-deps-check`: `public/vendor/alpinejs-csp-*.min.js`, no plugin.
- `/tauri-deps-check`: `src-tauri/`, with `tauri-plugin-opener`; the Android and iOS projects in `src-tauri/gen/`.

## Watched

- `sqlx`: the app's own queries (`sqlx::query!` on `sql::pool`), beside Sea-ORM's; its version is the one Sea-ORM asks for.
- `lettre`: the mail's boot checks (`src/settings.rs`, `src/deploy_checks.rs`) and Loco's mailer.
- `image`: the avatar (`src/uploads.rs`), its decoding limits.
- `tower_governor`, `governor`: the rate limits (`src/middleware/rate_limit.rs`).
- `thirtyfour`: the browser tests (`e2e/`).

## Accepted advisories

In `.cargo/audit.toml`, each with its reason:

- RUSTSEC-2024-0436, `paste` (via Leptos): unmaintained, compile-time only. Ends with a Leptos release that drops it.
- RUSTSEC-2026-0459, `libsqlite3-sys` 0.37.0 (via sqlx-sqlite 0.9.0). Ends with sqlx 0.9.1, which allows 0.39 (released 2026-11-17, qualifies 2026-12-02).

## Workarounds

- **sqlx #4434** (a connection a full disk left inside a transaction): `close_stuck` in `src/sql.rs`, through `sql::write`. Canary: `a_connection_stuck_in_a_transaction_is_closed_and_the_write_goes_on` (`tests/models/sql_pool.rs`). Fixed in sqlx 0.9.1 (#4473); once taken, `close_stuck` goes.
- **htmx #4147 and #4182** (`parseHTMLUnsafe` dropping Leptos's comments, then refused by older Chrome): `delete Document.parseHTMLUnsafe` in `public/js/htmx-islands.js`. Ends once Chrome 156 is below the low bar and an htmx release has #4182's fix.
- **htmx #4193** (the history state wiped): `keep_scroll` merges `htmx: true` (`HTMX_KEY`, `src/islands.rs`); `e2e/tests/moves.rs` checks it. Ends with a release that has #4198.

## Last run

None yet.
