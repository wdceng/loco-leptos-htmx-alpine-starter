# Development

Day-to-day commands and a few traps. First time here? Start with "Run it in
five minutes" in `README.md`.

## Run

```bash
lsof -t -i tcp:5150 -i tcp:3001 -a -c app -c cargo-lep | xargs kill; cargo leptos watch --split -- start                     # free an old loop's 5150 and 3001, then the dev loop for this Mac only
lsof -t -i tcp:5150 -i tcp:3001 -a -c app -c cargo-lep | xargs kill; BINDING=0.0.0.0 cargo leptos watch --split -- start   # the same, for every device on the network
```

Open http://localhost:5150. Save a file and the page updates by itself, like
hot reload. Strictly it's live reload: a Rust change reloads the whole page,
a change to `style/tailwind.css` swaps in without one. Real hot reload needs
nightly Rust, so we don't use it.

Keep the `-- start`. Without it the app only prints its help.

Keep `--split` on every cargo-leptos command. Without it no island runs, and
the server refuses to start ("Lazy islands" below).

**On your phone:** use the second line, get your Mac's address with
`ipconfig getifaddr en0` and open `http://<address>:5150`. Allow `app` if
the firewall asks. The phone doesn't reload by itself, so refresh by hand.
Only do this on a network you trust.

**Server only**, without rebuilding the wasm or the CSS:

```bash
cargo loco start    # run once
cargo loco watch    # restart on every change (needs cargo-watch)
```

Run one loop at a time, they share port 5150, and the watch loops 3001 for
the live reload (Leptos's default). The `lsof` above stops an old loop's
`app` and `cargo-leptos` alone, never another program on those ports. Another
port: `PORT=5151 cargo leptos watch --split -- start`.

- **"Reload TCP port 127.0.0.1:3001 already in use":** another program
  holds 3001, VS Code's Markdown Preview Enhanced for one. Close its
  preview, or put `LEPTOS_RELOAD_PORT=3002` on the command.

```bash
ps aux | grep 'app start'   # find the running app
top -pid <PID>              # watch it
```

## Inspect

```bash
cargo loco routes                                  # every URL
cargo loco middleware                              # which middleware is on
cargo loco middleware -c                           # the same, with settings
cargo loco doctor                                  # check config and environment
LOCO_ENV=staging cargo loco middleware             # another environment
LOCO_CONFIG_FOLDER=/path/to/copy cargo loco start  # try an edited copy of config/
```

## Lint and Format

CI runs these, so run them before you push:

```bash
cargo fmt
cargo clippy --all-targets
cargo clippy --lib --target wasm32-unknown-unknown --no-default-features --features hydrate -- -D warnings
npm run lint   # the stylesheet: ESLint (npm install once, Node 24+)
```

The wasm32 clippy checks the browser half. It's the only one that sees
island code.

- **A Baseline finding on a feature a browser can do without** gets
  `/* eslint-disable css/use-baseline */` … `/* eslint-enable … */`
  around those lines alone, with the reason (`@property` in
  `style/tailwind.css`).
- **A mistyped class** (`text-primay`) is caught by nothing: Tailwind
  writes no CSS for it and says nothing. Look at the page.

## Database and Tasks

```bash
cargo loco db status          # which migrations ran
cargo loco db migrate         # run new ones
cargo loco db reset           # wipe and rebuild (development only)
cargo loco db entities        # regenerate src/models/_entities/ (needs sea-orm-cli)
cargo loco task user_create   # run a task; `cargo loco task` lists them
```

Migrations also run on every start, so you rarely need `migrate`. The
database is `app_development.sqlite`: delete it any time, the next start
makes a new one.

### After a migration, or a new or changed query

The app's own queries are SQLx `query!` macros, checked at compile time
against the schema (`architecture.md`, "Sea-ORM and SQLx"). Every build
reads that schema from the committed `.sqlx/` cache, never a database:
`SQLX_OFFLINE = "true"` in `.cargo/config.toml` makes sure of it, on every
machine and in the server build. The one step that touches the database is
`cargo sqlx prepare`, so after a schema or query change, refresh the cache:

```bash
cargo loco db migrate                                                          # the schema first
DATABASE_URL=sqlite://app_development.sqlite cargo sqlx prepare -- --all-targets   # then the cache
```

Commit `.sqlx/` with the change. CI fails if you forget.

- **Never `export DATABASE_URL`.** Every config reads it, so `cargo test`
  would run against that database and wipe it. Set it on the one command.
- **Always `-- --all-targets`.** Without it the tests' queries drop out of
  the cache.
- **Checking against the live database** while you write a query: put
  `SQLX_OFFLINE=false DATABASE_URL=sqlite://app_development.sqlite` in front
  of `cargo check --all-targets`. A wrong column then fails the build with
  `no such column`.

## Tests

```bash
time cargo nextest run                  # all, the usual way: faster than cargo test, the same tests
cargo nextest run reset                 # those whose name holds it
cargo test                              # all, the plain way
cargo llvm-cov nextest --summary-only   # all, with each file's coverage
```

No server or browser needed. The browser tests and more: `testing.md`.

## Release Builds

```bash
LEPTOS_HASH_FILES=true LEPTOS_SITE_ROOT=target/site-release cargo leptos build --release --split   # build
LEPTOS_SITE_ROOT=target/site-release ./target/release/app start                                   # run it locally
```

Always keep `LEPTOS_HASH_FILES=true`. It puts a hash in the file names
(`app.<hash>.css`) and writes `hash.txt` next to the binary, so production
can cache files for a year. Without it, visitors get stale files.

Always keep `LEPTOS_SITE_ROOT=target/site-release` too. The watch loop owns
`target/site`; a release build there would replace its files with hashed
ones. Kept apart, both can run at once: give the release one
`PORT=5153`.

You get `target/release/app`, `target/release/hash.txt` and
`target/site-release/`.

Size, for reference: with no island the wasm is about 28 KB gzipped. The
first island brings in the Leptos runtime, about 52 KB gzipped. More islands
cost much less, and a lazy one only on the pages that render it.

### For a Linux server

The server part is built natively with `cargo zigbuild`: Zig links it for
Linux, no container.

```bash
LEPTOS_HASH_FILES=true LEPTOS_SITE_ROOT=target/site-release cargo leptos build --release --split --frontend-only   # site files and hash.txt
CARGO_TARGET_DIR=target/zig cargo zigbuild --release --target x86_64-unknown-linux-gnu.2.31                       # target/zig/x86_64-unknown-linux-gnu/release/app
```

- `.2.31` is the oldest glibc the binary needs (Step 0 in `production.md`).
- `target/zig` keeps this build apart from the Mac's own, so neither
  rebuilds the other.
- For the dev server, `--profile dev-server` instead of `--release`: it
  builds faster. Staging and production build `--release`.
- Full steps in `dev-server.md`, `staging.md` and `production.md`.

**Keep `strip = true` on every profile you deploy.** Zig's linker doesn't
merge debug strings: line tables made a sister app's binary 2 GB.

**`ignoring deprecated linker optimization setting "1"` is harmless.**
rustc passes `-O1`, Zig's linker ignores it.

## Lazy islands

`#[island(lazy)]` moves an island out of the main wasm into a chunk of its
own, fetched only by a page that renders it. `IslandsCheck` on the home page
is one (`src/islands.rs`).

```bash
tools/cargo-leptos/check.sh   # does this machine's cargo-leptos have both fixes?
tools/cargo-leptos/install.sh # if not: build and install it (~3 min)
```

- **`--split` on every cargo-leptos command:** the watch loop, local
  builds, deploys, CI. Without it the main script imports a module that
  doesn't exist, so no island runs on any page. The server checks at boot
  and refuses that build (`src/assets.rs`).
- **Lazy when only some pages use it.** An island on every page saves
  nothing as lazy. Render an island only where it's needed: rendered means
  fetched.
- **`hash.txt` gains `split:` and `manifest:` lines.** It travels with the
  binary as before.

**Why the patched cargo-leptos.** Released cargo-leptos (0.3.11) has two
bugs with `--split` and hashed names. The watch loop doesn't hash, so they
show only in release builds:

| Bug | Effect | Fix |
|---|---|---|
| [cargo-leptos #690](https://github.com/leptos-rs/cargo-leptos/issues/690) | The main `.js`/`.wasm` keep their old names after the loader's name is written into them. After a deploy, a cached main script imports a loader that's gone: no island runs until a hard refresh | [PR #691](https://github.com/leptos-rs/cargo-leptos/pull/691), commit `0fb3de86`. Merged 2026-10-16, not released |
| [wasm-split-prototype #79](https://github.com/WorldSEnder/wasm-split-prototype/issues/79) | The loader's imports come out in random order, so every build gets new names and visitors download it all again | `tools/cargo-leptos/wasm-split-sort.patch`, the same code as upstream's `cc582f1`. Merged 2026-10-16, not released, and released cargo-leptos still locks 0.2.3 |

`install.sh` builds cargo-leptos at that commit, against the patched crate.
CI uses the same script, and checks that two hashed builds give the same
names and that the main script names a loader that exists.

**Released yet?**

```bash
tools/upstream.sh   # every upstream fix the template waits for, and whether it's released
```

It watches four: the two above, leptos #4953 (islands disposed when they
leave the page) and htmx #4147 (Chrome dropping the comments Leptos
hydrates by). Fix 2 counts only once a cargo-leptos **release's own
`Cargo.lock`** locks a newer `wasm_split_cli_support`: `cargo install
--locked` takes that lock, not crates.io's latest. When both split fixes
are in a release it prints a two-build check that installs the plain
release into a scratch folder, so your working cargo-leptos stays.
**If that passes:** `cargo install --locked cargo-leptos@<version>`, the
same line in CI, and delete `tools/cargo-leptos/` ("Way back" in
`docs/decisions.md`).

## Environments

`LOCO_ENV` picks the file in `config/`. There's no `.env` file, secrets come
from environment variables.

| Environment | For | Files from | Browser cache | Host |
|---|---|---|---|---|
| `development` | your machine (default) | `target/site` | rechecks every time | `http://localhost:5150` |
| `test` | `cargo test` | none | none | `http://localhost:5150` |
| `e2e` | the browser tests (`e2e/run.sh`) | `target/e2e/site` | rechecks every time | `http://localhost:5160` |
| `dev-server` | development copy online, staging's twin | `site/` | 60 s | `server.host` in the config |
| `staging` | test copy online | `site/` | 60 s | `server.host` in the config |
| `production` | live site | `site/` | one year | `server.host` in the config |

The host is where links in e-mails point. It's used exactly as written, no
port added, so if you run locally on another port, change it in
`config/development.yaml` too.

With a one-year cache, a changed file needs a new URL:

- **CSS, JS, wasm:** release builds put a hash in the name.
- **Everything in `public/`:** `build.rs` hashes each file and generates a
  constant for it in `paths::assets`, in modules that follow the folders
  (`paths::assets::favicon::APPLE_TOUCH_ICON_PNG`). Drop a file anywhere in
  `public/`, link it through its constant, done. Replacing it changes only
  its own `?v=`. A page that links a `public/` file as a plain string fails
  the links test.
- **A file named in `style/tailwind.css`** (a self-hosted font, say)
  carries the `?v=` written out. Replace it and `cargo test` fails with the
  exact value to paste.

### On the server

Deploying, the server layout and the systemd unit: `dev-server.md`,
`staging.md` and `production.md`. Each one is a single file you follow top
to bottom.

## Add a page or an island

A page, `/about` from the home page:

1. Copy `src/views/home.rs` to `src/views/about.rs`, rename the component,
   add `pub mod about;` to `src/views/mod.rs`.
2. Copy `src/controllers/home.rs` to `src/controllers/about.rs`, point it
   at the component, set its `PageMeta` title and the URL in `routes()`,
   add `pub mod about;` to `src/controllers/mod.rs`.
3. In `src/app.rs`, `fn routes`: add `.add_route(controllers::about::routes())`.
4. A public page gets a line in `/llms.txt` (`src/controllers/llms.rs`).

`cargo loco routes` lists every URL.

An island, for what htmx and Alpine can't do:

- **In `src/islands.rs`,** the only file also compiled for the browser: an
  island anywhere else renders but never runs.
- **Its whole body inside `collected(|| { … })`,** so it's freed when htmx
  swaps it out. A test fails without it.
- **`#[island(lazy)]` when only some pages use it,** as `IslandsCheck`
  (with the plain `island` for tests).
- **Content comes in as `children`:** it renders on the server and never
  goes to the browser. **Props are plain data:** a `String`, a number, a
  struct with `Serialize` and `Deserialize`. **Nothing server-only,** like
  the database: ask the JSON API.

Then clippy on both halves (Lint and Format, above).

## The app

```bash
cd src-tauri && cargo tauri build --bundles app                                         # macOS: the .app in src-tauri/target/release/bundle/macos/
export PATH="/opt/homebrew/opt/llvm/bin:$PATH" && cd src-tauri && cargo tauri build --no-bundle --runner cargo-xwin --target x86_64-pc-windows-msvc   # Windows: the .exe in src-tauri/target/x86_64-pc-windows-msvc/release/
src-tauri/linux.sh                                                                      # Linux: the AppImage in src-tauri/target/linux/x86_64/, built in a Docker box of Ubuntu 22.04 (Docker running; about 15 min the first time, 3 after); LINUX_ARCH=aarch64 for arm64
cd src-tauri && cargo tauri ios dev                                                     # iPhone or iPad Simulator: pick one from the list
cd src-tauri && cargo tauri android dev                                                 # Android emulator (start one first) or a phone on USB
src-tauri/screens.sh                                                                    # the app on the eight devices and this Mac against the dev loop, booted where not up, photographed; `--https://<site>` for another (testing.md, "The app on the phones")
cd src-tauri && cargo tauri ios build                                                   # the App Store build; needs your Apple team (below)
cd src-tauri && cargo tauri android build --aab                                         # the Play Store bundle; needs your signing key
cd src-tauri && cargo tauri icon ../public/favicon/android-chrome-512x512.png -o /tmp/app-icons   # the desktop icons again, after the site's icon changes
```

The desktops and the phones run the same app: `run` in `src-tauri/src/lib.rs`.

- One window showing the live site, so every deploy shows at once. The app
  is rebuilt only when `src-tauri/tauri.conf.json`, the start page or the
  icon changes.
- The window opens the start page built into the app,
  `src-tauri/offline/index.html`: "Connecting…", then the site once its
  `/up` answers `ok`. Until then it says whether the internet or the site
  is down, with "Try again" and a check every 5 s.
- **The address is in two places:** `SITE` in the start page and
  `connect-src` in `tauri.conf.json`. `tests/config.rs` fails when they
  differ.
- **A gated host lets `/up` past its challenge** (`dev-server.md`,
  `staging.md`), or the start page waits for good.
- **A connection lost later is the webview's:** on a Mac the page stays
  until it's back, on Windows F5 reloads. A server down later shows the
  proxy's error page, not the start page.
- A crate and lock of its own: the site's builds never compile Tauri. No
  capabilities file, so the site gets no access to the computer.
- **Unsigned:** the first start asks once. Windows: "Windows protected
  your PC", _More info_, _Run anyway_. macOS: right-click the app, _Open_.
- **A downloaded Mac app is "damaged":** macOS's quarantine on an
  unsigned app. This lifts it:
  `xattr -dr com.apple.quarantine "/Applications/SaaS Starter.app"`
- Each build is the program alone, nothing to install. Windows gets no
  installer: the `.exe` is the app, a few MB that open the site.
- **Keep only the icons the builds read:** `icon.ico`, `icon.icns`,
  `icon.png` and the three PNGs in `tauri.conf.json`. `cargo tauri icon`
  writes phones' and the Store's too.
- **macOS 26 shrinks a square icon into a gray tile.** A Mac icon of its
  own follows Apple's template: a 1024 canvas, an 824 rounded square.
- **Finder can keep showing an old icon** from its cache: move the app, or
  `touch` it.
- **Linker warnings `LNK4099` on the Windows build are harmless.**
- **Linux builds only on Linux:** Tauri's `.deb`, `.rpm` and AppImage
  bundlers exist in a CLI running there, and the app links against
  Linux's WebKitGTK and GTK. So `linux.sh` builds inside Docker
  (`src-tauri/linux/Dockerfile`), on Ubuntu 22.04, the oldest base
  Tauri's docs name: the AppImage runs there, on Debian 12 and on anything
  newer (the binary asks for glibc 2.34 or newer). x86_64 is emulated on
  an Apple-silicon Mac, hence the minutes; arm64 builds at full speed. The
  box is about 3 GB. The AppImage carries WebKitGTK and GTK themselves
  (169 libraries, 81 MB packed), so nothing is installed first.
- **The Linux binary is `saa-s-starter`:** Tauri writes the product name
  in kebab-case into the AppImage. The file keeps the name with its space.
- **Tauri warns that `com.example.app` ends in `.app`,** the Mac's bundle
  extension. A project's identifier shouldn't (README, "Start a new
  project").

The phones:

- **`gen/apple` and `gen/android` are Tauri's,** written by `cargo tauri
  ios init` and `android init` in a scratch worktree and kept as
  generated. Their names and ids come from `tauri.conf.json` at that
  moment: after a rename, generate them again.
- **Two Android files aren't as generated:** `MainActivity.kt` (the
  status bar's colour) and `core-ktx` in `app/build.gradle.kts`. A new
  `android init` overwrites them: bring both back.
- **The identifier: letters, digits and dots only.** Tauri turns a `-`
  into `_` for Android and a `_` into `-` for iOS, so either gives the app
  two ids. It's permanent once the app is installed anywhere.
- **A build leaves the tree clean:** each iOS build rewrites
  `gen/apple/app-native_iOS/Info.plist`, committed as it writes it,
  without a final newline; the build output is ignored by `gen/`'s own
  `.gitignore` files.
- **The phones' icons live in `gen/`:** `cargo tauri icon` without `-o`,
  in a scratch worktree, writes them there; bring `gen/` back, not the
  extra desktop icons.
- **The Simulator and the emulator need no account.** A real iPhone or
  iPad needs Developer Mode on (Settings, Privacy & Security) and your
  Apple team: `APPLE_DEVELOPMENT_TEAM=<team id>` on the command or
  `bundle.iOS.developmentTeam` in `tauri.conf.json`, or the build warns "No
  code signing certificates found". The stores need Apple's developer
  program and a Google Play listing, each with its signing.
- **Android builds with Java 21, not a Mac's default 27:** Gradle 9.6.1
  fails on 27 ("Unsupported class file major version 71"). `screens.sh`
  sets it; without the profile line (`prerequisites.md`), put
  `JAVA_HOME=$(/usr/libexec/java_home -v 21)` on `cargo tauri android`.
- **The start page waits with the site up?** Read the webview's console:
  "No 'Access-Control-Allow-Origin' header" means `/up` got something
  other than the site's own answer, a challenge or an error page. Safari,
  Develop, the Simulator, for iOS; `chrome://inspect` for Android.
- **Apple may turn down an app that only shows a website** (its review
  guideline 4.2): a store app should do something the site can't, such as
  push.
- **The app's webview lies under the status bar** on iOS, iPadOS and
  Android (`viewport-fit=cover`; `enableEdgeToEdge` in `MainActivity.kt`):
  the page's top adds `--safe-area-top` ("Phones: safe areas and browser
  bars"). Not yet seen in the app, which shows its start page until `SITE`
  is a real site; the foot and Android's navigation bar are untested.

## Start a new project

Replace the template's placeholders before your first deploy. Production's
check (`production.md`) stops on what's left.

| What | Where |
|---|---|
| The app's name in titles, header, footer and on home screens | `APP_NAME` in `src/views/layout.rs` |
| The description | `DESCRIPTION` in `src/controllers/home.rs` |
| The mail sender | `config/development.yaml`, `test.yaml`, `staging.yaml`; production reads `MAILER_FROM` |
| The security contact | `contact:` under `security:` in `config/production.yaml`; production won't boot on `example.com` |
| The tagline | `src/views/home.rs` |
| Favicon | `public/favicon/` |
| Colours | `style/tailwind.css`: `primary`, `surface`, `ink`, `ink-muted`, `danger` |
| The app's name, id and address | `src-tauri/tauri.conf.json` (`productName`, `mainBinaryName`, `title`, `identifier`, `connect-src`; the identifier letters, digits and dots only, not ending in `.app`); `<title>` and `SITE` in `src-tauri/offline/index.html`; then the phones' projects again ("The app") |

Renaming `app` is optional: "Rename the app" below.

## Rename the app

Optional: the project is called `app`, and keeping that breaks nothing.
To rename it, change `app` in:

- `Cargo.toml`, `.cargo/config.toml`, `src/bin/main.rs`,
  `examples/playground.rs`, `tests/`
- the `app_<env>.sqlite` lines in `config/*.yaml`
- every `app` naming the binary in `development.md`, `testing.md`,
  `dev-server.md`, `staging.md` and `production.md` (`target/release/app`,
  `dist/app`, `/srv/app/stg/app`, `LEPTOS_OUTPUT_NAME=app`). The `/srv/app/`
  folders and the `app-dev`, `app-stg`, `app-prod` units can stay

**Name the binary exactly like the package.** cargo-leptos trips over Loco's
`-cli` suffix.

**`LEPTOS_OUTPUT_NAME` must match.** Otherwise production links to asset
files that don't exist.

## Phones: safe areas and browser bars

The page reaches the glass (`viewport-fit=cover`), so its background runs
to every edge. What stands at a side keeps clear of the camera itself:

| Part | Job |
|---|---|
| `--safe-area-left`, `--safe-area-right` (`style/tailwind.css`) | the screen's insets, the one pair every side gap reads |
| `max(1.5rem, var(--safe-area-left))` | a side gap: the design's, or the inset where that's wider (`COLUMN`) |
| `--safe-area-top` | the status bar over the app's webview, 0 in a browser |
| `calc(1.5rem + var(--safe-area-top))` | a top gap: the design's plus the bar (the header, the skip link, the 429 page) |
| `public/js/safe-area.js` | marks the camera's side on `<html>` before the first paint (`data-safe-area`), so the other side gets the design's gap back |
| `md:`, `lg:` with a height of 480px too | a phone on its side keeps the phone's layout |

- **iOS reports the inset on both sides** of a turned phone, not only the
  camera's: hence the mark.
- **Chrome on an iPhone keeps its own view clear** (724 of 852 px) and
  still reports the inset: the mark says `none`, or the page would be
  indented twice (Chromium issue 465922618). It's set from the window's
  shape at the turn's first resize, before the angle comes.
- **A turn glides the sides over 500ms** (`@property` on the pair): the
  camera's side waits for the angle, and at 200ms it still read as a jump.
- **After a reload on its side, Chrome on an iPhone** lays the page out
  narrower than `innerWidth` until the first scroll. Nothing to fix from
  the page.
- **The top and the foot** keep the `standalone:` variant's fixed gaps:
  only the sides were measured on a phone. A top with both adds them:
  `standalone:` is room beside iOS's blur, the inset room for a bar over
  the page.
- **Whatever sets its own top adds the inset:** a padding on `body` doesn't
  reach a fixed, absolute or sticky element. A browser that reports a top
  inset it already keeps clear would add a gap: unchecked on a phone.
- **Measured** in an islands app on this stack, iPhone, iOS 27.2,
  2026-10-27. The angle-to-side map (90 is left) was seen on that phone in
  Safari, Chrome, Firefox and Edge, 2026-10-28.

### A background to the screen's foot

The template paints none. An app's glow, gradient or bottom bar meets
this on day one: a `fixed` box is cut at the height the browser reports,
while the screen shows more (Firefox on iOS with its bar hidden: 733 of
852 px; Safari and Chrome while their bars slide back in).

- **The way:** a `sticky` box a quarter screen taller than the view,
  `height: calc(100dvh + 25vh)` with a negative bottom margin of the same,
  so it takes no room. Its bottom edge carries on into that quarter.
- **Never `fixed`.** `lvh`, `dvh` and `-webkit-fill-available` alone don't
  fix it.
- **`<body>` stays transparent,** the page colour on `<html>`.
- **In a flex `<body>`, the box needs `width: 100%`.**
- **A variable built from others resolves where it's declared.** Turning
  it off means overriding both variables where they're used.
- **Six background layers need six values** in every list.
- **Unverified:** the end of a long page (a sticky box can't leave
  `<body>`), and iOS 26 or older. One phone on iOS 27.2 so far.

No driver can slide a browser's bars, so check by hand on an iPhone:
Firefox scrolling down, Safari and Chrome scrolling up, each to the footer,
and the home-screen app.

### Check a phone

Safari's Web Inspector (Chrome on iOS too: Settings, Content Settings, Web
Inspector), then this line in its console:

```js
(() => { const p = document.createElement('div'); p.style.cssText = 'position:fixed;padding-left:env(safe-area-inset-left);padding-right:env(safe-area-inset-right)'; document.body.append(p); const s = getComputedStyle(p); const r = { insetL: s.paddingLeft, insetR: s.paddingRight, iw: innerWidth, scroll: document.documentElement.scrollWidth, angle: screen.orientation?.angle, mark: document.documentElement.dataset.safeArea }; p.remove(); return r; })()
```

## Gotchas

- **Won't start after a release build?** It was built without
  `LEPTOS_SITE_ROOT=target/site-release`, so `target/site` has hashed names
  the debug build can't find. Run `cargo leptos build --split` or the watch
  loop.
- **Keep `LEPTOS_OUTPUT_NAME` in `.cargo/config.toml`.** Leptos needs it to
  name the wasm file the same way cargo-leptos does.
- **Some crates recompile** when you switch between `cargo leptos` and plain
  `cargo`. It only costs a few seconds.
- **`target/site` and `target/site-release` are wiped on every build.** Put
  static files in `public/`.
- **Two `Error` types.** Loco and Leptos both export one. In files that use
  both, import Leptos items by name, not `leptos::prelude::*`.
- **`as` and `type` are keywords.** In `view!`, write `r#as` and `r#type`.
- **Islands go in `src/islands.rs`.** One in `src/views/` renders, but never
  runs in the browser.
- **A grid column is `minmax(0, 1fr)`, never a plain `1fr`.** A plain
  `1fr` is at least as wide as its content, and Chrome on an iPhone kept a
  turned phone's columns that wide. Tailwind's `grid-cols-N` is already
  right; `tests/one_job.rs` stops a `1fr` in `src/`.
- **Keep `overflow-anchor: none` on `<html>`** (`style/tailwind.css`). With
  a toast or a loading bar after the page's region, Chrome and Firefox
  anchor on it and a move flashes the new page's footer.
- **With `scroll-behavior: smooth` a move's jump to the top glides.**
  Scroll it from the `Navigation` island, instantly, not by `show:top`.
- **`class={CONST} {..spread}` in `view!`, braces on the constant.**
  `class=CONST {..spread}` parses as a struct literal.
- **Leptos writes `&` as `&amp;` in an attribute.** A test asserts
  `[&amp;_option]`, not `[&_option]`.
- **A substring test hits class names.** `contains("checked")` passes on
  `checked:bg-…`; test the attribute (` checked` then a space, `>` or `=`).
- **Alpine's `.outside` does nothing on a `display: contents` element.**
  It skips an element with no size, and islands are `contents`: listen on
  the window and test `$el.contains($event.target)` yourself.
- **A popover counts as outside its opener.** It lives in the top layer:
  leave a tap inside `[popover]` out of a close-on-outside.
- **A morph keeps Alpine's state; a changed `x-data` resets it.** State the
  server writes into `x-data` is thrown away on every swap.
- **Safari's autofill grabs a field named `name` or holding "address".**
  Name it for what it is.
- **A value a script sets never counts as `tooShort`.** A test that types by
  script checks no `minlength`.
- **An empty live region is truly empty,** no space, or some readers
  announce it.
- **`leptos` comes from a fork, pinned by commit** (`[patch.crates-io]` in
  `Cargo.toml`): 0.8.22 with the island loader's idle timeout. On a Leptos
  upgrade, drop the block if the release has the timeout, else move the
  fork to the new version; `the_islands_loader_waits_for_idle_two_seconds_at_most`
  checks the loader either way.
- **Staging and production restart every night.** Development never does,
  so a watch loop left on overnight is still running in the morning.
- **The home-screen app reaches the glass.** Opened from a phone's home
  screen, the page has no Safari bars around it: give controls at the top
  and the foot room with `standalone:max-md:…` (`style/tailwind.css`).
- **A part with its own side padding ignores the insets.** Anything at a
  side reads `max(gap, var(--safe-area-left))`, as `COLUMN` does, or it
  sits under the camera of a phone on its side.
- **Anchors under a sticky header: `scroll-margin-top` on the targets.**
  `scroll-padding-top` on `<html>` made desktop Safari glide to the top
  whenever a field inside the sticky header took the focus.
- **No test sees the home-screen app.** WebDriver can't open one, and
  DevTools doesn't truly emulate `display-mode` (`matchMedia` stays false).
  Check it by hand on a phone.
- **Swipe the home-screen app away after a deploy, then reopen it.** A
  backgrounded app keeps its old CSS.
