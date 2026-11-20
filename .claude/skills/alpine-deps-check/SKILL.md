---
name: alpine-deps-check
description: Inspect an app's vendored Alpine.js for upstream bugs and issues that reach it. Alpine core with its CSP build is the major; the Alpine plugins the app loads (morph, collapse, focus, ...) are the mid; browser-specific issues against the app's oldest supported browsers take the minor's place. Reads the version from the vendored file, searches alpinejs/alpine's issues, discussions, open pull requests, releases and the diff to the latest release, matches them against the app's code, and reports. Read-only. Use when asked to check Alpine or its plugins for bugs, issues, regressions or an upgrade.
---

# Alpine dependency check

Run inside an app that ships Alpine (where it vendors it: the project's
list, `.claude/deps.md`, or `public/vendor/`). Read-only: it reports and
stops. Nothing is edited, upgraded or filed.

## Arguments

- none: major, mid and browsers.
- `upgrade`: also the shipped vs latest release table (step 5).

## Tiers

| Tier | What |
|---|---|
| Major | Alpine core, and the CSP build where the app ships it (its expression parser is its own) |
| Mid | each Alpine plugin the app loads |
| Browsers | issues naming a browser or version, against the app's oldest supported browsers |

Alpine has no dependencies of its own, so there is no minor tier.

**The htmx bridge is not checked here.** `hx-alpine-compat` is an htmx
extension, checked by `htmx-deps-check`. The Alpine functions it calls
(`cloneNode`, `closestDataStack`, `deferMutations`, `destroyTree`) and what
they do during a morph (`x-cloak`, `_x_dataStack`) are Alpine's, checked here.

## Steps

### 1. Versions

The vendored files and their versions:

```sh
find . -path ./target -prune -o -path ./node_modules -prune -o -path ./dist -prune -o -iname "alpine*.js" -print
```

The version inside the file, where the name does not carry it:

```sh
grep -oE 'version:"[^"]+"' public/vendor/alpinejs-*.js
```

Where the app loads it, and how (its page shell's script tag, `defer`):

```sh
grep -rn 'ALPINEJS\|alpinejs' --include=*.rs --include=*.html crates src | grep -v target
```

A name with `csp` is the CSP build. Two Alpine builds, or two versions of
one plugin, shipped at once is a finding of its own.

### 2. Relevance

The directives the app writes:

```sh
grep -rhoE --include=*.rs --include=*.html '(^|[^h-])x-(data|show|cloak|model|modelable|init|effect|transition|ref|for|if|text|html|ignore|id|teleport|bind:[a-z-]+|on:[a-z-]+(\.[a-z.]+)?)\b' crates src | sed 's/^[^x]*//' | sort | uniq -c | sort -rn
```

The magics:

```sh
grep -rhoE --include=*.rs --include=*.html '\$(watch|nextTick|dispatch|el|refs|event|root|data|id|store)\b' crates src | sort | uniq -c
```

What the app registers from a script (none, where the project's rules keep
Alpine to attributes):

```sh
grep -rnE 'Alpine\.(data|store|plugin|directive|magic|bind)|alpine:init' --include=*.rs --include=*.js --include=*.html crates src public/js | grep -v vendor
```

Drop what the app does not use, one line each: a plugin vendored but not
loaded, a directive or magic with no use in the app.

Note any workaround the app already carries for Alpine, and the limits it
writes down for the CSP build (its CLAUDE.md, "Islands, htmx and Alpine").
Read their comments: they name the issues they cover.

### 3. Browsers

The app's oldest supported browsers, from its docs or browser tests (its
low bar: the testing doc, the browser tests' runner). Keep the list for
step 4 and the report.

### 4. Upstream

Alpine routes many bug reports to Discussions, and many fixes arrive as a
pull request with no issue behind them. Search all three.

Releases:

```sh
gh release list --repo alpinejs/alpine --limit 8
```

The shipped release's date:

```sh
gh release view v3.17.4 --repo alpinejs/alpine --json publishedAt --jq .publishedAt
```

Open issues:

```sh
gh issue list --repo alpinejs/alpine --state open --limit 200 --json number,title,createdAt
```

Issues closed since the shipped release:

```sh
gh issue list --repo alpinejs/alpine --state closed --search "closed:>2026-09-21" --limit 60 --json number,title,closedAt
```

Open pull requests (a fix not merged yet):

```sh
gh pr list --repo alpinejs/alpine --state open --limit 100 --json number,title,createdAt
```

Pull requests merged since the shipped release (fixed upstream, maybe not in the app's version):

```sh
gh pr list --repo alpinejs/alpine --state merged --search "merged:>2026-09-21" --limit 60 --json number,title,mergedAt
```

Discussions, by a word the app needs (`csp`, `morph`, a directive, a browser):

```sh
gh api graphql -f query='{search(query:"repo:alpinejs/alpine csp",type:DISCUSSION,first:30){nodes{... on Discussion{number title createdAt closed}}}}' --jq '.data.search.nodes[] | "\(.number) \(.closed) \(.createdAt) \(.title)"'
```

One discussion with its answers:

```sh
gh api graphql -f query='{repository(owner:"alpinejs",name:"alpine"){discussion(number:4788){title body comments(first:20){nodes{body}}}}}' --jq '.data.repository.discussion'
```

Narrow issues and pull requests the same way:

```sh
gh search issues --repo alpinejs/alpine "csp" --limit 30 --json number,title,state,isPullRequest,updatedAt
```

Every issue the app's code or comments name:

```sh
grep -rnoE '(alpinejs/alpine#|[Aa]lpine ?#)[0-9]+' --include=*.rs --include=*.js --include=*.md . | grep -v target | sort -u
```

Open any issue, discussion or pull request whose title might apply and read
its body before judging it.

Security advisories:

```sh
gh api repos/alpinejs/alpine/security-advisories --jq '.[] | "\(.ghsa_id) \(.severity) \(.summary)"'
```

### 5. Shipped vs latest (`upgrade`)

The latest release's notes:

```sh
gh release view v3.17.5 --repo alpinejs/alpine --json body --jq .body
```

What changed in the source between the two:

```sh
gh api repos/alpinejs/alpine/compare/v3.17.4...v3.17.5 --jq '.files[] | "\(.filename) +\(.additions) -\(.deletions)"'
```

One package's diff (the CSP build's parser):

```sh
gh api repos/alpinejs/alpine/compare/v3.17.4...v3.17.5 --jq '.files[] | select(.filename=="packages/csp/src/parser.js") | .patch'
```

A function in the latest release's source, where a claim needs it:

```sh
gh api "repos/alpinejs/alpine/contents/packages/csp/src/parser.js?ref=v3.17.5" --jq .content | base64 -d | grep -n "globalThis" -A4
```

The same in the shipped file, to compare:

```sh
grep -oE '.{40}getOwnPropertyNames\(globalThis\).{120}' public/vendor/alpinejs-csp-*.min.js
```

Issues and discussions opened against the latest release are its
regressions: search its version (`3.17.5`).

### 6. Match to the app

For each candidate, find the directive, modifier, magic or build it touches
and grep the app for it:

```sh
grep -rn -- 'x-on:keydown.escape' crates src | grep -v target
```

Typical names: `x-data`, `x-init`, `x-effect`, `x-show`, `x-cloak`,
`x-transition`, `x-bind:class`, `.window`, `.prevent`, `.capture`, `$refs`,
`$root`, `$el.dataset`, a CSP parser feature (`?.`, `??`, `;`, a global).
An issue counts only with evidence: the file and line in the app that
reaches it, or the build that carries it (a CSP-only bug needs the CSP
build). A browser issue counts only where a supported browser falls in its
range.

## Report

One table per tier:

| Part (version) | Issue | Affects the app? | Evidence | Whose |
|---|---|---|---|---|

- **Affects the app?:** yes, no, or not checked, with the reason.
- **Evidence:** a markdown link to the app's file and line, or the build.
- **Whose:** Alpine core, the CSP build, the plugin, the browser, or the app (its own use).

With `upgrade`, then a table of what the latest release changes for the app:

| Change | App uses it at | Shipped | Latest |
|---|---|---|---|

Then:

- **Fixed upstream, not yet in the app:** the release that has it, or the open pull request.
- **Regressions in the latest release:** whether they reach the app.
- **Workarounds already in the app:** the code that covers an issue, and whether an upgrade still needs it.
- **Not checked:** what was skipped and why; always `hx-alpine-compat`, pointing to `htmx-deps-check`.

Rules:

- Report only. Never edit, upgrade, comment or file an issue.
- Quote an issue, discussion or pull request only after reading it.
- Say plainly what is unverified; nothing here is tested in a browser.
- A browser's bug belongs to the browser, not to Alpine, even when Alpine works around it.
