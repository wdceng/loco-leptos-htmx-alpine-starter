# Native audit: the starter's own list

Read by `/native-audit`. The app: Tauri 2 around the live site, `src-tauri/`, a crate and lock of its own; the phones' projects are Tauri's, generated into `src-tauri/gen/` (`android/`, `apple/`). The whole description: `docs/architecture.md`, "What Is in the App", the row "The app".

## Native parts

- **The start page** (web, built into the app): `src-tauri/offline/index.html`, handing over to the site once `/up` answers; says whether the internet or the site is down.
- **Links handed to the system** (a bridge in Rust alone): `destination`, `SYSTEM_SCHEMES` and `new_window` in `src-tauri/src/lib.rs`, through `tauri-plugin-opener`.
- **The status bar on Android** (native): `MainActivity.kt` in `src-tauri/gen/android`, `GradientProtection` in the page's `theme-color`, asked at each page load by `paint_status_bar` (`src-tauri/src/lib.rs`); iOS blurs it by itself.
- **The safe areas** (web): `public/js/safe-area.js`, `--safe-area-*` in `style/tailwind.css`.

## The bridge

None: no capabilities file, so the site calls no Tauri command. The app asks the page for its `theme-color` (Android); nothing goes the other way.

## App lanes

- `e2e/run.sh app` (Android; not in any bar, only on the maintainer's word): `e2e/tests/app.rs`, the start page, offline and back.
- No iOS app lane yet. What a lane cannot drive: `docs/testing.md`, the checks by hand.

## Calls

- No push, no passkeys, no Web Share in the app yet: each, once wanted, a Tauri plugin plus a capability naming the site's origin (`docs/architecture.md`).
- The plugin's own script for links to a new tab is off (`open_js_links_on_click(false)`): it asked the app over IPC, which the site may not.

## Tried and dropped

`docs/tried-and-dropped.md`.
