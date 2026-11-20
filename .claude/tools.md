# Tools: the starter's own list

Read by `/tools`. Every tool and its install line: `docs/prerequisites.md`; the update rules: its "Updates" section. The machines' names, aliases and the servers' addresses: `CLAUDE.local.md`.

## Machines

- **The development Mac:** the watch loop (`cargo leptos watch --split`), fmt and both clippys, the desktop browser lanes when the test machine is busy, the app builds (`src-tauri/app-for.sh`).
- **The test machine:** the suite (`cargo nextest run`) and the browser lanes and bars (`e2e/run.sh`), by commit hash, in a repo of its own with no GitHub on it.
- **The servers:** dev-server, staging, production (`docs/{dev-server,staging,production}.md`): the binary, Caddy, the units, Mailpit nowhere.

## Where things live

- Chrome's and Edge's drivers: `~/Library/Caches/chromedriver`, `/opt/homebrew/bin`; an emulator's Chrome's driver: `~/Library/Caches/chromedriver-android/<milestone>`.
- The low bar's browsers and drivers: `~/Library/Caches/e2e-floor` (`docs/prerequisites.md`, Browsers).
- The beta bar's: `~/Library/Caches/e2e-beta` (`docs/prerequisites.md`, The beta bar).
- The Android SDK: `/opt/homebrew/share/android-commandlinetools`.

## Pins

- The emulator 37.2.12, not 37.2.13 (crashed every lane): `docs/prerequisites.md`, Browsers, the devices, with the way back.
- The low bar's browsers and the AVDs' Chrome 124: `docs/testing.md`, the low bar.
- cargo-leptos 0.3.12 on both Macs.
