---
name: htmx-deps-check
description: Inspect an app's vendored htmx for upstream bugs and issues that reach it. htmx core is the major; the extensions the app loads (hx-preload, hx-alpine-compat, ...) are the mid; browser-specific issues against the app's oldest supported browsers take the minor's place. Reads the versions from the vendored files, searches bigskysoftware/htmx's issues, releases and the diff to the latest release, matches them against the app's code, and reports. Read-only. Use when asked to check htmx or its extensions for bugs, issues, regressions or an upgrade.
---

# htmx dependency check

Run inside an app that ships htmx (where it vendors it: the project's list,
`.claude/deps.md`, or `public/vendor/`). Read-only: it reports and stops.
Nothing is edited, upgraded or filed.

## Arguments

- none: major, mid and browsers.
- `upgrade`: also the shipped vs latest release table (step 5).

## Tiers

| Tier | What |
|---|---|
| Major | htmx core |
| Mid | each extension the app loads |
| Browsers | issues naming a browser or version, against the app's oldest supported browsers |

htmx has no dependencies of its own, so there is no minor tier.

## Steps

### 1. Versions

The vendored files and their versions:

```sh
find . -path ./target -prune -o -path ./node_modules -prune -o -path ./dist -prune -o -name "htmx*.js" -print -o -name "hx-*.js" -print
```

The version inside the core file, where the name does not carry it:

```sh
grep -oE 'version: *"[^"]+"' public/vendor/htmx-*.js
```

The extensions the app turns on, from its htmx config (the `htmx-config`
meta its page shell writes):

```sh
grep -rn '"extensions"' --include=*.rs --include=*.html --include=*.js . | grep -v target
```

Two versions of htmx or of one extension shipped at once is a finding of its own.

### 2. Relevance

Drop what the app does not use, one line each:

- an extension file vendored but not in `extensions`;
- a feature with no attribute, modifier or config key in the app (step 6).

Note any workaround the app already carries for htmx (the list's
workarounds; a hand-written helper in `public/js/`). Read its comments: they
name the issues it covers.

### 3. Browsers

The app's oldest supported browsers, from its docs or browser tests (its
low bar: the testing doc, the browser tests' runner). Keep the list for
step 4 and the report.

### 4. Upstream

Releases:

```sh
gh release list --repo bigskysoftware/htmx --limit 8
```

The shipped release's date:

```sh
gh release view v4.0.0 --repo bigskysoftware/htmx --json publishedAt --jq .publishedAt
```

Open issues, most recently active first:

```sh
gh issue list --repo bigskysoftware/htmx --state open --limit 200 --json number,title,labels,createdAt
```

Issues closed since the shipped release (fixed upstream, maybe not in the app's version):

```sh
gh issue list --repo bigskysoftware/htmx --state closed --search "closed:>2026-08-28" --limit 60 --json number,title,closedAt
```

Narrow by what the app uses (an extension, a browser):

```sh
gh issue list --repo bigskysoftware/htmx --state all --search "preload" --limit 30 --json number,title,state,updatedAt
```

```sh
gh issue list --repo bigskysoftware/htmx --state all --search "Chrome in:title,body" --limit 30 --json number,title,state,updatedAt
```

Every issue the app's code or comments name:

```sh
grep -rnoE '(bigskysoftware/htmx#|htmx ?#)[0-9]+' --include=*.rs --include=*.js --include=*.md . | grep -v target | sort -u
```

Open any issue whose title might apply and read its body before judging it.

Security advisories:

```sh
gh api repos/bigskysoftware/htmx/security-advisories --jq '.[] | "\(.ghsa_id) \(.severity) \(.summary)"'
```

### 5. Shipped vs latest (`upgrade`)

The latest release's notes:

```sh
gh release view v4.0.1 --repo bigskysoftware/htmx --json body --jq .body
```

What changed in the source between the two:

```sh
gh api repos/bigskysoftware/htmx/compare/v4.0.0...v4.0.1 --jq '.files[] | "\(.filename) +\(.additions) -\(.deletions)"'
```

One extension's diff:

```sh
gh api repos/bigskysoftware/htmx/compare/v4.0.0...v4.0.1 --jq '.files[] | select(.filename=="src/ext/hx-preload.js") | .patch'
```

A function in the latest release's core, where a claim needs it:

```sh
gh api "repos/bigskysoftware/htmx/contents/dist/htmx.js?ref=v4.0.1" --jq .content | base64 -d | grep -n "parseHTMLUnsafe" -A6
```

Issues opened against the latest release are its regressions: search its
version in the titles (`4.0.1`).

### 6. Match to the app

For each candidate issue, find the attribute, modifier, event or config key
it touches and grep the app for it:

```sh
grep -rn -- 'hx-sync' crates src public/js | grep -v target
```

Typical names: `hx-sync`, `delay:`, `throttle:`, `every `, `hx-on`,
`hx-boost`, `hx-preload`, `hx-push-url`, `hx-partial`, `morph`,
`morphSkip`, `history`, `htmx.ajax`, `htmx.findAll`, `:inherited`. An issue
counts only with evidence: the file and line in the app that reaches it, or
the config key that turns it on. A browser issue counts only where a
supported browser falls in its range.

## Report

One table per tier:

| Part (version) | Issue | Affects the app? | Evidence | Whose |
|---|---|---|---|---|

- **Affects the app?:** yes, no, or not checked, with the reason.
- **Evidence:** a markdown link to the app's file and line, or the config key.
- **Whose:** htmx core, the extension, the browser, or the app (its own glue or use).

With `upgrade`, then a table of what the latest release changes for the app:

| Change | App uses it at | Shipped | Latest |
|---|---|---|---|

Then:

- **Fixed upstream, not yet in the app:** the release that has it.
- **Regressions in the latest release:** whether they reach the app.
- **Workarounds already in the app:** the code that covers an issue, and whether an upgrade still needs it.
- **Not checked:** what was skipped and why.

Rules:

- Report only. Never edit, upgrade, comment or file an issue.
- Quote an issue only after reading it.
- Say plainly what is unverified; nothing here is tested in a browser.
- A browser's bug belongs to the browser, not to htmx, even when htmx works around it.
