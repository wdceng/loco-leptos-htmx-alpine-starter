# Loco Leptos htmx Alpine Starter

**Loco, Rails for Rust, on the back end. Leptos, htmx and Alpine on the front.**

[![License: MIT or Apache-2.0](https://img.shields.io/badge/license-MIT%20or%20Apache--2.0-blue)](#license)
[![Rust 1.94+](https://img.shields.io/badge/rust-1.94%2B-orange)](docs/prerequisites.md)
[![Loco 1.2](https://img.shields.io/badge/loco-1.2-green)](https://loco.rs)
[![Leptos 0.8 islands](https://img.shields.io/badge/leptos-0.8%20islands-red)](https://leptos.dev)
[![htmx 4](https://img.shields.io/badge/htmx-4-blue)](https://htmx.org)
[![Alpine 3 CSP](https://img.shields.io/badge/alpine-3%20CSP-teal)](https://alpinejs.dev)
[![Tailwind v4](https://img.shields.io/badge/tailwind-v4-38bdf8)](https://tailwindcss.com)
[![Tauri 2](https://img.shields.io/badge/tauri-2-ffc131)](https://tauri.app)
[![Platforms: web, iOS, Android, Windows, macOS, Linux](https://img.shields.io/badge/platforms-web%20%7C%20iOS%20%7C%20Android%20%7C%20Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)](#platforms)

## Introduction

A multi-page web app that feels like a single-page app, also as an app for
iPhone, iPad, Android, Windows, macOS and Linux. Made to be built with AI.

Leptos in place of Loco's templates: pages rendered on the server, islands
in WASM, htmx and Alpine on top as attributes in the Leptos views: no
template files, no script files to write, the two that ship are plumbing.
htmx swaps each page in place, every page is whole and works without it,
and a page loads only the islands it uses.

AI agents: read `CLAUDE.md` first; its rules apply to every change.

## Motivation

An agent writes fast but guesses; the Rust compiler answers every guess
before anything runs: the queries against the schema, the server, the
pages and the islands, one compiler from the database to the browser and
the app. What the compiler can't check, the
tests and the rules in `CLAUDE.md` do. As little JavaScript as possible,
and none in the business logic, so nothing slips past that loop. No Node.
Built first in a larger app on the same stack, with Claude Code, steered
and committed by a person.

## Platforms

| Platform | What you get |
|---|---|
| Web | The site; tested in Chrome, Edge, Firefox and Safari, and in Chrome 114 and Firefox 128 |
| iPhone, iPad | The Simulator; a real device; the App Store build |
| Android | The emulator or a phone on USB; the Play Store bundle |
| Windows | An `.exe`, built from the Mac |
| macOS | A `.app`, nothing to install |
| Linux | An AppImage, built from the Mac in Docker; Ubuntu 22.04, Debian 12 and newer |

The apps are a Tauri 2 window on the live site.

- **Already handled:** the page runs to the glass, clear of the camera
  and the status bar; a phone number, mail or text link opens the
  phone's own app; the site gets no access to the device.
- **Not yet:** nothing native, the site in a window only. Apple may turn
  down a store app that only shows a website (its review guideline 4.2).
  Android's navigation bar at the page's foot is untested.

```sh
cd src-tauri && cargo tauri build --bundles app   # the Mac's .app
src-tauri/linux.sh                                # the Linux AppImage, in Docker
cd src-tauri && cargo tauri ios dev               # an iPhone or iPad Simulator
cd src-tauri && cargo tauri android dev           # the Android emulator, or a phone on USB
```

## Built from

| Part | From |
|---|---|
| Server, database, accounts, e-mail, jobs | Loco's Rest API starter |
| Pages, rendered on the server | Leptos in islands mode |
| Moves between pages, without a reload | htmx 4 |
| Small page state, like a menu | Alpine, its CSP build |
| Styling | Tailwind v4, its default look |
| The app's own queries | SQLx `query!`, checked at compile time |
| The app on desktops and phones | Tauri 2 |

## What the template adds

- **Security:** headers, a CSP with a nonce per page, rate limits,
  `security.txt` ([docs/security.md](docs/security.md)).
- **Pages:** favicons, web manifest, 404, 429 and error pages,
  `robots.txt`, `llms.txt`.
- **Phones:** edge to edge, 44px tap targets, Reduce Motion.
- **Mail:** the HTML part in Leptos, compile-checked.
- **The app:** Tauri 2 on six platforms (`src-tauri/`).
- **Environments:** development, test, dev-server, staging, production.
- **Checks:** SQL, pages, mail and file links at compile time; config
  tests; browser tests.
- **One rule for the front:** htmx for what needs the server, Alpine for
  small page state, an island only for what neither can;
  `tests/one_job.rs` guards it.

## Trade-offs

- **A patched cargo-leptos and a pinned Leptos fork** until their fixes
  ship ("Fixed upstream" below).
- **`--split` on every build:** the server refuses to start a build
  without it.
- **JWT only:** no logout or revocation until an app adds cookie sessions
  ([docs/security.md](docs/security.md)).
- **The app is the site in a window:** Apple may refuse it (guideline
  4.2). No off-the-shelf widgets: a rich one is an island you write.

## Where things are

| To change | Look in |
|---|---|
| A page | `src/views/` |
| Something interactive (an island) | `src/islands.rs` |
| A URL | `src/controllers/` |
| The database | `migration/`, then refresh the query cache (`docs/development.md`) |
| A query | `sqlx::query!` on `sql::pool(&ctx)` (`src/sql.rs`) |
| Loco's users model | `src/models/` |
| The e-mails | HTML: `src/views/mail.rs`. Subject and text: `src/mailers/auth.rs` |
| Styling | Tailwind classes in the views; app-wide in `style/tailwind.css` |
| A static file | `public/` |
| Settings per environment | `config/*.yaml` |
| The app on desktops and phones | `src-tauri/` |

Adding a page or an island: `docs/development.md`, "Add a page or an
island".

## Run it in five minutes

You need Rust (https://rustup.rs). Then, in the project folder:

```sh
rustup target add wasm32-unknown-unknown   # compile for the browser
tools/cargo-leptos/install.sh              # the patched build tool (~3 min)
cargo leptos watch --split -- start        # builds the server, the WASM and Tailwind, runs on http://localhost:5150
```

## Tests

```sh
time cargo nextest run                  # all, the usual way: faster than cargo test, the same tests
cargo nextest run reset                 # those whose name holds it
cargo test                              # all, the plain way
cargo llvm-cov nextest --summary-only   # all, with each file's coverage
e2e/run.sh all                          # the browser tests: four desktop browsers, three Simulators, two emulators
e2e/run.sh low                          # the same on the oldest versions supported, the low bar
e2e/run.sh beta                         # the same on the browsers' next releases
src-tauri/screens.sh                    # pictures of the app on each phone, tablet and this Mac, against the dev loop
```

What each covers, and what the browser tests need:
[docs/testing.md](docs/testing.md).

## Before you deploy

- **Keep the [firewall](docs/production.md#firewall-once-per-server) and
  the [origin key](docs/production.md#the-origin-key-once-per-server).**
  The rate limiter trusts Cloudflare's `CF-Connecting-IP`: without them,
  anyone can pick their own bucket.
- **No Cloudflare?** [Switch the limiter to `X-Forwarded-For`](docs/production.md#without-cloudflare).

## Fixed upstream

Bugs found upstream while building this, each reported, and the workaround
used here until the fix is released. `tools/upstream.sh` asks GitHub and
crates.io whether it is; how to undo each workaround is under "Way back" in
`docs/decisions.md`.

| Problem | Reported | Upstream fix | Here | Goes when |
|---|---|---|---|---|
| cargo-leptos: with `--split` and hashed names, the main files kept an old name | [#690](https://github.com/leptos-rs/cargo-leptos/issues/690) | [#691](https://github.com/leptos-rs/cargo-leptos/pull/691), merged | A patched cargo-leptos (`tools/cargo-leptos/`) | A cargo-leptos release with #691 that locks wasm-split 0.2.4 |
| wasm-split: unsorted imports, new file names on every build | [#79](https://github.com/WorldSEnder/wasm-split-prototype/issues/79) | [#80](https://github.com/WorldSEnder/wasm-split-prototype/pull/80), released in 0.2.4 | The same patched build | The same cargo-leptos release |
| Leptos: an island's state stayed in memory after htmx swapped it out | [#4952](https://github.com/leptos-rs/leptos/issues/4952) | [#4953](https://github.com/leptos-rs/leptos/pull/4953), merged, for 0.9 | An island collector (`src/collect.rs`) | Leptos 0.9; `htmx-islands.js` then calls its `$$dispose` |
| Leptos: the islands loader waited for idle time forever, so Chrome 114 to 123 could start no island | [#4959](https://github.com/leptos-rs/leptos/issues/4959) | [#4978](https://github.com/leptos-rs/leptos/pull/4978) by Leptos's author, open, for 0.9 | Leptos from our fork, the wait capped at 2 s, pinned in `Cargo.toml` | The Leptos release with #4978 (0.9, or a backport to 0.8) |
| htmx: Chrome 150 to 156 drops the comments Leptos hydrates by | [#4147](https://github.com/bigskysoftware/htmx/issues/4147) | Chrome 157 | `public/js/htmx-islands.js` removes `parseHTMLUnsafe`, so htmx parses with DOMParser | **Stays** while Chrome 150 to 156 are above the low bar |
| htmx 4: an extension loaded with `defer` can miss the first page, so its links don't preload (seen in Chrome 114) | [#4170](https://github.com/bigskysoftware/htmx/issues/4170) | Closed: `defer` is outside htmx's documented way | htmx and its extensions as plain tags, as its docs show | Stays: it's the documented way |

## Docs

| Read | For |
|---|---|
| [development.md](docs/development.md) | Commands, environments, builds, gotchas |
| [testing.md](docs/testing.md) | What the tests cover, security checks by hand |
| [dev-server.md](docs/dev-server.md), [staging.md](docs/staging.md), [production.md](docs/production.md) | Deploying to a Linux server. Read production.md first: Caddy, Cloudflare, firewall, mail |
| [architecture.md](docs/architecture.md) | Why it's built this way |
| [prerequisites.md](docs/prerequisites.md) | Every tool |
| [CLAUDE.md](CLAUDE.md), [AGENTS.md](AGENTS.md), `.claude/skills/loco/` | For AI coding agents |

## License

MIT or Apache-2.0, your choice: `LICENSE-MIT`, `LICENSE-APACHE`.
