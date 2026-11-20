---
name: tauri-deps-check
description: Inspect a Tauri app's framework stack for upstream bugs and issues that reach it. Tauri (every crate of tauri-apps/tauri, the installed tauri-cli and its mobile templates) is the major; wry, tao, the Tauri plugins the app uses and the mobile toolchain (cargo-mobile2, Gradle, the Android Gradle plugin, Kotlin, androidx.webkit, Xcode) are the mid, checked every run, with the webview engines of the platforms the app ships; Tauri's other direct dependencies are the minor, on request ("minor" or "all"). Reads the versions from src-tauri/Cargo.lock and the generated mobile projects, searches the GitHub issues, releases, advisories and the unreleased change notes on dev, matches them against the app's code, and reports. Read-only. Use when asked to check Tauri, wry, tao, a Tauri plugin or the mobile build for bugs, issues, advisories or unreleased fixes.
---

# Tauri dependency check

Run from the root of an app with a `src-tauri/` folder. Read-only: it
reports and stops. Nothing is edited, upgraded, regenerated or filed.

The project's list, `.claude/deps.md` (`/deps-check`), is read first where
it exists: the plugins the app uses, its workarounds.

**`src-tauri` is its own workspace.** Its `Cargo.lock` is the one read here,
never the root's: its tokio, serde or url are not the site's.

## Arguments

- none: major, mid and engines.
- `minor`: minor only.
- `all`: major, mid, engines and minor.

## Tiers

| Tier | What | Repository |
|---|---|---|
| Major | `tauri`, `tauri-runtime`, `tauri-runtime-wry`, `tauri-utils`, `tauri-macros`, `tauri-codegen`, `tauri-build`, `tauri-plugin`; the installed `tauri-cli`, its bundler and its mobile templates | `tauri-apps/tauri` (one repo, one search) |
| Mid | `wry`, `tao` | `tauri-apps/wry`, `tauri-apps/tao` |
| Mid | each `tauri-plugin-*` the app uses (in `src-tauri/Cargo.toml`; the starter's: `tauri-plugin-opener`) | `tauri-apps/plugins-workspace`, label `plugin: <name>` |
| Mid | the mobile toolchain: `cargo-mobile2` (the CLI's), Gradle, the Android Gradle plugin, Kotlin, `androidx.webkit`, the SDK levels, Xcode, the iOS deployment target | `tauri-apps/cargo-mobile2`; Tauri's labels `platform: Android`, `platform: iOS` |
| Engines | WKWebView (macOS, iOS), Android System WebView, WebView2 (Windows), WebKitGTK (Linux), each only where the app ships | issues naming them in tauri and wry |
| Minor | every other direct dependency of `tauri` at the locked version, for the app's platforms only (step 1) | each crate's own |

Minor, by platform (tauri 2.12.5; the locked version's own list wins):

| Platform | Crates |
|---|---|
| every | `http`, `url`, `serde_json`, `serialize-to-javascript`, `tokio`, `mime`, `cookie`, `image`, `glob`, `dirs` |
| desktops | `muda`, `tray-icon` |
| macOS | `objc2-app-kit`, `objc2-web-kit`, `window-vibrancy`, `plist`, `embed_plist` |
| Windows | `webview2-com`, `windows`, `window-vibrancy` |
| Linux | `gtk`, `webkit2gtk` |
| iOS | `swift-rs`, `objc2-ui-kit`, `reqwest`, `rustls` |
| Android | `jni`, `reqwest`, `rustls` |

## Steps

### 1. Versions

Each crate's locked version:

```sh
awk -v n="wry" '$0=="name = \""n"\""{getline; gsub(/version = |"/,""); print; exit}' src-tauri/Cargo.lock
```

More than one version of a crate in the lock is a finding of its own:

```sh
grep -c '^name = "objc2"$' src-tauri/Cargo.lock
```

The installed CLI. It has its own number (tauri 2.12.5 ships with tauri-cli
2.12.4):

```sh
cargo install --list | grep -A1 '^tauri-cli'
```

The `cargo-mobile2` the CLI builds the phones with:

```sh
grep -A1 'dependencies.cargo-mobile2\]' ~/.cargo/registry/src/*/tauri-cli-<version>/Cargo.toml
```

The Android toolchain, from the generated project:

```sh
grep -rhoE 'com.android.tools.build:gradle:[0-9.]+|kotlin-gradle-plugin:[0-9.]+|gradle-[0-9.]+-bin|androidx.webkit:webkit:[0-9.]+|(compileSdk|minSdk|targetSdk) = [0-9]+' src-tauri/gen/android --include=*.kts --include=*.properties --include=*.gradle | sort -u
```

The iOS deployment target, and the Xcode in use:

```sh
grep -A1 deploymentTarget src-tauri/gen/apple/project.yml
```

```sh
xcodebuild -version
```

Tauri's own dependency list at the locked version, for the minor tier, with
the platform each is compiled for:

```sh
grep -oE '^\[(target\..+\.)?dependencies\.[a-zA-Z0-9_-]+\]' ~/.cargo/registry/src/*/tauri-<version>/Cargo.toml
```

### 2. Relevance

The app's platforms decide which issues can reach it:

- the desktop bundles: `bundle.targets` in `src-tauri/tauri.conf.json`
  (`app`, `dmg`: macOS; `appimage`: Linux; `nsis`, `msi`: Windows);
- the phones: `src-tauri/gen/android` and `src-tauri/gen/apple` present;
- what the scripts build: `cargo tauri ... build` in `src-tauri/*.sh`.

```sh
grep -n '"targets"\|"csp"\|"create"\|"url"\|"features"' src-tauri/tauri.conf.json src-tauri/Cargo.toml
```

```sh
grep -rn 'cargo tauri' src-tauri/*.sh
```

The app's Tauri profile. Count the uses:

```sh
for p in 'on_navigation' 'on_page_load' 'on_new_window' 'with_webview' 'jni_handle' 'from_config' 'mobile_entry_point' '.plugin(' 'invoke_handler' 'Emitter' 'tray' 'menu'; do printf '%-20s %s\n' "$p" "$(grep -rF "$p" src-tauri/src --include=*.rs | wc -l)"; done
```

The site's access to the device. No `capabilities/` folder means no command
is allowed to any page: an ACL or IPC issue reaches the app only where it
lets a page past that.

```sh
ls src-tauri/capabilities
```

Drop the issues on platforms, features and APIs the app does not use, in one
line each.

### 3. Repository

From the crate's own manifest, then the repository's current name:

```sh
grep -m1 '^repository' ~/.cargo/registry/src/*/tauri-plugin-opener-2.7.2/Cargo.toml
```

```sh
gh api repos/tauri-apps/plugins-workspace --jq .full_name
```

### 4. Upstream

The locked version's release date:

```sh
curl -s -A tauri-deps-check https://crates.io/api/v1/crates/tauri/2.12.5 | python3 -I -c "import json,sys;print(json.load(sys.stdin)['version']['created_at'])"
```

Releases. Each crate has its own tag (`tauri-v2.12.5`, `tauri-cli-v2.12.4`):

```sh
gh release list --repo tauri-apps/tauri --limit 12
```

Open issues on each platform the app ships, most recently active first:

```sh
gh issue list --repo tauri-apps/tauri --state open --label "platform: Android" --limit 60 --json number,title,updatedAt --jq '.[] | "\(.number) \(.updatedAt[0:10]) \(.title)"'
```

The labels: `platform: macOS`, `platform: iOS`, `platform: Android`,
`platform: Windows`, `platform: Linux`; `scope: core`, `scope: cli`,
`scope: bundler`.

Issues closed since the locked release:

```sh
gh issue list --repo tauri-apps/tauri --state closed --search "closed:>2026-10-21 reason:completed" --limit 60 --json number,title,closedAt --jq '.[] | "\(.number) \(.closedAt[0:10]) \(.title)"'
```

Narrow by the APIs from step 2:

```sh
gh issue list --repo tauri-apps/tauri --state all --search "on_page_load" --limit 20 --json number,title,state,updatedAt
```

The plugins, by their label:

```sh
gh issue list --repo tauri-apps/plugins-workspace --state open --label "plugin: opener" --limit 30 --json number,title,updatedAt --jq '.[] | "\(.number) \(.updatedAt[0:10]) \(.title)"'
```

wry and tao, narrowed the same way:

```sh
gh issue list --repo tauri-apps/wry --state open --search "navigation" --limit 20 --json number,title,updatedAt
```

Open any issue whose title might apply and read its body and comments before
judging it:

```sh
gh issue view 16379 --repo tauri-apps/tauri --json body,comments
```

Every issue the app's code or comments name:

```sh
grep -rnoE '(tauri-apps/[a-z-]+#|tauri#|wry#|tao#)[0-9]+' src-tauri/src src-tauri/*.sh docs 2>/dev/null | sort -u
```

### 5. Fixed, not yet released

Merged fixes waiting for the next release leave a note in `.changes/` on
`dev`, naming the crates they bump:

```sh
gh api "repos/tauri-apps/tauri/contents/.changes?ref=dev" --jq '.[].name'
```

```sh
gh api "repos/tauri-apps/tauri/contents/.changes/fix-linux-touch-resize-drag.md?ref=dev" --jq .content | base64 -d
```

### 6. Template drift

`gen/android` and `gen/apple` are written once by `cargo tauri android init`
and `ios init`, and never again. The installed CLI's templates show what a
fresh project would get:

```sh
grep -rhoE 'com.android.tools.build:gradle:[0-9.]+|kotlin-gradle-plugin:[0-9.]+|gradle-[0-9.]+-bin|androidx.webkit:webkit:[0-9.]+|(compileSdk|targetSdk) = [0-9]+' ~/.cargo/registry/src/*/tauri-cli-<version>/templates/mobile/android | sort -u
```

A value the template differs on is drift: name it, with the issue or release
note that changed it where one exists. `minSdk` and the iOS target are
`{{...}}` placeholders there, filled from the config.

### 7. Advisories

The app's own lock:

```sh
cargo audit --file src-tauri/Cargo.lock
```

The repositories' own advisories, which `cargo audit` may not list yet:

```sh
gh api repos/tauri-apps/tauri/security-advisories --jq '.[] | "\(.ghsa_id) \(.severity) \(.published_at[0:10]) \(.summary)"'
```

The same for `tauri-apps/wry` and `tauri-apps/plugins-workspace`. An
advisory counts only where the locked version is in its range and the app
reaches the affected part (step 2: a remote page, a capability, a plugin).

### 8. Match to the app

For each candidate issue, find the API, setting or platform it touches and
grep the app for it. Read the crate's source in `~/.cargo/registry` at the
locked version where the claim needs it. An issue counts only with evidence:
the file and line in the app that reaches it, or the config key that turns
it on.

The workarounds in `src-tauri/src/lib.rs` and the generated projects name
what they cover in their comments. Read them and match each to its upstream
issue. Those of the starter's app shell:

- the opener's link script off (`open_js_links_on_click(false)`);
- the status bar painted through `with_webview` and `jni_handle().exec`.

## Report

One table per tier:

| Crate (version) | Issue | Affects the app? | Evidence | Whose |
|---|---|---|---|---|

- **Affects the app?:** yes, no, or not checked, with the reason.
- **Evidence:** a markdown link to the app's file and line, or the config key.
- **Whose:** Tauri, wry or tao, the plugin, the engine (Apple, Google,
  Microsoft, WebKitGTK), the toolchain (Gradle, Kotlin, Xcode), or the app
  (its own glue or use).

Then:

- **Fixed upstream, not yet in the app:** the release that has it, or only
  the `.changes` note on `dev` (step 5).
- **Workarounds already in the app:** the code that covers an open issue, and
  the release that lets it go.
- **Template drift:** what a fresh `gen/` would differ in (step 6).
- **Not checked:** what was skipped and why.

Rules:

- Report only. Never edit, upgrade, regenerate `gen/`, comment or file an
  issue.
- Quote an issue only after reading it.
- Say plainly what is unverified; nothing here is run on a device.
- A bug inside a crate belongs to that crate; a webview engine's bug to its
  vendor, even when it reaches the app through Tauri or wry.
