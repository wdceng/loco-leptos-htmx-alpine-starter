# Prerequisites

It's all Rust to build and run, no Node.js. Tailwind, wasm-bindgen and
wasm-opt are standalone binaries that cargo-leptos downloads on first use.
Node 24+ runs one check only, the stylesheet's lint (`npm install` once,
then `npm run lint`).

Install commands are each tool's own. Your package manager works too.

## 1. Required

| # | Tool | Purpose | Install |
|-----|------|---------|---------|
| 1.1 | **C toolchain** | Linker and system libraries for the native build | Xcode Command Line Tools on macOS (`xcode-select --install`), `build-essential` on Debian/Ubuntu, Visual Studio Build Tools on Windows |
| 1.2 | **Rust**, stable 1.94+ | Compiles both halves. Loco 1.2 and Sea-ORM 2.0.4 need 1.94+ | https://rustup.rs |
| 1.3 | **wasm32 target** | Compiles the islands bundle | `rustup target add wasm32-unknown-unknown` |
| 1.4 | **cargo-leptos**, patched | Builds both halves, runs Tailwind and the live-reload dev loop. 0.3.11 plus the two split fixes lazy islands need, the same build as CI (`development.md`, "Lazy islands"). About 3 minutes; once per machine, and again when `tools/cargo-leptos/` changes | `tools/cargo-leptos/install.sh` |

`clippy` and `rustfmt` come with Rust.

## 2. Optional Cargo Tools

| # | Tool | Purpose | Install |
|-----|------|---------|---------|
| 2.1 | **cargo-watch** | Only for `cargo loco watch`, which reloads just the server. The usual loop is `cargo leptos watch --split -- start` | `cargo install --locked cargo-watch` |
| 2.2 | **cargo-audit** | Scans `Cargo.lock` for known vulnerabilities. CI runs 0.22.2. `.cargo/audit.toml` lists ignored advisories | `cargo install --locked cargo-audit@0.22.2` |
| 2.3 | **cargo-zigbuild**, with Zig and the Linux target | Builds the Linux x86_64 server binary natively, Zig as its linker (`dev-server.md`, `staging.md`, `production.md`) | `brew install zig` (or https://ziglang.org/download/), then `cargo install --locked cargo-zigbuild && rustup target add x86_64-unknown-linux-gnu` |
| 2.4 | **cargo-binstall** | Installs prebuilt cargo tools instead of compiling them. CI uses it for cargo-audit | `cargo install --locked cargo-binstall` |
| 2.5 | **loco** | The Loco CLI, only for `loco new` and generators. Not needed here: `cargo loco` is a cargo alias in `.cargo/config.toml` that runs the app binary | `cargo install --locked loco` |
| 2.6 | **cargo-nextest** | One process per test (`cargo nextest run`). `.config/nextest.toml` keeps app-booting tests on one thread | `cargo install --locked cargo-nextest` |
| 2.7 | **cargo-sweep** | Clears old native, wasm32 and Linux builds out of `target/` | `cargo install --locked cargo-sweep` |
| 2.8 | **sea-orm-cli** | Only for `cargo loco db entities`, which regenerates `src/models/_entities/`. `cargo loco doctor` flags it if missing | `cargo install --locked sea-orm-cli` |
| 2.9 | **sqlx-cli** | `cargo sqlx prepare` refreshes the `.sqlx/` query cache after a migration or a new query (`development.md`). Same version as the `sqlx` crate, 0.9.0 | `cargo install --locked sqlx-cli@0.9.0 --no-default-features --features sqlite` |
| 2.10 | **cargo-llvm-cov** | Each file's test coverage (`cargo llvm-cov nextest --summary-only`) | `rustup component add llvm-tools-preview && cargo install --locked cargo-llvm-cov` |

## 3. Local Services

| # | Tool | Purpose | Install |
|-----|------|---------|---------|
| 3.1 | **SMTP catcher** | `config/development.yaml` sends mail to `localhost:1025`. Registration and password reset need something listening. Mailpit, MailHog and maildev all default to 1025, with a web inbox on 8025 | `brew install mailpit && brew services start mailpit` on a Mac. Elsewhere `docker run -d -p 1025:1025 -p 8025:8025 axllent/mailpit`, or a native build from https://github.com/axllent/mailpit. Or set `stub: true` under `mailer:` to keep mail in memory |
| 3.2 | **sqlite3** | Inspect the database files (`app_<env>.sqlite`). `.backup` makes a consistent copy while the app runs | Package manager, or https://sqlite.org/download.html |

## 4. Downloaded by cargo-leptos

Nothing to install. They land in `~/.cache/cargo-leptos` on the first build.

| # | Tool | Purpose | Pin a version |
|-----|------|---------|---------------|
| 4.1 | **tailwindcss** | Compiles `style/tailwind.css` | `LEPTOS_TAILWIND_VERSION=v4.x.y` |
| 4.2 | **wasm-bindgen** | JS glue for the wasm bundle | matched to the crate in `Cargo.lock` |
| 4.3 | **wasm-opt** | Shrinks the release wasm | `LEPTOS_WASM_OPT_VERSION=version_NNN` |

## 5. On a Server Only

| # | Tool | Purpose |
|-----|------|---------|
| 5.1 | **Caddy** | Reverse proxy with automatic HTTPS in front of Loco (`staging.md`, `production.md`). Not needed locally |

## 6. Editor (VS Code)

| # | Extension | Purpose |
|-----|-----------|---------|
| 6.1 | **rust-analyzer** | Rust language support |
| 6.2 | **Tailwind CSS IntelliSense** (`bradlc.vscode-tailwindcss`) | Class completion, hover previews and its lint inside `view!`, and the stylesheet in its v4 mode. `.vscode/settings.json` sets both up |

`.vscode/extensions.json` recommends both, so VS Code offers them when you
open the folder.

## Browsers

The browser tests (`e2e/run.sh`, `docs/testing.md`) drive Chrome, Edge,
Firefox and Safari through their drivers, with cargo-nextest (2.6) and
bash 4.3 or newer (`brew install bash`; macOS's own is 3.2).

```bash
V=$("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --version | awk '{print $3}') && mkdir -p ~/Library/Caches/chromedriver && curl -fsSL "https://storage.googleapis.com/chrome-for-testing-public/$V/mac-arm64/chromedriver-mac-arm64.zip" -o /tmp/chromedriver.zip && unzip -o -q /tmp/chromedriver.zip -d ~/Library/Caches/chromedriver   # Chrome's, matching it (again after a Chrome update)
brew install geckodriver        # Firefox
sudo safaridriver --enable      # Safari, once; then Safari, Settings, Developer: "Allow remote automation"
V=$(defaults read "/Applications/Microsoft Edge.app/Contents/Info.plist" CFBundleShortVersionString) && D=$(mktemp -d) && curl -sfL -o "$D/e.zip" "https://msedgedriver.microsoft.com/$V/edgedriver_mac64_m1.zip" && unzip -oq "$D/e.zip" -d "$D" && mv "$D/msedgedriver" /opt/homebrew/bin/ && xattr -d com.apple.quarantine /opt/homebrew/bin/msedgedriver 2>/dev/null; msedgedriver --version   # Edge's, matching it (again after an Edge update)
```

- **A driver that no longer matches its browser** fails the session with
  "only supports … version": fetch it again.
- **Firefox needs Full Disk Access for the app the run starts from**
  (Terminal, or VS Code): System Settings, Privacy & Security, then a
  restart of that app. Without it: "Could not find profile folder".
- The low bar's Chrome 114 and Firefox 128: `docs/testing.md`, "The Low
  Bar".

## 7. The App

Only for `src-tauri/`. macOS and Windows:

```bash
cargo install --locked tauri-cli --version 2.12.4   # `cargo tauri …`
brew install llvm lld                               # the Windows compiler and linker; /opt/homebrew/opt/llvm/bin goes first on PATH for the build
rustup target add x86_64-pc-windows-msvc            # the Windows target
cargo install --locked cargo-xwin                   # the Windows SDK, fetched on the first Windows build (about 1 GB)
```

Linux, built inside Docker (`src-tauri/linux.sh`), from any Mac:

```bash
brew install --cask docker   # Docker Desktop; start it before the build
```

iOS and iPadOS, with Xcode from the App Store:

```bash
sudo xcode-select -s /Applications/Xcode.app && sudo xcodebuild -license accept && sudo xcodebuild -runFirstLaunch   # Xcode's system parts, once and after every Xcode update
xcodebuild -downloadPlatform iOS                                            # the iOS Simulator (about 8 GB)
brew install xcodegen cocoapods ios-deploy libimobiledevice                 # the Xcode project, pods, a real device over USB
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios  # devices, the Simulator on Apple Silicon and on Intel
```

- **After every Xcode update, `-runFirstLaunch` again.** Without Xcode's
  new system parts, `-downloadPlatform` fails on a missing `CoreSimulator`.
- **Xcode 27 has no Simulator app:** its window is DeviceHub
  (`/Applications/Xcode.app/Contents/Applications/DeviceHub.app`).

The iPhone Duo, Apple's foldable, is only on iOS 27.1:

```bash
xcodebuild -downloadPlatform iOS -buildVersion 27.1   # iOS 27.1 beside 27.2 (about 8 GB)
xcrun simctl create "iPhone Duo" com.apple.CoreSimulator.SimDeviceType.iPhone-Duo com.apple.CoreSimulator.SimRuntime.iOS-27-1   # the Duo, once
```

Android, with the SDK from `brew install --cask android-commandlinetools`:

```bash
brew install --cask temurin@21                                                 # the Java Gradle runs on
sdkmanager "platforms;android-37.0" "build-tools;37.0.0" "ndk;30.0.16248370"   # the NDK is about 1 GB
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

The emulators `testing.md` names ("The app on the phones"):

```bash
yes | sdkmanager --licenses && sdkmanager "platform-tools" "emulator" "system-images;android-37.0;google_apis_playstore;arm64-v8a"   # adb, the emulator, an Android 17 image
echo no | avdmanager create avd -n android-phone -k "system-images;android-37.0;google_apis_playstore;arm64-v8a" -d pixel_10         # a Pixel 10, once (its profile from cmdline-tools 23.0)
echo no | avdmanager create avd -n android-tablet -k "system-images;android-37.0;google_apis_playstore;arm64-v8a" -d pixel_tablet   # a Pixel Tablet, once
sed -i '' -e 's/^hw.lcd.width=.*/hw.lcd.width=540/' -e 's/^hw.lcd.height=.*/hw.lcd.height=1212/' -e 's/^hw.lcd.density=.*/hw.lcd.density=210/' ~/.android/avd/android-phone.avd/config.ini    # the phone at half resolution, the same 412×923 CSS layout
sed -i '' -e 's/^hw.ramSize=.*/hw.ramSize=4G/' -e 's/^hw.lcd.width=.*/hw.lcd.width=1920/' -e 's/^hw.lcd.height=.*/hw.lcd.height=1200/' -e 's/^hw.lcd.density=.*/hw.lcd.density=240/' ~/.android/avd/android-tablet.avd/config.ini   # the tablet at 1.5× with 4 GB, the same 1280×800 CSS layout
echo no | avdmanager create avd -n android-phone-full -k "system-images;android-37.0;google_apis_playstore;arm64-v8a" -d pixel_10 && echo no | avdmanager create avd -n android-tablet-full -k "system-images;android-37.0;google_apis_playstore;arm64-v8a" -d pixel_tablet && sed -i '' 's/^hw.ramSize=.*/hw.ramSize=4G/' ~/.android/avd/android-tablet-full.avd/config.ini   # the app's pictures' own (src-tauri/screens.sh), at full density so a camera cutout is its real size
```

Then in the shell's profile (`~/.bash_profile` for bash, `~/.zshrc` for
zsh), and a new terminal:

```bash
export ANDROID_HOME=/opt/homebrew/share/android-commandlinetools   # where Tauri finds the SDK
export NDK_HOME=$ANDROID_HOME/ndk/30.0.16248370                    # and the NDK
export JAVA_HOME=$(/usr/libexec/java_home -v 21)                   # Gradle's Java
export LANG=en_US.UTF-8                                            # CocoaPods' encoding
```

- **Java 21, not the newest:** Gradle runs on `JAVA_HOME`. Temurin 27 is
  untested with Tauri's Gradle 9.6.1.
- **`LANG`:** without a UTF-8 terminal, CocoaPods warns "CocoaPods
  requires your terminal to be using UTF-8 encoding".
- **The first Android build fetches build-tools and platform 36 itself.**
  Nothing to install for them.
- **The emulators are shared with the other apps on this stack,** so
  they're set up one way: `android-phone` and `android-tablet` at half
  density for the browser tests, the `-full` pair for the pictures.

## Verify

You're set if this prints the target and "both fixes in":

```bash
rustup target list --installed | grep wasm32 && tools/cargo-leptos/check.sh
```

## Updates

```bash
rustup update                    # Rust, clippy, rustfmt
tools/cargo-leptos/install.sh    # after tools/cargo-leptos/ changed
```

Once both split fixes are released, cargo-leptos goes back to
`cargo install --locked cargo-leptos@<version>`, with the same version in
`.github/workflows/ci.yaml`.
