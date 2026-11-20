---
name: test-coverage
description: Use when the maintainer asks what the tests cover or miss (/test-coverage): every part of the project read from the tree itself, by test level, unit and integration (the server, the apps' Rust, the config, the test crates and migrations), e2e on the web (the server under the lanes, the routes no lane opens, the islands, the views' htmx and Alpine, the stylesheet's own rules, the hand-written JS and the scripts), e2e in the apps (Swift and Kotlin, the behaviours on every app lane, phones, tablets, the years, the desktop), every lane's engine against the project's floors at every run, the tests' quality (a clean build at each floor, the flaky tests across runs, whether the tests assert, mutation on the maintainer's word), and last the manual level, what only hand checks reach. Any level, row or group runs alone. Reports the gaps, each a test to write or a reason it needs none. Reads and measures only, nothing in the repo edited.
---

# Test coverage

What no test reaches, at each level. Run inside a project's repo; every project fact (its lanes, its floors, its folders, its logs) comes from its list and its tree, never from this file.

The rows are grouped by test level, each group a run of its own: unit and integration (1–4), e2e on the web (5–10), e2e in the apps (11–12), the tests' quality (13–15), manual (16).

## The project's list first

Read `.claude/coverage.md` in the project's root: its workspaces, its lane script and bars, its floors, where each lane logs, its results log, its islands, views, stylesheet, hand-written JS, scripts and native files, its hand checks, and the parts it has none of (no apps, no lanes), each said so. Below, "(the list)" means a name or a path found there. No list: say so, run anyway, and propose one at the end (a new project copies the starter's).

## Rules

- **Only on the maintainer's word**, never as a check of a change (the maintainer's rules: coverage only on the maintainer's word).
- **On the test machine, by hash** (the maintainer's rules, `CLAUDE.local.md`): a checkout of its own at the commit's hash, its output to a file, as the suite's runs; never a copy of the working tree.
- **The lists come from the project, never from this file** (Rule no. 1): the lanes and bars from the lane script, the files from the tree, the hand checks from the testing doc. A name below is an example of the day it was written, not the list.
- **Rows 5 and 6 need a run of the lanes,** so they come only in a testing period the maintainer calls. Outside one, the report gives the other rows and says those wait. Rows 1, 2, 4, 5, 6, 10 (`shellcheck`, `tsc`), 13 and 15 run something; every other row is read from the code, the tests and the logs.
- **Any level, row or group runs alone:** `/test-coverage unit`, `/test-coverage e2e web`, `/test-coverage e2e apps`, `/test-coverage quality`, `/test-coverage manual`, `/test-coverage rows 7, 8`. With ultracode on, the whole skill goes as a workflow, one agent a level and a big level split by part (the islands in groups, the native code by platform), all at once; with it off, level by level, one a run. Never every row in one reader: one reader can hold a level whole, not the project (2026-11-18: five rows at once left half the island verdicts judged by a test's name).
- **A test's body read, never its name:** a behaviour is tested only where the test's own assert, or its helper's, checks its effect, cited by file and line. A name or a doc comment alone is "unread", said so.
- **Rows a project lacks** (no apps, no lanes, no shaders) are said so in the report, one line each.
- **Nothing in the repo is edited.** The coverage builds write only their own target folders.
- **Not measured, on purpose:** line coverage of the islands' WASM (nightly only, experimental, a Clang matching nightly's LLVM, the split chunks unknown: a probe on the maintainer's word, later); the frameworks' own code (Tauri, wry, Leptos, Loco: their bugs are the `*-deps-check` skills'); the CSS framework's generated utilities (row 9 takes the stylesheet's own rules); vendored files, images, fonts, test snapshots, and CI files the project keeps off; the proxy's config and the deploy docs' commands (the deploys are the maintainer's, tested by the maintainer's own deploy and its check); generated files (`.sqlx/`, the Gradle wrapper), lint configs, and the starter's unused examples and fixtures.

## 0. The map, first

Before any row, the project's parts are listed from the tree and each is given its row; **a part no row takes is the report's first finding** (the skill fell behind the project), with the row proposed for it. This is how the skill keeps up: it is checked against the project at every run.

- **The lanes and bars:** read from the lane script (the list), its lanes and its bars, each lane with its platform, its size (phone, tablet, desktop), its engine and its year.
- **The floors:** the oldest systems and engines the project supports, from its own docs and config (the list). **Every lane's engine is held against its floor here, at every run, the web's lanes and the apps' alike,** read from the lane's last run (the browser and system its first session's driver answered with; where a lane has none yet, the driver it fetched or the version it printed): one under its floor is a finding, the lane testing what the project does not support; a floor no lane runs at is one too, the floor itself untested; and a lane with no version in its last output is said so (2026-11-18, an islands app on this stack: an Android image carried a webview under the floor, found only when its lane could not start).
- **The workspaces:** every `Cargo.toml` with a `[workspace]` (the list).
- **The native files:** every `*.swift`, `*.kt`, `*.java`, `*.m`, `*.mm` outside `target/`, `build/`, `.tauri/` (Tauri's copy) and `generated/`; one run as a build script is the scripts', one of a native unit test the next bullet's.
- **The native unit tests:** every unit-test target in the iPhone project's spec and every JVM test folder, row 11 crediting a behaviour their asserts check.
- **The hand-written JS:** every `*.js` outside `vendor/`, `node_modules/` and the build's output, and every `<script>` inside a hand-written `*.html` (an app's start page).
- **The views with htmx or Alpine:** every view writing their attributes (row 8).
- **The stylesheet's hand-written rules:** its custom variants, layers, keyframes, fallbacks and theme roles (row 9).
- **The shaders:** every `*.wgsl`, `*.frag`, `*.vert`, `*.glsl`.
- **The server's config:** every environment's file (`config/*.yaml`).
- **The release-only build steps:** what changes the shipped build and not the lanes' (Android's shrinker rules, the release profiles).
- **The scripts:** every `*.sh` outside `target/`, and a script in another language run as one.
- **The app's config:** `tauri*.conf.json`, `capabilities/`, `permissions/`, the entitlements and `Info.plist`s, the Android manifest, and each outside service's config file.
- **The apps' declared parts:** what the platforms read from data, not code: Android's resource XML (`res/xml/`, `res/layout/`, `res/values*/`: a widget's layout and info, the quick actions, the network, backup and file-path rules), the iPhone project's own files (`project.yml`, the launch storyboard, the asset catalogue's colours and image sets, `Podfile`, a bridging header), and Apple's privacy manifests (`*.xcprivacy`).
- **The tests the runner skips:** every `#[ignore]` and every doctest (nextest runs none), counted; and each lane's own skips (row 4).
- **The run logs:** the project's results log and each lane's last output (row 14).
- **The copies:** a file kept as a copy of another for reading (a schema), and the secrets' examples (`*.env.example`).
- **The mails:** each mail's view.
- **The app's targets:** each platform the app's config builds (iOS, iPadOS, Android, macOS, Windows, Linux), and each extension (a widget, a notification service).
- **The hand checks:** the testing doc's, and the project's memory of the phone checks left.

## Unit and integration tests

### 1. The server under the unit and request tests

```bash
cargo llvm-cov nextest --workspace --summary-only
```

In place of the plain suite run, never after it. The report keeps the totals and the files under 50% lines, lowest first.

### 2. The app's Rust

Each app workspace, measured, then named crate by crate (the app's own and each plugin's `src/lib.rs`), a crate with no test of its own said so:

```bash
cargo llvm-cov nextest --manifest-path src-tauri/Cargo.toml --workspace --summary-only
```

### 3. The config and the release build

- **The app's:** what the site's page may call in the app (`capabilities/`, `permissions/`), the entitlements, the schemes and the associated domains, against a test that pins each (the app's own Rust tests, the server's request tests for `assetlinks.json` and the Apple site file). A capability or permission no test pins is a gap: one added by accident widens what a page may call; a pin that lets a whole prefix through or leaves the folder's other files unread is one too. Each outside service's config file against the app's own id and its twin on the other platform: none held by a test is a gap.
- **The server's:** each environment's file against the test that pins its policy (the list); a key that changes policy with no pin is a gap.
- **The privacy manifests:** each `*.xcprivacy` against what the app and its SDKs collect and the required-reason APIs they call; one no check reads is a gap, since Apple refuses an upload over it (a check proposed: a script reading the archive's manifests, or Xcode's privacy report, before each upload).
- **The secrets' examples:** every key of each `*.env.example` against the keys the code and the config test read; a key in one and not the other is a gap.
- **The release-only steps:** what only the shipped build does (Android's shrinker and its keep rules, a release profile's settings), against any lane or check that runs a release build. None is a gap: a class the shrinker strips (a push service, a widget's receiver) breaks the shipped app alone, so the proposal is a release build installed on one app lane before a store upload.

### 4. The test crates and the migrations

- Each test workspace's own helpers (the list), measured as row 1 with its manifest.
- **The tests the runner skips:** each `#[ignore]` with its reason and when it last ran, each doctest with whether anything runs it (`cargo test --doc`; nextest does not). Zero of each is said so.
- **The tests the harness skips:** each lane's own skips from its last run (the list), counted a lane; a test every lane of a size, a year or a platform skips is untested there, a gap of row 12's kind, its reason read for whether it still holds.
- **The copies:** each against what it copies (`sqlite3 -readonly … ".schema"` against a kept schema); one no test compares is a gap, with the guard proposed.
- The migrations: applied to an empty database by every test; whether one is tested against a copy of real data (a deployed server's shape, a large table) is said, and a migration that rewrites rows with no such test is a gap.

## E2e tests, the web

### 5. The server under the e2e tests

The e2e server built instrumented, the lanes run as usual, the counters read after. First a probe: whether cargo-leptos's build of the e2e site takes the flags `show-env` sets (unverified); if it does not, the row says so and stops.

```bash
cargo llvm-cov clean --workspace && source <(cargo llvm-cov show-env --export-prefix) && e2e/run.sh main && cargo llvm-cov report --summary-only
```

`main` by default; on the maintainer's word every bar the map found (tablets, low, beta, the apps'), each counted apart. Compared with row 1: code only the e2e tests reach, code only one bar reaches, and code neither reaches.

### 6. Routes no lane opens

Every route (`cargo loco routes`) against the paths the e2e servers answered in that run (each lane's server log, the list), a number in a path read as its pattern (`/products/12-x` as `/products/{id}-{slug}`). A route with no hit is a page or a post no e2e test makes; one hit by a single size or platform alone is named with it.

### 7. Island behaviours no test checks

Every island file (the list), read whole: each behaviour a test would check (a job a visitor sees, a failure or refusal path, a fallback, a cleanup) against the tests that trigger it and check its effect: the e2e tests, their helpers, the islands' own `#[test]` and `#[wasm_bindgen_test]`. Waking is no check: every island wakes on every page.

- Each behaviour marked tested, partly, untested, by hand only (a real vibration, a GPU's look), or none needed (plumbing other tests drive).
- Every gap read a second time, trying to find the test that covers it, before it is reported.
- Per gap, where its test goes: a lane (web or app) or a WASM unit test.
- The shaders of the map: whether a test compiles each one (naga for WGSL, or the browser's own compile on a lane with WebGPU and WebGL2), and whether a lane draws through each path; the look is hand only.
- The maps so far: the list's, each with its date and its counts (behaviours, tested, partly, untested, by hand only).

### 8. The views' htmx and Alpine behaviours

Every view that writes htmx's or Alpine's attributes (`hx-*`, `x-data`, `x-show`, `x-init`, `@click`, `:class`…), read whole: each behaviour a visitor or a staff member sees (a menu, a dropdown, a fold, a panel, a toggle, a list put in place, a form answered in place with its 422 or 409, a poll, a preload, Back and Forward in place) against the lanes' tests that trigger it and check its effect, marked as in row 7. A request test that finds the attribute drawn is partly: it shows the HTML, not the browser doing it. What the plain link or form does with htmx or Alpine blocked is a behaviour too. Per gap, the lane and the size its test goes on.

### 9. The stylesheet's own rules

What the stylesheet holds by hand, beside the framework's generated utilities, which come only from classes the views use and so need no row (the list): each custom variant, each rule in its layers, each keyframe and the state that plays it, each `@supports` and `@media` fallback, and each role colour with its light and dark value (`@theme`). Each against an e2e test that puts the page in that state and reads what it draws (the computed style, an element's visibility, place or animation), marked as in row 7; a request test that finds the class drawn is partly. Per gap, the lane, the size, the scheme (light, dark), the motion setting (Reduce Motion) or the engine (a `@supports` fallback on the low bar) its test needs. The look itself stays a hand check (row 16); the stylesheet's own build lists (`@source`, `@source not inline`) are the build's, not a behaviour.

### 10. The hand-written JS and the scripts

- **The JS of the map:** each behaviour against the lanes' tests that reach it, marked as in row 7.
- **The JS's types:** where the project has its own `jsconfig.json`, its check (`npm run lint`, or `tsc --noEmit -p jsconfig.json`); otherwise `tsc --noEmit --allowJs --checkJs` over the JS of the map, a read with no edit (`DOM` for the page's, `WebWorker` for a service worker). Its errors counted, the ones that could break a page or a push named.
- **The scripts of the map:** each against what runs it and checks its result; `shellcheck` over them, a read with no edit, its warnings counted, the ones that could break a lane named.

## E2e tests, the apps

### 11. The app's native code

Every native file of the map, read whole, no line coverage, and the apps' declared parts with it (a widget's layout, a quick action, a launch screen are behaviours too, declared in data). Each behaviour against the app lanes' tests that trigger it and the native unit tests that assert it, marked as in row 7, and per gap where its test goes: an app lane, a native unit test (XCTest, JUnit, worth it for logic with no screen: a payload parsed, a count kept, a word chosen), or hand only. None written yet is itself a line in the report.

### 12. The app's behaviours on every app lane

Line coverage is not possible there; scenarios are. The app's behaviours, from the project's architecture doc and the app's `src/lib.rs` (the links handed to the system, a scheme opening nowhere, push and its tap, passkeys, the status bar, the start page offline, the cold start from a link, the quick actions, the share sheet), each against the tests of every app lane the map found that exercises it: the phones, the tablets, each year, and the desktop app.

- A behaviour a size, a year or an engine changes (an iPad's tab bar, a system's own API, the WebView's version) is checked on that lane; each year's lane named with the engine it tested (WKWebView's iOS, Android System WebView's version). One a year's system lacks has its test check what that year draws instead, never a skip alone.
- Each lane's engine against its floor is row 0's, checked at every run for every lane.
- A target the app builds with no lane (a desktop the screenshots script alone photographs, a system with no machine here) is a gap of its own, with what a lane there would take.

## The tests' quality

### 13. A clean build at each floor

Each target the app builds, from an empty target folder (a `CARGO_TARGET_DIR` of its own, the native build's derived data and the Swift packages' output with it), at the floors of the map (the list's build script for the apps). A compile or a link that fails there is a finding a cache hides (2026-11-18, an islands app on this stack: the iPhone app linked from a Swift build cached at an older target and failed fresh at the floor, a C++ library no longer brought along). Minutes of a Mac's time: on the maintainer's word, or after a floor or a toolchain changes.

### 14. The flaky tests across runs

From the project's results log and each lane's last output (the list; nextest's `FLAKY`, `TRY 2 FAIL`): every test flaky, or red and then green alone, in the last two weeks, with its lanes and its count. One flaky twice or more is a finding, a test that hides its next real failure: its cause probed (the project's e2e skill, "A failure") or its wait made to hold, never another retry. No run, the logs alone. A log line that counts flaky tests without naming them cannot be counted across runs: said so, and the names asked of the next lines.

### 15. Whether the tests assert

Line coverage counts code reached, not code checked. `cargo mutants`, one crate or file at a time (the sign-in, the permission and cross-site checks, anything that takes money first), its surviving mutants listed: one that survives is code the tests run and never check. Hours a crate: only on the maintainer's word, the tool's version taken as `/tools` and the two-week rule say.

## Manual tests

### 16. What only a hand check reaches

Last, once every row before it has said what the automatic tests reach: the behaviours no lane can drive (a widget on the home screen, a notification service extension, a real push, a crash report reaching its service, a real vibration, the glass, a mail as Gmail, Outlook and Apple Mail draw it, light and dark), each against the testing doc's hand checks: one with no hand check is a gap, one with a hand check never done since its change is named.

## Report

Row 0 first: every part with its row, and the parts no row took. Then a table a level, one line a row: what was measured, the number or the list, the gaps.

## Outstanding and verdict

| Gap | Where | Kind | Proposed |
|---|---|---|---|
| a part no row takes | the part | skill | the row to add to this skill |
| a file or function no test reaches | its file and line | server, app, test crate | the test to write, or the reason it needs none |
| a capability, permission or policy key no test pins | the file | config | the test that should |
| a release-only step no release build checks | the file | release | the release build on an app lane |
| a declared part (resource XML, launch screen, privacy manifest) no lane or check reads | the file | declared | the lane's test or the check that should |
| a secret's key in the example and not the code, or the reverse | the key | config | the example or the test fixed |
| a test the runner skips | the test | skipped | its run, or its removal |
| a test a size, a year or a platform always skips | the test and its lanes | skipped | the test made to run there, or its reason written |
| a copy no test compares | the file | copy | the guard that should |
| a migration rewriting rows untested on real data | the migration | data | the test that should |
| a route no lane opens | the route | page, post | the lane and test that should |
| an island, view or JS behaviour no test checks | its file | island, view, JS | the lane's test, or a WASM unit test, that should |
| a stylesheet rule no test puts the page in | the rule | CSS | the lane's test reading what it draws |
| a shader no test compiles | the file | shader | the compile check that should |
| a script's warning that could break a lane | the script and line | script | the fix |
| a native behaviour with no test | the Swift or Kotlin file | native | the app lane's test, a native unit test, or hand only |
| an app behaviour with no test | the behaviour, the lane, size, year or target | app | the lane's test that should |
| a target with no lane | the platform | app | what its lane would take |
| a lane's engine under a floor, or a floor no lane runs at | the lane and its version | engine | the lane's image or the floor changed |
| a target that fails to build from clean at its floor | the target and its error | build | the build fixed |
| a test flaky twice or more | the test, its lanes, its count | flaky | its cause probed, its wait fixed |
| a surviving mutant | its file, line and change | assert | the assert that would catch it |
| a hand-only behaviour with no hand check | the behaviour | hand | the check to add to the testing doc |
| a row not run | the row | waiting | the testing period, or the probe |

Then the verdict, one line, with the count of each kind:

- 🟢 **covered:** no gap worth a test.
- 🟡 **gaps worth tests:** listed above, none on a path a visitor or the business depends on.
- 🔴 **a shipped path untested:** the sign-in, a permission or cross-site check, a mail a person needs, a push, or anything that takes money or an order.

## The list's shape

`.claude/coverage.md`, in the project's git, short: `## Workspaces`, `## Lanes and bars` (the script, its bars, where each lane logs, the results log), `## Floors`, `## Parts` (the islands, the views, the stylesheet, the JS, the scripts, the native files, the mails), `## None here` (the rows the project lacks, each with one line), `## Hand checks`, `## Maps so far` (each run's date and counts). The test machine's name stays in `CLAUDE.local.md`.
