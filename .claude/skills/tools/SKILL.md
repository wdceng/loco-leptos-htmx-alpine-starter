---
name: tools
description: Use when the maintainer asks to check the tools (/tools): every tool the maintainer's projects are built, tested, packaged, signed and shipped with, on the development Mac, the test machine and the servers, against its latest release and each project's docs/prerequisites.md update rules. Read-only: one table and the commands that bring each machine in line, nothing updated.
---

# Check the tools

Read-only. Nothing is installed, updated, upgraded or removed; the report hands the maintainer the commands, one `&&` pipeline a machine.

The tools belong to the machines, not to a project: every project beside this one runs on the same development Mac and test machine, so one run covers them all.

## The project's list first

Read `.claude/tools.md` in the project's root: the machines by role and what each builds, tests or serves; where the drivers, the low bar's and the beta bar's browsers live; the pins. The machines' names, their ssh aliases and the servers' addresses are in the maintainer's own `CLAUDE.local.md`, never in git. No list: say so, run anyway, and propose one at the end (a new project copies the starter's).

## Sources

- **The projects:** every folder beside this one (its parent folder) with a `docs/prerequisites.md`, and any new one.
- **The tools:** each project's prerequisites.md, its Install, Optional and Browsers and devices sections: every install line there is a tool. Merged into one list, each tool with the projects that need it. This skill keeps no list of its own (Rule no. 1); a tool found in use but missing from every prerequisites.md is reported, not added.
- **The rules:** each project's Updates section (the starter's: monthly, the first Monday; a release at least two weeks old; security fixes at once; the development Mac first, then one Android lane, then the test machine; one major upgrade at a time; never while the browser lanes run; the way back kept). Where two projects' rules differ, the stricter wins and the report says so.
- **Pins and pairs:** where a prerequisites.md states them, beside their tool (an emulator build, sqlx-cli, tauri-cli, the low bar's versions, Appium and its XCUITest driver). Two projects pinning one tool differently is a clash on a shared machine: ⚠️. A pin the machines show but no prerequisites.md states goes in the report's last line.
- **The crates are not tools:** `/cargo` covers the upgrades, the `*-deps-check` skills the upstream bugs.
- **A testing period:** if the maintainer has called one in any project (the maintainer's word, or that project's memory notes), everything is read but every "to do" is "wait".

## Reading

Each read runs on the development Mac, then on the test machine as `ssh <test machine> 'zsh -lc "…"'` (the login shell, for Homebrew's and the Android SDK's paths; the alias in `CLAUDE.local.md`). The test machine may be busy with a bar or another project's run: reads only, never a build, never an emulator started.

| Group | Reads |
|---|---|
| macOS, Homebrew | `sw_vers -productVersion`, `softwareupdate -l`, `brew --version`, `brew outdated --greedy --verbose` |
| Rust | `rustup check`, `rustc -V`, `rustup target list --installed`, `rustup component list --installed` |
| Cargo tools | `cargo install --list`, `cargo install-update -l` (the latest beside each); a tool installed from git against the rev its prerequisites.md names |
| cargo-leptos's downloads | each project's `wasm-bindgen` in its `Cargo.lock` against the wasm-bindgen-cli cargo-leptos fetched, with Tailwind's and wasm-opt's versions, in cargo-leptos's cache |
| Node | `node -v`, `npm -v`, `npm ls -g --depth=0`, `npm outdated` in each project with a `package.json`, `appium -v`, `appium driver list --installed` |
| Apple | `xcodebuild -version`, `xcodebuild -showsdks`, `mas outdated`, `xcrun simctl list runtimes`, WebDriverAgent's build in `~/Library/Developer/Xcode/DerivedData/WebDriverAgent-*` against the Xcode, `safaridriver --version` |
| Android | the SDK's `sdkmanager --list_installed` and `--list` (never `--update`), `emulator -version`, `avdmanager list avd`; an AVD's Chrome only if its emulator is already running |
| Browsers | each app's `CFBundleShortVersionString` (`defaults read "/Applications/<app>.app/Contents/Info.plist" CFBundleShortVersionString`): Chrome, Edge, Firefox, Safari, and the betas the projects use (Edge Beta, Firefox Developer Edition, Safari Technology Preview) |
| Drivers | `--version` of each driver a project's prerequisites.md installs, wherever it puts it (the list's paths), each beside its browser |
| Basics | git, gh, rsync, python3, jq, sqlite3, Zig, Mailpit, xcodegen, CocoaPods, the JDK (`/usr/libexec/java_home -V`), each where a project uses it |
| Expiry dates | the signing certificates (`security find-identity -v -p codesigning`, each one's date by `security find-certificate -c "<name>" -p \| openssl x509 -noout -enddate`) and the provisioning profiles (`security cms -D -i <file> \| plutil -extract ExpirationDate raw -`). Dates only, never a key or a secret; a file with no date (a push key, a service account) is "no expiry to read" |

The servers are the maintainer's to reach (the maintainer runs the server commands). From each project's deploy docs (`docs/staging.md`, `dev-server.md`, `production.md`, or its own), the box's units; its address from `CLAUDE.local.md`. The report hands the maintainer one read-only line a box and fills the server's column from what comes pasted back. Its shape:

```bash
ssh root@<box> "lsb_release -ds; uname -r; ls /var/run/reboot-required 2>/dev/null; apt list --upgradable 2>/dev/null | tail -n +2; caddy version; systemctl is-active <units>"
```

## Latest

From each tool's own channel, with its release date: crates.io for the cargo tools, Homebrew's formula and cask pages, npm (`npm view <package> version time`), Google's Chrome for Testing feed (`last-known-good-versions.json`), Mozilla's, Microsoft's Edge release notes, Apple's (Xcode, the Simulators, Safari Technology Preview), Android's emulator release notes, the GitHub releases of the rest. A date not found is said so, never guessed.

## Verdicts

| Mark | Meaning |
|---|---|
| ✅ | up to date |
| ⬆️ | update: a newer release at least two weeks old |
| ⏳ | wait: the newer release is under two weeks old, or a testing period is on |
| 🔒 | a security fix: update now, outside the month |
| 📌 | pinned, with its reason and the project that pins it |
| ⚠️ | a pair that must match does not (a driver and its browser, Xcode and the newest Simulator runtime, WebDriverAgent and Xcode, wasm-bindgen-cli and a project's crate), or two projects pin one tool differently |
| ↔️ | the development Mac and the test machine differ |
| 🔑 | expires within 30 days, or has expired |

## Report

- One table, grouped by stage (write, build, test, package, sign, ship, run, the system): `Tool | Used by | Development Mac | Test machine | Server | Latest (date) | To do`.
- Then the commands, one `&&` pipeline a machine, each a block of its own with its check: the development Mac's first, then one Android lane of a project that has one (`e2e/run.sh android`), then the test machine's. Only the ⬆️, 🔒 and ⚠️ rows; never a pinned one.
- Then each box's line for the maintainer, with its check.
- Then one line: what no prerequisites.md states, and `/cargo` for the crates.
- What another project's session installed for a while is listed with that session's name and left alone.

## Outstanding and verdict

The report ends with what is still open, then one verdict.

| Item | State | Waits for | Earliest date |
|---|---|---|---|
| a release under two weeks old | ⏳ | its two weeks | the day it turns two weeks old |
| a pinned tool | 📌 | what unblocks it (a stable major, a fixed emulator release) | the release that frees it, or none known |
| a major | ⚠️ | its own plan and commit | the maintainer's call |
| an expiry | 🔑 | renewal | its expiry date |
| a pair that does not match | ⚠️ | the command above | now |

Then the verdict, one line, with the count of each kind (safe now, waiting, pinned, majors, expiries):

- 🟢 **all in line:** nothing to do until the next first Monday.
- 🟡 **update when convenient:** updates safe now, none urgent.
- 🔴 **act now:** a security fix, an expiry within 30 days, or a pair that stops a build or a lane.

## The list's shape

`.claude/tools.md`, in the project's git, short: `## Machines` (each by role, what it builds, tests or serves), `## Where things live` (the drivers, the low bar's and the beta bar's browsers, the caches), `## Pins` (each with its reason and where the project states it). No machine's name, alias or address: those stay in `CLAUDE.local.md`.
