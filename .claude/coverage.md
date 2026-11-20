# Test coverage: the starter's own list

Read by `/test-coverage`. The whole testing doc: `docs/testing.md`. The test machine and its repo: `CLAUDE.local.md`.

## Workspaces

- The root (`Cargo.toml`, `migration/` a member): `cargo nextest run`, the server's unit and request tests in `tests/`, the guards in `tests/one_job.rs`, the config in `tests/config.rs`.
- The app: `src-tauri/` (its own lock), its tests in `src-tauri/src/lib.rs`.
- The browser tests: `e2e/` (its own lock), the helpers in `e2e/src/`, the tests in `e2e/tests/`.

## Lanes and bars

- The script: `e2e/run.sh`, its header names every lane and bar (`desktop`, `phones`, `tablets`, `main`, `all`, `low`, `beta`, `app`).
- Each lane's files: `target/e2e/<lane>/` (`server.log`, `nextest.txt`, `skipped.txt`, `failures/`).
- The results log: on the test machine, named in `CLAUDE.local.md`.

## Floors

The low bar (`docs/testing.md`, "The Low Bar"): Chrome 114, Firefox 128, Safari on iOS 17.5, Chrome 124 on Android 15. The apps: `src-tauri/tauri.conf.json` and the generated projects in `src-tauri/gen/`.

## Parts

- The islands: `src/islands.rs`, `src/islands/` (`dialogs.rs`, `shrink.rs`).
- The views: `src/views/`, the mails in `src/views/mail.rs`.
- The stylesheet: `style/tailwind.css`.
- The hand-written JS: `public/js/`, the app's start page `src-tauri/offline/index.html`.
- The scripts: `e2e/run.sh`, `tools/upstream.sh`, `src-tauri/{app-for,devices,links,screens,linux}.sh`, `src-tauri/opaque.swift` (a build script).
- The native files: `MainActivity.kt` in `src-tauri/gen/android`; the rest of `gen/` is Tauri's.
- The config: `config/*.yaml`, `src-tauri/tauri.conf.json`, the Android manifest; no `capabilities/`.

## None here

- Shaders: none.
- Native unit tests: none.
- An iOS app lane: none yet; the Android app lane (`e2e/run.sh app`) only on the maintainer's word.
- Push, widgets, extensions: none.

## Hand checks

`docs/testing.md`, "Manual Testing".

## Maps so far

None yet.
