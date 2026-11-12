# Testing

What the tests cover, and how to check things by hand. The why is in
`architecture.md`.

## Automated Tests

All, the usual way: faster than `cargo test`, the same tests:

```bash
time cargo nextest run
```

Those whose name holds a word:

```bash
cargo nextest run reset
```

All, the plain way:

```bash
cargo test
```

The same, showing `println!` output:

```bash
cargo test -- --nocapture
```

All, with each file's coverage:

```bash
cargo llvm-cov nextest --summary-only
```

Request tests use Loco's harness, `request::<App, _, _>(|request, ctx| ...)`.
It boots the app in-process with `config/test.yaml`. No server, port or
browser. The database is `app_test.sqlite`, rebuilt on every boot
(`dangerously_truncate` and `dangerously_recreate` in `config/test.yaml`),
so the suite runs in about a second.

- **Every request test needs `#[serial]`.** Two app boots at once fight over
  the shared context.
- **`#[serial]` does nothing under nextest.** Each test is its own process,
  so `.config/nextest.toml` runs the binary built from `tests/mod.rs` on one
  thread. Unit tests and the config canary stay parallel. Without it you get
  `UNIQUE constraint failed: seaql_migrations.version` or `no such table`.

**Test files:**

- `tests/mod.rs`: module root, wires the folders below.
- `tests/config.rs`: the config canary. Loads each `config/<env>.yaml` with
  Loco's own loader. Edit a config file, run this first. Every environment
  must have:
  - an empty `ident`
  - `secure_headers` with the seven overrides, plus the strict CSP for
    anything that isn't a page
  - a CSP with `'wasm-unsafe-eval'` and no `unsafe-inline`
  - a 15 s `timeout_request` and a 64 KB `limit_payload`
  - `remote_ip` on, compression and fallback off
  - a `settings` block that parses
  - a mail sender (production's from `MAILER_FROM`)

  Differences on purpose:
  - three `cache_control` values, all valid headers (else Loco silently
    falls back to a year)
  - dev-server, staging and production: `CfConnectingIp`, burst 120
  - locally: `ConnectInfo`, the live-reload socket
  - dev-server and staging only: `X-Robots-Tag`
  - dev-server: the same config as staging, apart from the defaults the
    unit replaces (host, database, secrets)
  - test: burst 20
  - test and deployed: the auth API's own bucket, ten at once, then one per
    30 s
  - nightly restart: off locally, on (hour 3) when deployed
  - static-file limit: off locally, on when deployed (burst 300, one token
    per 100 ms)

  And the app: `connect-src` in `src-tauri/tauri.conf.json` is its start
  page's `SITE`, alone.
- `tests/models/users.rs`: the users model against the test database.
  Create with password, find by e-mail and pid, validation, duplicate
  e-mail, and the verification, reset and magic-link token flows (the
  model's, kept with it; no page sends a magic link). `insta` snapshots in
  `tests/models/snapshots/`.
- `tests/models/sql_pool.rs`: SQLx reads, through `sql::pool`, a user
  Sea-ORM just wrote. It uses a real `query_scalar!`, so it also proves the
  `.sqlx/` cache works.
- `tests/requests/account.rs`: the account page, signed in through the
  sign-in page (`session_cookie`): the browsers listed and signed out, a
  new password, the account deleted with the password or a fresh proof,
  and another site's posts refused.
- `tests/requests/register.rs`, `forgot.rs`, `verify.rs`: the auth pages
  a person uses, end to end. Users come from `prepare_data.rs` (the model,
  not a page); mails are read back through `mails.rs`. Also:
  - The sign-up and `/forgot` read their mail back. `From:` is
    `settings.mail.from`, and the link starts with `server.host` and carries
    the user's token.
  - Passwords of 7 and 129 characters are refused; a taken email, in any
    case, signs up like a new one; `/forgot` and a new welcome mail answer
    every address alike (`without_nonce` compares the pages).
  - Markup in the name arrives escaped in the HTML part; the subject is
    the app's.
  - A name over 100 characters signs up nobody and sends nothing.
  - A body over 64 KB is a 413.
  - `/verify` without a token sends the mail again, with a fresh link.
- `tests/requests/not_found.rs` keeps Loco's `/api/auth` a miss.
- `tests/requests/error_page.rs`: a browser's `POST /` is a 405 with the
  Leptos error page, `Allow` kept, the page CSP with its nonce and
  `no-store`. Without an HTML `Accept` it stays Loco's empty 405, and the
  JSON API answers a browser in JSON. A handler that fails gives a browser
  the page with status 500 and anything else Loco's JSON; no route of the
  app fails on purpose, so that one mounts the layer on a small router.
- `tests/requests/links.rs`: every address written as a string leads
  somewhere, since the compiler checks none of them.
  - Every `href` and `src` on the home page, the 404 page, the error page
    and the 429 page. An asset path must be a file in `public/`, checked on
    disk, because the test environment serves no files, and must carry its
    current `?v=`: a plain or stale string fails with the constant to use.
    Anything else must be a route that answers 200, and a `#id` must exist
    on the page. `/pkg/…` is cargo-leptos's output, which `home.rs` checks.
  - The web manifest (a route) is JSON with `APP_NAME`, `SURFACE_HEX`,
    `standalone`, and icons that are current versioned files in `public/`.
  - The three auth mails are sent and each link is requested: all three
    answer 200.
- `tests/requests/reset.rs`: the reset page as a person uses it. The mail's
  link shows the form (`no-store`, `no-referrer`, `noindex`); a short
  password shows it again with the reason; a good one changes the password,
  the old one stops working, every session of the user closes, and the
  link is spent. Another site's post changes nothing (403).
- `tests/requests/verify.rs`: the verify page as a person uses it. The
  welcome mail's link shows the button (`no-store`, `no-referrer`,
  `noindex`) and verifies nothing; the press verifies, the account signs
  in, and the link is spent. Another site's post verifies nothing (403).
- `tests/requests/login.rs`: the sign-in as a person uses it. The form
  (`no-store`); the right password opens a session, its cookie
  `HttpOnly; SameSite=Lax`, 12 hours, and a 303 home; the page then says
  who, with the sign-out. An unknown email, a wrong password and an
  unverified account get one answer, a 400 with the email typed and no
  cookie. The sign-out closes the row and takes the cookie; the old
  cookie opens nothing. Another site's post signs nobody in or out. The
  eleventh try in a row is a 429, visits to the page spending none.
- `tests/models/sessions.rs`: a session found by its secret alone, the
  row holding its SHA-256 and never the secret, a run-out one not found and
  swept, closing one or all of a user's.
- `tests/requests/home.rs`: a 200 with an HTML content type, a real document
  (doctype, `lang="en"`), the app name (`APP_NAME`), exactly one `<h1>`, the
  skip link and its target, the islands loader script, the wasm file it
  loads (`app.wasm`, guarding the compile-time `LEPTOS_OUTPUT_NAME` gotcha),
  and the footer year computed at render time.
- `tests/requests/not_found.rs`: unknown paths. As HTML, a 404 with the
  not-found page inside the shell and `cache-control: no-store`. Without an
  HTML `Accept`, a 404 with one line of text and no page. Both carry the
  security headers and a request id. Twenty misses in a row are answered,
  the twenty-first is a 429, and routes still answer: misses have their own
  bucket, with the site-wide numbers.
- `tests/requests/robots.rs`: `/robots.txt` in the test environment (200,
  `text/plain`, `Disallow: /`).
- `tests/requests/llms.rs`: `/llms.txt` is Markdown starting with
  `# APP_NAME`, its links start with `server.host`, and each one answers
  200.
- Robots tags and the manifest: the home page has no
  `<meta name="robots">` and its manifest link asks for credentials
  (`home.rs`); the 404 and error pages say `noindex` (`not_found.rs`,
  `error_page.rs`).
- `tests/requests/rate_limit.rs`:
  - With `burst: 20` from `config/test.yaml`, twenty requests pass with no
    `x-ratelimit-limit` or `x-ratelimit-remaining`: a bot can't read its
    tries left. The next is a 429 HTML page with `retry-after: 1` (under a
    second, rounded up, never 0), the same in `x-ratelimit-after`,
    `cache-control: no-store`, the security headers, the strict CSP for
    non-page responses and a request id, so the limiter sits inside
    `secure_headers` and `request_id`. An unmatched path is still a 404:
    the limiter only covers routes.
  - The auth bucket (`auth.burst: 10`): ten logins are 401s, no bucket
    state on them either, the eleventh is
    the 429 page with about 30 s in `retry-after`. `/robots.txt` still
    answers, since the site-wide bucket has tokens left.
- `tests/requests/security_headers.rs`:
  - On the home page, the CSP nonce matches every inline script. The CSP
    has `'wasm-unsafe-eval'` and `frame-ancestors 'none'`, no
    `unsafe-inline`.
  - The preset headers and overrides are there (`x-frame-options: DENY`,
    referrer, permissions, COOP, CORP, COEP, nosniff, HSTS). No
    `x-powered-by`.
  - Any `style=` attribute fails it, since `style-src` is `'self'`.
  - `/robots.txt` and a miss get the strict CSP from the config
    (`default-src 'none'`, no scripts).
- `tests/tasks/user_create.rs`: the `user_create` CLI task.
- `tests/workers/`: an empty Loco starter module.

**Unit tests in `src/`:**

- `src/assets.rs`: parsing the `css:` line of cargo-leptos's hash file. The
  boot-time checks (stale hash file, hashed site without one) are manual,
  see "Hashed asset names". They're skipped in the test environment, so
  `cargo test` passes whatever the last build left in `target/site`.
  Request tests run without a hash file, so they expect the plain
  `/pkg/app.css`.
- `src/controllers/llms.rs`: the body starts with the app name and the
  description, and a host with a trailing `/` gives no `//` in the links.
- `src/controllers/robots.rs`: `/robots.txt` on production (`Allow: /`) and
  on staging. The harness only boots the `test` environment.
- `src/views/mail.rs`: a mail is a whole HTML document with its link and no
  `<!>` hydration markers, and markup in a visitor's name arrives escaped.
- `src/deploy_checks.rs`: real secret values pass, and empty, blank and
  `replace_me` values are named. The boot itself (production refusing,
  the SMTP login warning) is a manual check below.
- `src/views/error.rs`: server errors, the 405, the 408 and other client
  errors each have their own wording.
- `src/views/too_many_requests.rs`: the 429 page links the stylesheet,
  keeps the `{wait}` placeholder for each refusal, and carries no script.
- `src/middleware/error_page.rs`: only a browser asking for a page outside
  `/api/` gets one, only error answers without HTML are replaced, and the
  page keeps the status and the original headers.
- `src/maintenance.rs`: the nightly restart's arithmetic. The next restart
  is today while the hour is ahead, tomorrow from the hour on. In
  Europe/Zagreb, 2026-03-29 02:30 (the spring-forward gap) is `None` but
  03:00 exists, and 2026-10-25 02:30 (repeated in autumn) resolves to the
  later, CET one. Development and test never restart, production and
  staging may. The stop itself is Loco's ordinary SIGTERM shutdown, checked
  by hand below.
- `src/middleware/rate_limit.rs`: the key extractor (IPv6 keyed by its /64
  network, IPv4 written as IPv6 keyed as IPv4), the 429 page (the wait
  rounded up, in words: "1 second", "6 seconds"; the stylesheet resolved at
  boot), and the config-to-key-source mapping.
- `src/paths.rs`: every generated address is a file in `public/` with a
  16-hex version from `build.rs`. Any `public/` file `style/tailwind.css`
  names (none today) is its current versioned URL; if not, the failure
  names the value to write.
- `src/render.rs`: the nonce substitution.
- `src/settings.rs`: the `settings:` block parses. Unknown keys, a CSP
  template without `{nonce}` and a zero rate-limit burst are refused. The
  auth bucket is required, and refuses zeros while the limiter is on. The
  static-file limit is off without its block, parses when on, and refuses
  zeros when enabled.
  `mail.from` is parsed the way the mailer parses it: `Name <address>`, a
  bare address and a quoted name pass, a bare comma in the name
  (`Doe, John <…>`) fails like any malformed sender. `nightly_restart`
  parses a zone name, refuses an unknown one and an hour above 23, and is
  off when the block is missing.

`tests/config.rs` and the auth tests use `rstest`: one body, many `#[case]`
inputs.

### CI

**This repo runs its checks locally; GitHub Actions is off.**
`.github/workflows/ci.yaml` is kept for a project made from the template
that uses Actions: with it on, it runs on every push to `main` and on pull
requests. Same list as the pre-deploy checks in `staging.md` and `production.md`:

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- the wasm32 lint below
- a full `cargo leptos build --split`, with cargo-leptos 0.3.12
- two hashed `--split` builds that must give the same file names, the main
  script naming a split loader that exists (`development.md`, "Lazy
  islands")
- `cargo audit`
- the `.sqlx/` cache check: a fresh test database, migrated, then
  `cargo sqlx prepare --check -- --all-targets`
- the browser tests in Chrome, `e2e/run.sh chrome` (below). A failed run
  keeps `target/e2e/chrome/` as an artifact

Every job builds with `SQLX_OFFLINE=true`, from the committed cache, like
the server build.

`cargo audit` fails on vulnerabilities. Advisories ignored on purpose are
in `.cargo/audit.toml`, each with its reason.

### Browser half lint

The normal build and tests only compile the server. This lints the library
for wasm32 with just the `hydrate` feature:

```bash
cargo clippy --lib --target wasm32-unknown-unknown --no-default-features --features hydrate -- -D warnings
```

It fails if a server-only dependency leaks out of the `ssr` gate, and it's
the only pass that sees the islands.

### An island freed as it leaves

Leptos 0.9 keeps an island's state behind `$$dispose` on its element, and
`public/js/htmx-islands.js` calls it for every island a swap takes out
(`architecture.md`). `an_island_that_leaves_is_freed`
(`e2e/tests/memory.rs`) swaps `IslandsCheck` away and watches its element
lose `$$dispose`, `Navigation` keeping its own;
`no_island_schedules_through_leptos_timer_helpers` (`src/islands.rs`,
native) refuses the timer helpers whose callbacks the tab would keep.

On a real page: keep the home page's island in the console
(`x = document.querySelector('leptos-island[data-component^="IslandsCheck"]')`),
follow any link, and `'$$dispose' in x` is `false`.

### Browser tests

`e2e/` drives browsers over WebDriver (thirtyfour) against a real build:
the moves in place, the memory and the health tests. A crate of its own,
so `cargo test` in the root runs none of it. Twenty lanes, as an islands
app on this stack runs them: nine on the top bar (the four desktop
browsers, Safari on three iOS Simulators, Chrome on two Android
emulators), seven on the low bar ("The Low Bar", below), four on the beta
bar.

The nine top lanes; at once on a 48 GB M1, else main then tablets:

```bash
e2e/run.sh all
```

Chrome, Edge, Firefox and Safari side by side:

```bash
e2e/run.sh desktop
```

The two phones and the four desktop browsers:

```bash
e2e/run.sh main
```

The iPhone and the Android phone:

```bash
e2e/run.sh phones
```

The iPad, the iPad Air upright and the Android tablet:

```bash
e2e/run.sh tablets
```

The low bar's seven, in two batches:

```bash
e2e/run.sh low
```

The browsers' next releases:

```bash
e2e/run.sh beta
```

One lane:

```bash
e2e/run.sh chrome
```

The Android app's own lane, after a change of the app: the app built for
the lane's site, the boot and the app's tests, about 10 minutes with the
builds:

```bash
e2e/run.sh app
```

One lane, its memory tests only (nextest's filter):

```bash
e2e/run.sh firefox memory
```

With the browser's window:

```bash
E2E_HEADED=1 e2e/run.sh chrome moves
```

Some lanes, side by side:

```bash
E2E_LANES="android chrome" e2e/run.sh main
```

The harness's own tests, no browser:

```bash
cargo test --manifest-path e2e/Cargo.toml --lib
```

| Lane | Site | Driver | Browser |
|---|---|---|---|
| `chrome` | 5160 | 9515 | Chrome |
| `edge` | 5161 | 9516 | Edge |
| `firefox` | 5162 | 9517 | Firefox |
| `safari` | 5163 | 9518 | Safari |
| `iphone` | 5164 | 9519 | Safari, iPhone 18 Pro Simulator, iOS 27.2 |
| `ipad` | 5165 | 9520 | Safari, iPad Pro 13-inch (M5) Simulator |
| `android` | 5166 | 9521 | Chrome, `android-phone` emulator on 5560 |
| `android-tablet` | 5167 | 9522 | Chrome, `android-tablet` emulator on 5562 |
| `ipad-air` | 5168 | 9523 | Safari, iPad Air 11-inch (M4) upright: the middle layout |
| `chrome-beta` | 5176 | 9531 | Chrome for Testing, the Beta channel |
| `edge-beta` | 5177 | 9532 | Edge Beta |
| `firefox-beta` | 5178 | 9533 | Firefox Developer Edition |
| `safari-beta` | 5179 | 9534 | Safari Technology Preview |
| `app` | 5180 | 9535 | the Android app, `android-phone-full` emulator on 5570 |

Never two families at once, nor beside another project's lanes: they
share the Simulators and the emulators.

**The app's lane:** the Android app itself, which no browser lane opens.
The debug build for the lane's site (`src-tauri/app-for.sh android
http://localhost:5180`), installed on the Play image's emulator and
started; the chromedriver of the webview's own version
(`com.google.android.webview`) attaches to the running app
(`androidUseRunningApp`, `Browser::App`), and the system's side goes
through adb (`e2e/src/device.rs`). It runs the boot test and
`e2e/tests/app.rs` alone, which every other lane leaves out; in no family:
the start page handing over, its "No internet connection" in airplane
mode and its handover once the network is back, and another site's link
going to the system.
As an islands app on this stack tests its own.

- **The emulator draws in software there** (`-gpu swiftshader_indirect`):
  on the Mac's GPU with no window the app's webview found no OpenGL ES
  context and the app died at its first page ("Chrome runs only on top of
  OpenGL ES"), where the browser lanes' Chrome draws on it.
- **Its AVD is `links.sh`'s and `screens.sh`'s, and an islands app's on
  this stack's own app lane:** never two of them at once.
- **What it doesn't test:** the iPhone's app, and the links to other apps
  (`tel:`, `mailto:`), which `links.sh` taps by hand.

The script builds the site into `target/e2e` with `--split`, once. Each
lane then starts its own site (`LOCO_ENV=e2e`, `config/e2e.yaml`, a fresh
SQLite file) and driver on its own ports, runs the boot test alone, then
the rest one at a time through cargo-nextest, a failure tried once more
and called flaky if it then passes. It never touches the watch loop's
`target/site`, port 5150 or database, so it runs beside `cargo leptos
watch`. Each lane's files are in `target/e2e/<lane>/`: `run.txt`,
`server.log`, `driver.log`, `skipped.txt` and `failures/`.

Each lane has its own Mailpit, which the site sends to (SMTP on 1100 plus
the lane's number, its web side on 8100 plus it, `config/e2e.yaml`'s
`E2E_SMTP_PORT`): the tests read the welcome mail's link and the sign-in
code from its API (`e2e/src/mail.rs`), so no mail leaves a run. A Mac
without `mailpit` refuses the lane (`prerequisites.md`).

Chrome's driver is the one in `~/Library/Caches/chromedriver`, or
`CHROMEDRIVER`, or the one on the `PATH`; the
others are on the `PATH`, the beta bar's in `~/Library/Caches/e2e-beta`,
and an emulator's is the one of its Chrome's version
(`prerequisites.md`).

- **What only DevTools can do runs in Chrome and Edge:** the safe-area
  insets, Reduce Motion asked for, a refused file, memory. The other lanes
  skip those tests, each with its reason in the lane's `skipped.txt`,
  counted apart in the summary.
- **An answer is changed in the page itself** (`Page::answer_with`,
  `window.fetch` wrapped, as the islands app on this stack does it): a
  newer build, a gateway's 502, a 500, a slow answer, in every lane.
- **A page's errors are heard in Chrome, Edge and Firefox**, Firefox's
  through WebDriver BiDi's log, its CSP refusals by a recorder that runs
  before the page's own scripts. Safari's driver has neither DevTools nor
  BiDi, nor has Chrome's on an emulator: those lanes hear none.
- **A device's screen is its own:** a phone runs the phone's tests at its
  screen, a tablet skips them, and a test that sizes the page reads what
  the device has. A press there is a finger's tap.
- **The devices' lanes start their device and shut it down after:** a
  Simulator by name on its runtime, an emulator cold, with no window, its
  port reversed to the Mac's. A Simulator lane whose first session fails
  tries once more with the Simulator and its driver started afresh, what
  it saw in `boot-why.txt`; an emulator that crashes is named in the
  summary with its report.
- **Quit Safari before its lane.** safaridriver waits behind an open
  Safari's windows; the lane refuses to start beside one, and quits its
  own at the end.
- **A lane that stops opening sessions is marked broken** after two tests
  in a row: the rest fail at once, and the summary says since when.

- **Moves** (`e2e/tests/moves.rs`): the lazy island wakes; a link moves in
  place with its title, the focus on `<main>` and the title read out; no
  field takes the focus on the way (htmx's refocus by `id`, blurred
  first); Back restores the scroll; a 404 and a 500 show in the region; a
  refused form swaps in, keeps the address and keeps the focus in its
  field; a second press sends no second post; a sent form looks busy until
  its slow answer; a newer build and a gateway's 502 load a link plainly,
  while a post answered in place swaps in and goes once; Back with no
  answer at all loads plainly; a reload comes back where it was scrolled;
  a move fetches no WASM again and starts the page at its top, never
  passing below, unless its address has a `#`; a desktop's move is asked for at the mouse's press, a hover
  asking nothing.
- **Accounts** (`e2e/tests/accounts.rs`): a visitor signs up, verifies
  through the welcome mail's link and signs in in place, the session's
  cookie from htmx's answer signing the next page in; a wrong password
  refused in the region, the field marked and focused; an admin's mailed
  code typed; the account's deletion asked in a dialog, the focus on
  Cancel; a large photo shrunk to a JPEG under 1 MB before it goes, a
  small JPEG or an unreadable file left as it was, the last of two chosen
  going, the form's buttons out of use meanwhile; a newer build's answer to
  a sign-in loads its page plainly. Each
  test signs up an account of its own (`mailbox`, a retry's its own), an
  admin made one by its role in the lane's database (`make_admin`).
- **Blocked** (`e2e/tests/blocked.rs`, Chrome and Edge): with htmx refused
  a link and a form are page loads; with Alpine or the WASM refused a move
  and a refused form still stay in place.
- **Memory** (`e2e/tests/memory.rs`): an island that leaves is freed; twenty
  moves and twenty Back and Forward steps keep the documents, detached
  nodes, listeners, JS heap and WASM memory flat, measured three times.
- **Health** (`e2e/tests/health.rs`), on one page of each kind (`PAGES`;
  a new page goes in): no sideways scroll at 360 to 1440 px, every control
  44px tall at a phone's width, nothing the accessibility audit (axe-core,
  `e2e/vendor/`) finds, a turned phone's camera side alone kept clear
  (DevTools' safe-area insets and angle; skipped in Chrome 114), the
  header and the skip link below a 59px status bar and back (skipped
  there too), the
  Reduce Motion rule as the browser read it and a transition stopped by
  it, twenty loads in one tab each with its islands awake, no script
  error or CSP refusal (a recorder before the page's scripts), and the
  skip link first on Tab, leading to `<main>`.
- **Every test fails on a page error**: an exception, a console error or
  warning, a CSP refusal. A test that causes a failed load on purpose
  names its status and page (`page.allow_failed_load(404, "/no-such-page")`);
  any other line still fails it. A test that refuses a script expects its
  dependants' errors (`page.allow_errors()`).

What a failed test's page showed (screenshot, HTML, the server log's tail,
and every line the page logged) is in `target/e2e/<lane>/failures/`.

- **The memory tests turn DevTools' log off.** It keeps every element a
  log entry names, and Chrome notes the reset form on each visit, which
  looked like a leak of one `<main>` per Back.
- **Count a page's requests where they leave, `window.fetch` wrapped.**
  Chrome lists a request among the page's resources
  (`performance.getEntriesByType`) only once its answer is read, and a
  press's preload waits unread for the click.
- **A press is a mouse's at a measured point** (`Page::press`, built on
  `hold` and `let_go`): through DevTools in Chrome and Edge, WebDriver's
  own pointer in Firefox, held 300 ms so it preloads, as a person's does.
  Never the driver's element: chromedriver keeps each one it hands a test,
  and that holds the old page in memory. Safari's driver sends no
  `mousedown`, so there it's the page's own events, nothing held; on a
  device a finger's tap.

## Manual Testing

### Full build served locally

```bash
cargo leptos build --split && cargo loco start
```

Then, in another terminal:

```bash
for p in / /pkg/app.css /pkg/app.js /pkg/app.wasm /favicon/favicon.ico /robots.txt /nope; do
  printf '%-28s ' "$p"; curl -s -o /dev/null -w '%{http_code} %{size_download}B\n' "http://localhost:5150$p"
done
```

Expected: 200 for all but `/nope`, a 404 with one line of text, since curl
doesn't ask for HTML. With `-H 'Accept: text/html'` it's the not-found
page, still a 404, with `cache-control: no-store`. No `x-powered-by`
anywhere.

### Islands in the browser

Open http://localhost:5150 with the developer console open. Expected: no
errors, no warning about a missing island function, and the wasm and JS
requests in the network tab.

The home page's last line is the `IslandsCheck` lazy island: "Islands:
running" once its chunk has loaded and hydrated, "Islands: not running"
before. The network tab shows the split loader (`__wasm_split.*.js`) and
the island's own `split_islands_check_*.wasm`. If it says "running", the
whole chain works: server markers, bundle load, the split chunk,
`hydrate()`, hydration.

**Missing island function?** The island is in `views/`. Move it to
`src/islands.rs`.

### Moves in place

Start on `/reset?token=x`, console and network tab open, then:

1. Click the app's name: the home page swaps in, no reload (the network
   tab shows one document request marked by htmx, not a navigation), the
   tab's title changes, and "Islands: running" appears.
2. Scroll a little, click a link, then Back: the page comes back where it
   was scrolled.
3. Send the reset form with a short password: the form comes back with
   the reason, and the address stays `/reset?token=x`.

Expected: no console errors, no CSP violations. With a screen reader the
new title is read after each move. The request tests check the shell's
markup (`the_shell_moves_between_pages_in_place`); the browser tests
above check the swap, the focus, the scroll, a deploy and a gateway's 502.

### Rate limiting

Development allows a burst of 300 route requests per IP, refilling one per
second (`settings.rate_limit` in `config/development.yaml`). Against
`cargo loco start`, in one go:

```bash
seq 1 305 | xargs -I{} curl -s -o /dev/null -w '%{http_code}\n' http://localhost:5150/robots.txt | sort | uniq -c
```

Expected: `300 200` and `5 429`. Then the 429 page, with `retry-after: 1`,
`x-ratelimit-after: 1` and `cache-control: no-store`; an allowed answer
carries no `x-ratelimit-*`:

```bash
curl -i http://localhost:5150/robots.txt
```

`200`: assets are never limited:

```bash
curl -s -o /dev/null -w '%{http_code}\n' http://localhost:5150/pkg/app.css
```

`rate_limit` with its numbers and key source:

```bash
cargo loco middleware -c
```

Locally the key is the TCP peer, so a `CF-Connecting-IP` header is ignored.
On staging and production the key is that header, set by the CDN and passed
on by the proxy. On a deployed copy, a burst with
`-H 'CF-Connecting-IP: 203.0.113.9'` against `http://127.0.0.1:<port>/`
empties only that fake visitor's bucket. Hits without the header all share
the proxy's bucket.

An IPv6 visitor is keyed by its /64 network, not its address. On a deployed
copy, a burst that changes the address inside one /64 still empties one
bucket:

```bash
for i in $(seq 1 125); do curl -s -o /dev/null -w '%{http_code}\n' -H "CF-Connecting-IP: 2001:db8:1:2::$i" http://127.0.0.1:<port>/robots.txt; done | sort | uniq -c
```

Expected: `120 200` and `5 429`. With `2001:db8:1:$i::1` instead, every
request is a different /64, so all 125 pass.

The auth forms' posts share their own, smaller bucket:
`settings.rate_limit.auth`, burst 30 in development. `cargo loco
middleware -c` shows it under `auth` in the `rate_limit` entry, not as a
middleware of its own. To see it, 35 sign-ins, `30 400` and `5 429`:

```bash
seq 1 35 | xargs -I{} curl -s -o /dev/null -w '%{http_code}\n' -d 'email=nobody@example.com&password=x' http://localhost:5150/login | sort | uniq -c
```

Then a page, a `200`: the site-wide bucket of 300 still has tokens:

```bash
curl -s -o /dev/null -w '%{http_code}\n' http://localhost:5150/robots.txt
```

### Outgoing mail

With a catcher on port 1025 (`prerequisites.md`) and the dev server
running:

```bash
curl -s -o /dev/null -w '%{http_code}\n' -d 'name=Ana&email=ana@example.com&password=12341234' http://localhost:5150/register
```

Expected: `200`, and in the catcher's inbox (http://localhost:8025) one
mail from `TODO App <noreply@example.com>` (`settings.mail.from` in
`config/development.yaml`, never Loco's `System <system@example.com>`),
with a verification link starting `http://localhost:5150/verify?token=`
(`server.host` as written, not `host:port`).

### Secrets and the SMTP login at boot

Production refuses a secret that is set but not filled in. Run it with a
throwaway database so nothing lands in the repo:

```bash
LOCO_ENV=production MAILER_HOST=replace_me MAILER_USER=u MAILER_PASSWORD= MAILER_FROM="App <a@example.com>" DATABASE_URL="sqlite:///tmp/prod-check.sqlite?mode=rwc" ./target/debug/app start
```

Expected: exit status 1 and
`production won't start: MAILER_HOST, MAILER_PASSWORD empty or still replace_me`.

The SMTP login only runs when deployed, so check it on staging after a
deploy: `journalctl -u app-stg -b | grep 'smtp login check'`. A wrong host
or password is a `WARN` with the reason, and the site still answers.

### Hashed asset names

```bash
LEPTOS_HASH_FILES=true LEPTOS_SITE_ROOT=target/site-release cargo leptos build --release --split &&
LEPTOS_SITE_ROOT=target/site-release PORT=5153 ./target/release/app start
```

Then the page's `app.<hash>.css` and `app.<hash>.js`:

```bash
curl -s http://localhost:5153/ | grep -o 'href="/pkg/[^"]*"'
```

And the same hashes in the hash file, plus `split:` and `manifest:`:

```bash
cat target/release/hash.txt
```

Both files must be in `target/site-release/pkg`, and `target/site` keeps
the watch loop's plain names. Then check the three refusals:

1. Edit a hash in `target/release/hash.txt` and start the release binary
   again. It must refuse with "the hash file is stale".
2. Run `LEPTOS_SITE_ROOT=target/site-release cargo loco start`. The debug
   binary has no hash file next to it but `target/site-release` is hashed,
   so it must refuse with "a hashed build must be deployed together with
   its hash file".
3. Run `cargo leptos build` (no `--split`), then `cargo loco start`. It must
   refuse with "built without --split while a lazy island exists". Rebuild
   with `cargo leptos build --split`.

### Security headers

```bash
curl -sI http://localhost:5150/ | grep -iE '^(content-security|x-frame|referrer|permissions|cross-origin|strict-transport|x-content-type)'
```

Expected: all seven. The CSP has `'nonce-…'`, and a second request shows a
different one. `/robots.txt`, `/pkg/app.css` and `/nope` get the strict CSP
from the config instead (`default-src 'none'; style-src 'self'; …`). On a
deployed copy, run it against `http://127.0.0.1:<port>/`.

Under `cargo leptos watch --split -- start`, with the console open: no CSP
violation, and live reload still works (the development `connect-src`
allows its websocket).

After a deploy, scan the public host on https://securityheaders.com and
https://observatory.mozilla.org. CSP and X-Frame-Options must not fail.

**HSTS and `nosniff` may come from the CDN.** A CDN such as Cloudflare adds
its own in front of the app, so they show up even when the origin is down.

### Nightly restart

The timing is unit-tested. At the hour, the task sends SIGTERM to its own
process. You can try that any time. Build:

```bash
cargo build
```

Then start it and send SIGTERM after 5 s:

```bash
./target/debug/app start & sleep 5; kill -TERM $!; wait $!; echo "exit status $?"
```

Expected: `shutting down...` in the log and exit status 0, which the
unit's `Restart=always` turns into a restart. On a deployed copy,
`journalctl -u <unit>` shows `nightly restart scheduled for 03:00 UTC` at
boot, then next morning `nightly restart: stopping`, `shutting down...` and
a fresh boot.

### Request timeout

Nothing in the app hangs, so there's no automated test. To check the 15 s
`timeout_request`, add this to `src/controllers/home.rs` for a moment:

```rust
async fn slow() -> Result<Response> {
    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    format::text("too late")
}
// in routes(): .add("/slow", get(slow))
```

Run the server and time a request:

```bash
curl -s -o /dev/null -w 'status=%{http_code} seconds=%{time_total}\n' http://localhost:5150/slow
```

Expected: `status=408 seconds=15.0…`, `latency=15002 ms status=408` in the
server log, and `/` still answering in milliseconds. Remove the handler
afterwards.

### The app on the phones

No driver runs the app on a phone: a script takes its pictures, and the
rest is by hand. After an app change, and after a deploy that changes the
shell's edges (the header, the footer, anything fixed). Eight devices: the
iPhone 18 Pro, iPhone 17e, iPad Pro 13-inch (M5), iPad Air 11-inch (M4) and
iPad mini (A17 Pro) Simulators (iOS 27.2; the 17e the newest "e" model the
tools offer, the mini 744 wide upright, below `md`, so the phone's layout
on an iPad), the iPhone Duo's on iOS 27.1 (Apple's
foldable, open about 466×678 points, its status bar a strip at the right
side; `prerequisites.md`), the `android-phone-full` and
`android-tablet-full` emulators (a Pixel 10 and a Pixel Tablet at full
density, the pictures' own; the browser tests keep the half-density
`android-phone` and `android-tablet`). Then this Mac's own app. The site is
the dev loop's by default.

The dev loop's site (`host` in `config/development.yaml`): builds the app
once a platform, then one device at a time: boots it if off, photographs
the app, shuts it down if it booted it:

```bash
src-tauri/screens.sh
```

Another site, one a run:

```bash
src-tauri/screens.sh --https://example.com
```

Stopping at each Simulator for its sideways picture (⌘← in DeviceHub, then
Enter):

```bash
src-tauri/screens.sh --turn
```

The links that leave the app, tapped on `android-phone-full`: a call, a
mail, a text, another site and one in a new tab, put into the home page,
each to its own app, the app still on its page; exits 1 on a miss:

```bash
src-tauri/links.sh
```

By hand, a Simulator up, the same for each iPad:

```bash
xcrun simctl boot "iPhone 18 Pro" && open /Applications/Xcode.app/Contents/Applications/DeviceHub.app
```

By hand, an emulator up, the same for `android-tablet-full`:

```bash
A=/opt/homebrew/share/android-commandlinetools && ("$A/emulator/emulator" -avd android-phone-full -gpu swiftshader_indirect >/dev/null 2>&1 &)
```

- **`links.sh` after a change of how the app opens links**
  (`src-tauri/src/lib.rs`, the opener plugin): the opener's own script
  once left every link to a new tab dead (an islands app on this stack,
  2026-10-29). Android alone: an iPhone needs a real tap, which no
  Simulator takes from the terminal, so check 5 below by hand. The
  starter's pages have no link that leaves the site, so it puts five into
  the home page first. It reads the page through the webview's debugging
  socket with Node, the stylesheet lint's.
- **`devices.sh` is what both scripts share:** the site and its `/up`, the
  builds with the run's site, an emulator started and the app opened on
  it, the devices shut down again. Sourced, never run. The builds are
  `src-tauri/app-for.sh`'s, which the browser tests' app lane builds with
  too.
- **The site is the run's alone:** the app is built with its address (a
  copy of the start page and the CSP's `connect-src`, merged by the CLI's
  `--config`); `offline/index.html` and `tauri.conf.json` stay as they
  are. Its `/up` must answer `ok` before anything is built. An emulator
  reaches the Mac's `localhost` through `adb reverse`, a Simulator
  directly. From another Mac, give the dev loop's Mac by its `.local`
  name, the loop run with `BINDING=0.0.0.0`. The Android emulator resolves
  no `.local` name, so its app gets the address this Mac resolves it to;
  iOS keeps the name (App Transport Security refuses plain `http` to a
  bare IP). An emulator the script starts is kept awake for the run.
- **The Mac's app comes last,** built in debug at two window sizes,
  `tauri.conf.json`'s own and its smallest: `macos-1280x800.png` and
  `macos-360x480.png`, the window alone. The first run asks for Screen
  Recording for the terminal (System Settings, Privacy & Security); over
  ssh it's skipped. Windows is left out.
- The pictures land in `src-tauri/target/screens/`, emptied first: each
  device's `<device>-upright.png` and `<device>-sideways.png` at the page's
  top, each emulator's `<device>-bottom.png` at the page's foot too. The
  app's builds' output goes to `src-tauri/target/app-build-ios.log` and
  `app-build-android.log` (`app-for.sh`), the Mac's to `app-build.log`,
  each printed only when a build fails.
- A device this Mac lacks is skipped with a line. One another project has
  up is used as it is and left running.
- An emulator turns by itself. A Simulator has no command to turn or
  scroll it: a plain run takes its upright picture alone, `--turn` waits
  at each for it to be turned. iOS's foot is check 3 by hand.
- A picture waits for the site (the Simulator's WebKit log, the emulator's
  webview asked over its debugging socket) and, on an emulator, for the
  screen to stop changing: after a turn or a swipe the webview is white a
  while. `SCREENS_WAIT=120` for a slow site, the longest wait in seconds,
  60 by default.
- **A fresh emulator meets Cloudflare's challenge with a box to tick,**
  which only a person passes: the script skips that emulator and prints
  the line that opens it with its window, to open the app and tick the box
  once. The pass stays as a cookie. A Simulator passes by itself.
- **A Simulator is photographed by its own screens** (`primary`, and the
  iPhone Duo's `primary-1`), a dark one dropped: after a cold boot the Duo
  stands folded, its larger screen black. If the app is gone by its
  picture, the script opens it once more.
- **On the half-density emulators the camera's inset is twice a real
  Pixel's,** at the top upright and on the camera's side turned: 108 CSS
  px against a real Pixel 10's 54. The cutout is set in the real screen's
  pixels (`adb shell dumpsys display | grep -i cutout`). The `-full` pair
  shows it at its real size.
- **The emulator's window stays grey on macOS 27 without `-gpu
  swiftshader_indirect`.** Android runs; its screen isn't drawn.
- A `cargo tauri ios dev` or `android dev` run leaves its device up too;
  stop it first, since it holds the app's install.

1. **The start page:** "Connecting…", then the site. The network off
   before the start: "No internet connection"; back on: the site at once.
2. **The top, upright:** the header below the status bar, never under the
   clock or the Dynamic Island (`--safe-area-top`). Scrolled, the page
   fades out under the status bar: iOS's blur, and on Android the page's
   `theme-color` fading down from the clock (`android-phone-full-bottom.png`).
3. **The foot:** scrolled to the end, the footer above the iPhone's home
   indicator and Android's navigation bar. Not handled yet
   (`development.md`, "The app"): write down what each device shows.
4. **On its side, both ways round:** the camera's side clear, the other
   side at the column's gap.
5. **A link that leaves the site:** a tap on a phone number opens the
   dialer, on a mail address the mail app, on another site the browser,
   and the page is still there when you come back (`destination`,
   `src-tauri/src/lib.rs`). **On the iPhone and iPad, a real tap only.**
   WebKit drops a script's click on a link that opens a new tab as a
   popup, before the app is asked, so it proves nothing either way.

**Until `SITE` is a real site, the app built by hand stops at "The site
isn't reachable right now"** (example.com has no `/up`). The pictures
don't: they open the dev loop.

The same wait with the site up: the webview's console (`development.md`,
"The app").

### The app on Linux

By hand, on a Linux desktop (Ubuntu 22.04 or newer, Debian 12 or newer):
the AppImage from `src-tauri/linux.sh`, made executable and
double-clicked. Docker builds it but shows no window.

1. **The start page:** "Connecting…", then the site.
2. **The window:** at its size from `tauri.conf.json`, and dragged down to
   its smallest (360×480), the phone's layout.
3. **A call or a mail:** a tap on a phone number or a mail address opens
   the system's app, the page still there.
4. **The icon:** in the window's title bar and the dock or taskbar.

Write down what the distribution and its desktop show: this is the third
WebKit after Safari's and the iOS webviews', and untested so far.

## The Low Bar

The oldest browsers the template supports, about 30 months back: each top
lane's twin with `-low`, the same tests on another binary or device.
Edge and the Mac's Safari have none: no old Edge to download, and Safari
comes with macOS.

The seven: the phones and the desktop's, then the tablets:

```bash
e2e/run.sh low
```

One of them, nextest's filter as on a top lane:

```bash
e2e/run.sh chrome-low moves
```

| Lane | Site | Driver | Browser |
|---|---|---|---|
| `chrome-low` | 5169 | 9524 | Chrome for Testing 114, chromedriver 114 |
| `firefox-low` | 5170 | 9525 | Firefox 128 |
| `iphone-low` | 5171 | 9526 | Safari, iPhone 15 Simulator, iOS 17.5 |
| `ipad-low` | 5172 | 9527 | Safari, iPad Pro 13-inch (M4) Simulator, iOS 17.5 |
| `ipad-air-low` | 5173 | 9528 | Safari, iPad Air 11-inch (M2) Simulator, iOS 17.5 |
| `android-low` | 5174 | 9529 | Chrome 124, `android-phone-low` emulator on 5564, Android 15 |
| `android-tablet-low` | 5175 | 9530 | Chrome 124, `android-tablet-low` emulator on 5566 |

Chrome for Testing 114 with chromedriver 114, and Firefox 128, from
`~/Library/Caches/e2e-floor/` (below), or another folder in `E2E_LOW`.
The installed geckodriver drives Firefox from 115 on. Local only, never in
`ci.yaml`. In Chrome 114 the two leak tests and the two inset tests skip,
saying why (no `DOM.getDetachedDomNodes`, no safe-area insets); the rest
run.

| Browser | Low bar | Why that version |
|---|---|---|
| desktop Chrome | 114 (May 2023) | first with popovers |
| Firefox | 128 (Jul 2024) | Tailwind v4's floor |
| iOS | 17.5 (May 2024) | last iOS 17 |
| Android | 15, Chrome 124 (Apr 2024) | Android 14's Chrome is 113, before popovers |

Getting the browsers (macOS), in one folder and one shell.

Chrome for Testing 114:

```bash
curl -O https://storage.googleapis.com/chrome-for-testing-public/114.0.5735.90/mac-arm64/chrome-mac-arm64.zip
```

Its driver (CfT's own drivers start at 115):

```bash
curl -O https://chromedriver.storage.googleapis.com/114.0.5735.90/chromedriver_mac_arm64.zip
```

Chrome unpacked by `ditto`, not unzip: the app holds links:

```bash
ditto -x -k chrome-mac-arm64.zip ~/Library/Caches/e2e-floor/chrome-114
```

The driver unpacked where `run.sh` looks:

```bash
ditto -x -k chromedriver_mac_arm64.zip ~/Library/Caches/e2e-floor/chromedriver-114
```

Firefox 128, Tailwind v4's floor:

```bash
F=~/Library/Caches/e2e-floor && curl -fL -o "$F/firefox-128.dmg" "https://ftp.mozilla.org/pub/firefox/releases/128.0.3/mac/en-US/Firefox%20128.0.3.dmg" && hdiutil attach -nobrowse -quiet -mountpoint /tmp/ff128 "$F/firefox-128.dmg" && mkdir -p "$F/firefox-128" && cp -R /tmp/ff128/Firefox.app "$F/firefox-128/" && hdiutil detach -quiet /tmp/ff128 && rm "$F/firefox-128.dmg"
```

- iOS 17.5: the `.dmg` from developer.apple.com/download/all, then
  `xcrun simctl runtime add <dmg>`. A disk image, not a cryptex, or
  safaridriver opens no session.
- Android 15: `system-images;android-35;google_apis_playstore;arm64-v8a`,
  chromedriver 124. Each low emulator once with its window, then, in one
  shell, Chrome back to the image's 124:

```bash
A=/opt/homebrew/share/android-commandlinetools/platform-tools/adb && $A shell dumpsys package com.android.chrome | grep -m1 versionName && $A shell pm uninstall-system-updates com.android.chrome && $A shell dumpsys package com.android.chrome | grep -m1 versionName
```

The Play Store off, so it cannot move Chrome again:

```bash
$A shell pm disable-user --user 0 com.android.vending
```

- **The low emulators' Play Store stays off.** It moved both to Chrome 156
  on 2026-10-29, and the low lanes ran on 156's driver without a word;
  `e2e/run.sh` now stops a low lane whose Chrome is not 124. Going back
  resets Chrome's data, so switch its translate offer off again after.

Traps that looked like site bugs and weren't:

- **macOS freezes each new Chrome for Testing version** on a Bluetooth or
  local-network prompt until someone clicks Allow on that Mac. Every test
  just times out.
- **Chrome 117 to 121 don't start on macOS 27.** 114 to 116 and 122 and up do.
- **Boot emulators cold (`-no-snapshot`).** A snapshot a broken run saved
  lost Chrome's connection and every `adb` forward.
- **Not all low lanes at once,** even on 64 GB: run them in two batches.
- **iOS 17.5 opens new tabs empty:** no cookies, no `localStorage`. A
  second-tab test checks first and skips with the reason.
- **iOS 27 Simulators time out "connecting to a Safari instance"** until
  Xcode has been opened once after its update to install its parts.
- **iOS 27.2 is drivable only added as its disk image**
  (`xcrun simctl runtime add <the .dmg>`); its cryptex finds no session
  hosts. iOS 27.1 has only the iPhone Duo.
- **No driver sees a real iPhone's safe areas.** Turn a phone with Safari's
  Web Inspector open and run the measuring line (`development.md`, "Phones
  on their side").
- **`checkVisibility` before Chrome 121** knows only the old option names:
  pass `{ visibilityProperty: true, checkVisibilityCSS: true,
  opacityProperty: true, checkOpacity: true }`.
- **`scrollIntoView({ behavior: 'instant' })` still glides in Chrome 114**
  under `scroll-behavior: smooth`: set `scrollBehavior = 'auto'` on
  `<html>` around it.
- **Chrome 114's DevTools has no `DOM.getDetachedDomNodes`:** the memory
  tests skip there, asked of the browser, not by version.
- **A retried test reuses names** unless each try makes its own.

Site bugs it found, for when an app uses these features:

- **Firefox 128's click has no `pointerType`.** A hover-opened panel took
  it for not-a-mouse and closed: keep a hovered flag from `pointerenter`
  and `pointerleave` instead.
- **No popovers (below the bar):** a notice shown by CSS alone,
  `hidden [@supports_not_selector(:popover-open)]:block`, and every
  `[popover]` and `[popovertarget]` hidden, or the buttons do nothing.
- **No anchor positioning (Chrome before 125, Firefox 128, Safari before
  26):** a fallback placing dropdowns under the header.
- **Ask for the property you use: `@supports not (position-area: bottom)`.**
  Chrome 125 to 128 have `anchor-name` but only the old `inset-area`.
- **No subgrid in Chrome 114 to 116:** `grid-cols-subgrid` lists stack
  every value on its own line.
- **Chrome 114 to 123 stop starting islands** after 9 to 14 whole loads
  in one tab (idle time starved). Light pages never get there. Leptos
  0.9's loader waits 2 s at most.

Probe before fixing: a small test that prints what the page saw, on the
failing lane and on its current twin. Rerun a failure alone before
believing it.

## Tests to Add With the Features They Cover

- **Each new page:** a request test for status, `lang` and one distinctive
  string.
- **The first island** (a login or registration form): validation rejects
  bad input, a valid submission reaches the JSON API (its rate-limit bucket
  is already covered in `tests/requests/rate_limit.rs`). A natural home for
  `rstest`. Its request test checks the server render:
  `<leptos-island data-component="Name_` (prefix only, the name carries a
  hash) and `<leptos-children>`, which proves the content went in as
  children and stayed out of the wasm.
