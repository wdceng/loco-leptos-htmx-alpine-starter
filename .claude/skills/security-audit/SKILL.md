---
name: security-audit
description: Use when Davor asks for a security audit, a pen inspection or a white-hat check of this starter ("audit the app for security", "check all endpoints for weaknesses", "think like a hacker"). A white-box read of the code, the configs and the deploy docs from the attacker's side, findings by severity, each Medium and up checked twice, no edit until his go. Not a pen test: nothing is sent at a live site.
---

# Security audit

A white-box audit: the source, `config/*.yaml` and the deploy docs read from the attacker's side (a visitor, someone with another person's mail link, a stolen JWT, another project's code on the shared box). Read-only. Not a pen test: no attack is sent at the dev-server, staging or production.

Inline, one focused pass, one subagent at most (CLAUDE.md, Working with Davor); the multi-agent workflow only on his word ("ultracode", "use a workflow"), and an agent runs no cargo command. A review of recent work is the `review` skill; this one reads the whole app.

## Scope

- The whole app, or the commits Davor names; ask when unclear.
- Every route: `CARGO_TARGET_DIR=target/claude cargo loco routes`.
- The dependencies: `cargo audit` (its ignores in `.cargo/audit.toml`, each with its reason), and the vendored htmx and Alpine in `public/vendor/` checked against their GitHub advisories.

## Areas, one pass each

1. **Sign-in and the auth API** (`src/controllers/auth.rs`, `src/models/users.rs`, `src/hashing.rs`): every way in without the password (verification, magic link, reset), what a JWT alone may do, its secret, algorithm and life, enumeration by words, status or time, the auth bucket (`rate_limit::auth_bucket`), addresses in any case (`normalized_email`).
2. **The pages and their forms** (`src/controllers/{reset,verify,home,not_found}.rs`, `src/render.rs`, `src/middleware/error_page.rs`): a token in an address (`render_token_page`), a GET that changes something, what a page echoes, the 404 fallback and the files (`Site`), `/up`, `/robots.txt`, `/llms.txt`, `/.well-known/security.txt`, the manifest.
3. **The browser and the app**: anything placed as HTML outside Leptos's escaping (`inner_html`, `set_inner_html`), Alpine expressions under its CSP build, htmx's config, the two helpers in `public/js/`, the islands, `src-tauri/` (no capabilities file, `destination` and `SYSTEM_SCHEMES`, its CSP).
4. **Mail** (`src/mailers/auth.rs`, `src/views/mail.rs`): what a visitor types reaching a mail (its subject, its body), links from `server.host`, never a request's `Host`.
5. **The database and the logs**: every query bound (`sqlx::query!` on `sql::pool`, Sea-ORM), transactions, the request log (`request_log`, never a query), personal data or a token in any log line, the tasks that could harm production (`user_create`).
6. **Config and deploy**: the six configs (CSP, headers, secrets and their defaults, bindings, rate limits, body and time limits), the middleware stack (`Hooks::middlewares`, src/app.rs), the boot checks (`deploy_checks`), the units, the Caddy block, the firewall, the origin key and Cloudflare's rules in `docs/{production,staging,dev-server}.md`.

## Yardsticks

- OWASP ASVS Level 2 for the app; NIST SP 800-63B-4 for the sign-in; the stricter where they differ, unless it costs people much for little, then a decision written down.
- What `docs/security.md` (its accepted risks, "When your app grows") or `docs/decisions.md` records as a reasoned decision is not reported again unless the reason is wrong.

## Severity

| | Meaning |
|---|---|
| High | an account, another person's data or code execution, with no precondition the attacker cannot meet |
| Medium | the same with a realistic precondition (a stolen token, a mail scanner, a shared PC, an old browser), or a defence the code relies on bypassed |
| Low | a hardening gap, defence in depth, a leak of little value |
| Info | worth knowing, nothing to do |

Every High and Medium is checked a second time by trying to refute it (the subagent, or the workflow's skeptic): confirmed, plausible with the step not established named, or dropped. Lows and Infos are marked unverified where they were not.

## The server

Davor runs the server; read-only checks may be run (`ssh <server> "stat -c '%a %U %n' /srv/app/*/secrets.env /var/lib/app-*/*"`, `ss -ltnp`, `systemctl cat app-prod`), never a write. Anything that changes the box is handed to him as a command with its proof and its way back. Never print a secrets file, `/etc/caddy/origin-check.caddy` or the origin key.

## Report

A table, most severe first: number, severity, a clickable file link with its line, the attack, the fix proposed; the server's findings with the check that proves them. Then one question: "go all", "go 1, 3" or "skip". Before the fixes, where a fix changes what a person sees, the choices with the best UX the sources give (OWASP, NIST, the vendors' docs, each opened before it is quoted), then a plan in plan mode. Nothing is edited before the go; after it, the fixes, `docs/security.md` updated in the same change (its review table and the rows the fixes change), the checks, and the commit line.

The review of 2026-10-31 is the worked example: its findings and their status in `docs/security.md`, the decisions it took in `docs/decisions.md`.
