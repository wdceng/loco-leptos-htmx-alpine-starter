---
name: migration
description: Use when adding or changing a table or a column in this starter: the migration, applying it, the SQLx cache, the queries, the tests and the docs. On top of the loco skill, which explains Loco's migrations.
---

# A table or a column

How Loco's migrations work: the loco skill (`.claude/skills/loco/`). Which queries are SQLx and which Sea-ORM: [docs/architecture.md](../../../docs/architecture.md), "Sea-ORM and SQLx". The commands: [docs/development.md](../../../docs/development.md), "After a migration, or a new or changed query".

1. **The migration.** `migration/src/m<YYYYMMDD_HHMMSS>_<what>.rs`, written with Write (a generator only in a scratch `git worktree`, each file brought over by hand), added to `migration/src/lib.rs` after the last one. The newest file there is the shape to follow. Loco's own `users` migration stays as generated.
2. **Apply it to the development database.** If Davor's watch loop runs (`pgrep -fl cargo-leptos`), its rebuild after the save starts the server, which migrates (`auto_migrate`); never a `cargo leptos build` beside it. Otherwise `cargo loco db migrate`.
3. **The SQLx cache**, a repo change Bash may make, against a copy of the development database in the scratchpad, never the file itself, which the watch loop's server holds (as an islands app on this stack does it):
   ```bash
   cp app_development.sqlite <scratchpad>/app_dev_copy.sqlite && { [[ ! -f app_development.sqlite-wal ]] || cp app_development.sqlite-wal <scratchpad>/app_dev_copy.sqlite-wal; }   # the copy, with its WAL where one stands beside it: a migration just applied can sit there
   SQLX_OFFLINE=false DATABASE_URL=sqlite://<scratchpad>/app_dev_copy.sqlite CARGO_TARGET_DIR=target/claude cargo sqlx prepare -- --all-targets   # .sqlx/ rewritten from the migrated schema, the tests' queries too
   ```
   `DATABASE_URL` on the one command, never exported (every config reads it, so `cargo test` would wipe that database). Always `-- --all-targets`. First check the copy has the newest migration: `sqlite3 -readonly <copy> "SELECT version FROM seaql_migrations ORDER BY version DESC LIMIT 1"` against the newest file in `migration/src/`. The maintainer's own command, the file itself, is in `docs/development.md`.
4. **The queries**: the app's own through `sqlx::query!` on `sql::pool(&ctx)`, every value bound; Loco's own models stay on Sea-ORM. A wrong column shows with `SQLX_OFFLINE=false DATABASE_URL=sqlite://app_development.sqlite cargo check --all-targets`.
5. **Tests**: a request test in `tests/requests/`, a model test in `tests/models/` (`sql_pool.rs` runs an own query on the test database); then fmt, both clippys, and nextest on the M1 with its own test database.
6. **Docs**: a sentence the change made false in `docs/architecture.md`; a choice of schema with its dated line in `docs/decisions.md`.
7. **The commit line**, `.sqlx/` and the migration in it.
