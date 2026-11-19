---
name: security-audit
description: Use when the maintainer asks for a security audit, a pen inspection or a white-hat check of a project's web app ("audit the app for security", "check all endpoints for weaknesses", "think like a hacker"). A white-box read of the code, the configs and the deploy docs from the attacker's side, with the project's own list (.claude/security.md) first, findings by severity, each Medium and up checked twice, no edit until the maintainer's go. Not a pen test: nothing is sent at a live site.
---

# Security audit

A white-box audit: the source, the configs and the deploy docs read from the attacker's side (a visitor, a customer, a stolen staff session, another project's code on a shared box). Read-only. Not a pen test: no attack is sent at a dev-server, staging or production.

Inline, one focused pass, one subagent at most (the maintainer's rules: `CLAUDE.local.md`, `~/.claude/CLAUDE.md`); a multi-agent workflow only on the maintainer's word ("ultracode", "use a workflow"), and an agent runs no cargo command.

## The project's list first

Read `.claude/security.md` in the project's root: each area's files and names (its sign-in, its permission and cross-site checks, its upload, its push), the server and its read-only checks, the earlier audits and where their findings and decisions are written. No list: say so, run anyway, and propose one at the end (a new project copies the starter's).

## Scope

- The whole app, or the commits the maintainer names; ask when unclear.
- Every route (the framework's route list: `cargo loco routes` in a Loco app, built into the project's own target dir).
- The dependencies: `cargo audit`, and any vendored JavaScript checked against its GitHub advisories (`/deps-check` goes deeper).

## Areas, one pass each

1. **Sign-in and sessions**: every way in without the password and the second step, what a session alone may change, cookies, enumeration by words, status or time, rate limits and the per-account wait, open redirects.
2. **The public side**: prices and quantities from the server alone, one-time keys against a double submit, a private page by its token, mail headers, the 404 fallback and the files.
3. **The staff side**: every POST behind its permission check and the cross-site check, who may touch whom, uploads, the push endpoints.
4. **The browser and the app**: anything from the server placed as HTML outside the template's escaping, Alpine expressions, the islands' fetches, the service worker, htmx's config; a native shell: what the site's page may call (its capabilities), every command's input read before the phone keeps or runs it, what the phones put into their own scripts, the schemes handed to the system.
5. **Outbound calls and uploads**: push (allowed endpoints, no redirect), mail, an upload pipeline's limits and locks, a classifier's failure.
6. **The database, privacy and logs**: every query bound, foreign keys, transactions through the project's one way, what is kept and for how long against the privacy page, personal data in logs, the tasks that could harm production.
7. **Config and deploy**: every environment's config (CSP, headers, secrets and their defaults, bindings), the middleware stack, the units and the proxy's blocks in the deploy docs.

A project without one of these (no staff side, no app) says so in its list; the area is named in the report's clean line with that reason.

## Yardsticks

- OWASP ASVS Level 2 for the app; NIST SP 800-63B-4 for the sign-in; the stricter where they differ, unless it costs people much for little, then a decision written down.
- What the code's own docs record as a reasoned decision is not reported again unless the reason is wrong.

## Severity

| | Meaning |
|---|---|
| High | an account, another person's data, money or code execution, with no precondition the attacker cannot meet |
| Medium | the same with a realistic precondition (a stolen session, a shared PC, an old browser), or a defence the code relies on bypassed |
| Low | a hardening gap, defence in depth, a leak of little value |
| Info | worth knowing, nothing to do |

Every High and Medium is checked a second time by trying to refute it (the subagent, or the workflow's skeptic): confirmed, plausible with the step not established named, or dropped. Lows and Infos are marked unverified where they were not.

## The server

The maintainer runs the server; read-only checks may be run (the list's), never a write. Anything that changes the box is handed to the maintainer as a command with its proof; the way back only once the proof fails. Never print a secret: an origin key, a secrets file, a push key.

## Report

A table, most severe first: number, severity, a clickable file link with its line, the attack, the fix proposed; the server's findings with the check that proves them. Then one question: "go all", "go 1, 3" or "skip". Before the fixes, where a fix changes what a person sees, the choices with the best UX the sources give (OWASP, NIST, the vendors' docs, each opened before it is quoted), then a plan in plan mode. Nothing is edited before the go; after it, each group its own commit, the checks, and the commit line.

## The list's shape

`.claude/security.md`, in the project's git, short: a heading an area with its files and names, `## The server` with its read-only checks (the address in the maintainer's own `CLAUDE.local.md`, never in git), `## Earlier audits` with each one's date and where its findings and decisions are written. Nothing the skill already says is repeated there; no secret in it.
