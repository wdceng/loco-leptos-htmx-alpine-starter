# Security audit: the starter's own list

Read by `/security-audit`. The routes: `CARGO_TARGET_DIR=target/claude cargo loco routes`. The accepted advisories: `.cargo/audit.toml`, each with its reason. The vendored JavaScript: `public/vendor/` (htmx and its two extensions, Alpine's CSP build).

## Areas

1. **Sign-in and sessions**: `src/controllers/{login,register,verify,forgot,reset,account}.rs`; `src/models/{users,sessions,sign_ins,auth_events,roles}.rs`; `src/cookies.rs`, `src/secret.rs`, `src/hashing.rs`; the auth buckets in `src/middleware/rate_limit.rs`; Loco's JWT API unmounted (`src/controllers/auth.rs`, kept compiled).
2. **The public side**: no shop. The token pages (`/verify`, `/reset`, `render_token_page`), the 404 fallback (`not_found.rs`, `src/middleware/error_page.rs`), `/up`, `/robots.txt`, `/llms.txt`, `/.well-known/security.txt`, the manifest.
3. **The staff side**: no back office yet. The checks every post goes through: `Signed` and `Admin` (`src/controllers/signed.rs`), `SameSite` (`src/controllers/cross_site.rs`); the canaries in `tests/requests/canaries.rs`. The avatar upload: `src/uploads.rs`, `src/controllers/uploads.rs`.
4. **The browser and the app**: Leptos's escaping, the islands (`src/islands.rs`, `src/islands/`), the two helpers in `public/js/`, htmx's config (`src/views/layout.rs`); the app in `src-tauri/` (no capabilities file: the page calls nothing; `src/lib.rs`, the schemes handed to the system).
5. **Outbound calls and uploads**: mail only (`src/mailers/auth.rs`, `src/views/mail.rs`; links from `server.host`, never a request's `Host`). No push. The upload's limits: `src/uploads.rs`.
6. **The database, privacy and logs**: `sqlx::query!` on `sql::pool`, `sql::write` (`src/sql.rs`); the request log (`src/middleware/request_log.rs`, never a query); the tasks (`src/tasks/`).
7. **Config and deploy**: the six `config/*.yaml`; the middleware stack (`Hooks::middlewares`, `src/app.rs`); the boot checks (`src/deploy_checks.rs`); the units, the Caddy block, the firewall and the origin key in `docs/{production,staging,dev-server}.md`.

## The server

The maintainer's; its address is in `CLAUDE.local.md`. Read-only checks, never a write:

```sh
ssh <server> "stat -c '%a %U %n' /srv/app/*/secrets.env /var/lib/app-*/*"
```

Also `ss -ltnp` and `systemctl cat app-prod`. Never print a secrets file, `/etc/caddy/origin-check.caddy` or the origin key.

## Earlier audits

- 2026-10-18 and 2026-10-31: their findings and status in `docs/security.md` ("The review of …"), the decisions they took in `docs/decisions.md`. The 2026-10-31 one is the worked example.
