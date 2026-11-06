#!/usr/bin/env bash
# The app built for a site of the run's, never the config's own: the
# scripts that run the app on this Mac's devices (devices.sh, for
# screens.sh and links.sh) and the browser tests' app lane (e2e/run.sh)
# each build it for the site they test. As an islands app on this stack
# builds its own.
#
#   src-tauri/app-for.sh config apple|android <address>   the merged config's path
#   src-tauri/app-for.sh android <address>                a debug APK, its path
#   src-tauri/app-for.sh ios <address>                    the Simulators' debug app, its path
#
# The site in an app: a copy of the start page with its SITE, and
# tauri.conf.json's CSP with its connect-src, both merged over the config
# for this build alone (the CLI's `--config`), while offline/index.html and
# tauri.conf.json stay as they are. Prints the path on standard output and
# nothing else; the build's own output goes to a log of its own, shown only
# when it fails.
set -euo pipefail

usage="usage: src-tauri/app-for.sh config apple|android <address> | android <address> | ios <address>"
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
cd "$here"

# The merged config for `platform` and `address`, written under target/;
# its path printed.
site_config() {
  local platform=$1 address=$2
  local start="$here/target/app-start-$platform" config="$here/target/app-config-$platform.json"
  mkdir -p "$here/target"
  rm -rf "$start"
  cp -R offline "$start"
  sed -i '' -E "s#const SITE = \"[^\"]*\";#const SITE = \"$address\";#" "$start/index.html"
  grep -q "const SITE = \"$address\";" "$start/index.html" || {
    echo "the start page's SITE not found in offline/index.html" >&2
    exit 1
  }
  python3 -I -c '
import json, re, sys
conf, start, site, out = sys.argv[1:]
csp = json.load(open(conf))["app"]["security"]["csp"]
csp = re.sub(r"connect-src [^;]+", "connect-src " + site, csp)
json.dump({"build": {"frontendDist": start}, "app": {"security": {"csp": csp}}}, open(out, "w"))
' tauri.conf.json "$start" "$address" "$config"
  echo "$config"
}

# `$@` run with its output in `log`, the log shown when it fails.
built() {
  local log=$1
  shift
  if ! "$@" >"$log" 2>&1; then
    cat "$log" >&2
    echo "the build failed: $* (its output above, and in $log)" >&2
    exit 1
  fi
}

(($# == 2 || $# == 3)) || { echo "$usage" >&2; exit 2; }
case $1 in
  config)
    (($# == 3)) && [[ $2 == apple || $2 == android ]] || { echo "$usage" >&2; exit 2; }
    site_config "$2" "$3"
    ;;
  # The iOS app for the Simulators of Apple silicon, through the Tauri CLI:
  # the Xcode project's build phase asks the CLI for the Rust side, so
  # xcodebuild alone fails ("the `tauri ios dev` or `tauri ios build`
  # command must be running").
  ios)
    (($# == 2)) || { echo "$usage" >&2; exit 2; }
    config=$(site_config apple "$2")
    echo "== building the iOS app for $2" >&2
    built "$here/target/app-build-ios.log" cargo tauri ios build --debug --target aarch64-sim --config "$config"
    name=$(python3 -I -c 'import json, sys; print(json.load(open(sys.argv[1]))["productName"])' tauri.conf.json)
    app="$here/gen/apple/build/arm64-sim/$name.app"
    [[ -d $app ]] || { echo "no app at $app" >&2; exit 1; }
    echo "$app"
    ;;
  # The Android app, a debug APK for the emulators' arm64, built with Java
  # 21: the project's Gradle (9.6.1) runs on Java up to 26, and a Mac's
  # default Java 27 failed it ("Unsupported class file major version 71",
  # an islands app on this stack, 2026-10-29). A JAVA_HOME already set is
  # kept. A debug build, whose webview a driver can attach to (wry turns
  # its debugging on in debug builds alone).
  android)
    (($# == 2)) || { echo "$usage" >&2; exit 2; }
    config=$(site_config android "$2")
    android_sdk=${ANDROID_HOME:-/opt/homebrew/share/android-commandlinetools}
    export ANDROID_HOME=$android_sdk
    export NDK_HOME=${NDK_HOME:-$(find "$android_sdk/ndk" -maxdepth 1 -mindepth 1 -type d | sort | tail -1)}
    JAVA_HOME=${JAVA_HOME:-$(/usr/libexec/java_home -v 21 2>/dev/null || true)}
    export JAVA_HOME
    echo "== building the Android app for $2" >&2
    built "$here/target/app-build-android.log" cargo tauri android build --debug --apk --target aarch64 --config "$config"
    apk=$(find "$here/gen/android/app/build/outputs/apk" -name '*-debug.apk' -print0 | xargs -0 ls -t | head -1)
    [[ -f $apk ]] || { echo "no APK under gen/android/app/build/outputs/apk" >&2; exit 1; }
    echo "$apk"
    ;;
  *)
    echo "$usage" >&2
    exit 2
    ;;
esac
