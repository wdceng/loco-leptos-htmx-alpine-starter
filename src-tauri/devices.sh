# What the scripts that run the app on this Mac's devices share (screens.sh,
# the pictures; links.sh, the links that leave the app): the site the app
# opens, the app built for it, an emulator started with the app opened on
# the site, and the devices this run started shut down again. Sourced,
# never run: the script sets `site` (empty for the dev loop's) and `usage`
# first, and `set -euo pipefail`. As an islands app on this stack splits it.
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
cd "$here"

# The site: the dev loop's, the development config's own address
# (config/development.yaml, `host`), unless the script was given one.
site=${site:-$(awk '/^  host:/ { print $2; exit }' "$here/../config/development.yaml")}
site=${site%/}
# The site's host and port, which the emulators' webview is asked for, and
# a `localhost` an emulator reaches through adb (emulator_up).
site_host=${site#*://}
site_host=${site_host%%/*}
case $site_host in
  *:*) site_port=${site_host##*:} ;;
  *) [[ $site == https://* ]] && site_port=443 || site_port=80 ;;
esac

# The Android emulator resolves no `.local` name: it asks the Mac's DNS
# servers through its own network, and they know nothing of multicast DNS
# (another Mac's watch loop out of reach of the emulators, the Simulators
# reaching it, in an islands app on this stack, 2026-10-29). Its app gets
# the address this Mac resolves the name to; iOS keeps the name, App
# Transport Security refusing a bare IP.
android_site=$site
android_host=$site_host
host_name=${site_host%%:*}
if [[ $host_name == *.local ]]; then
  ip=$(python3 -I -c 'import socket, sys; print(socket.gethostbyname(sys.argv[1]))' "$host_name" 2>/dev/null || true)
  if [[ -z $ip ]]; then
    echo "$host_name does not resolve on this Mac" >&2
    exit 1
  fi
  android_site=${site/$host_name/$ip}
  android_host=${site_host/$host_name/$ip}
fi

# The site answering before anything is built: its own `ok` at /up
# (src/controllers/up.rs), at Android's address too where it is another.
# On another Mac than the dev loop's, `localhost` has no site: give that
# Mac's address (docs/testing.md).
for address in "$site" "$android_site"; do
  if [[ $(curl -s --max-time 5 "$address/up" || true) != ok ]]; then
    echo "$address/up does not answer ok: is the dev loop running? ($usage)" >&2
    exit 1
  fi
done
echo "== the site: $site"
[[ $android_site == "$site" ]] || echo "== the site on Android: $android_site"

wait_for_site=${SCREENS_WAIT:-60}

# The app's id and name, tauri.conf.json's alone.
conf() {
  python3 -I -c 'import json, sys; print(json.load(open(sys.argv[1]))[sys.argv[2]])' tauri.conf.json "$1"
}
id=$(conf identifier)
name=$(conf productName)

android_sdk=${ANDROID_HOME:-/opt/homebrew/share/android-commandlinetools}
adb="$android_sdk/platform-tools/adb"
export ANDROID_HOME=$android_sdk
export NDK_HOME=${NDK_HOME:-$(find "$android_sdk/ndk" -maxdepth 1 -mindepth 1 -type d | sort | tail -1)}
avds=$("$android_sdk/emulator/emulator" -list-avds)

# The device this run booted and has not shut down yet, shut down whatever
# happens: a Simulator's udid, an emulator's serial or the Mac's app's
# process. An emulator started but not yet listed by adb has its process
# alone (`own_emulator_pid`), killed so a skipped start leaves none behind.
own_sim=
own_emulator=
own_emulator_pid=
own_mac=
shut_down() {
  # The app quit first and a moment left before the Simulator goes: shut
  # down at once, its home screen (SpringBoard) aborted mid-shutdown and
  # macOS reported the crash (an iPad Air, in an islands app on this stack,
  # 2026-10-29).
  if [[ -n $own_sim ]]; then
    xcrun simctl terminate "$own_sim" "$id" 2>/dev/null || true
    sleep 2
    xcrun simctl shutdown "$own_sim" 2>/dev/null || true
  fi
  [[ -n $own_emulator ]] && "$adb" -s "$own_emulator" emu kill >/dev/null 2>&1
  [[ -n $own_emulator_pid ]] && { kill "$own_emulator_pid" 2>/dev/null || true; }
  [[ -n $own_mac ]] && kill "$own_mac" 2>/dev/null
  own_sim=
  own_emulator=
  own_emulator_pid=
  own_mac=
  return 0
}
trap shut_down EXIT

# A Simulator's "udid state" by its name, on the newest runtime that has one.
simulator() {
  xcrun simctl list devices available -j | python3 -I -c '
import json, sys
found = []
for runtime, devices in json.load(sys.stdin)["devices"].items():
    version = tuple(int(n) for n in runtime.rsplit("iOS-", 1)[-1].split("-") if n.isdigit())
    found += [(version, d["udid"], d["state"]) for d in devices if d["name"] == sys.argv[1]]
if found:
    print(*max(found)[1:])' "$1"
}

# The serial of an emulator that is up, by its AVD's name; empty when none.
emulator_serial() {
  local serial
  for serial in $("$adb" devices | awk 'NR > 1 && $2 == "device" { print $1 }'); do
    if [[ $("$adb" -s "$serial" emu avd name 2>/dev/null | head -1 | tr -d '\r') == "$1" ]]; then
      echo "$serial"
      return 0
    fi
  done
}

# The builds' own output in a log of its own, shown only when a build fails.
build_log="$here/target/app-build.log"
built() {
  if ! "$@" >>"$build_log" 2>&1; then
    cat "$build_log" >&2
    echo "the build failed: $* (its output above, and in $build_log)" >&2
    exit 1
  fi
}
mkdir -p "$here/target"
: >"$build_log"

# The run's site in an app: a copy of the start page with its SITE, and
# tauri.conf.json's CSP with its connect-src, both merged over the config
# for these builds alone (the CLI's `--config`); one for Apple's (the
# Simulators and the Mac) and one for Android's, their addresses differing
# for a `.local` name (above). Prints the config's path.
site_config() {
  local address=$1 platform=$2
  local start="$here/target/app-start-$platform" config="$here/target/app-config-$platform.json"
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
config=$(site_config "$site" apple)
android_config=$(site_config "$android_site" android)

# The iOS app for the Simulators of Apple silicon, through the Tauri CLI:
# the Xcode project's build phase asks the CLI for the Rust side, so
# xcodebuild alone fails ("the `tauri ios dev` or `tauri ios build` command
# must be running"). Sets `app`.
build_ios() {
  echo "== building the iOS app"
  built cargo tauri ios build --debug --target aarch64-sim --config "$config"
  app="gen/apple/build/arm64-sim/$name.app"
  [[ -d $app ]] || { echo "no app at $app" >&2; exit 1; }
}

# The Android app, a debug APK for the emulators' arm64, built with Java 21:
# the project's Gradle (9.6.1) runs on Java up to 26, and a Mac's default
# Java 27 failed it ("Unsupported class file major version 71", an islands
# app on this stack, 2026-10-29). A JAVA_HOME already set is kept. Sets
# `apk`.
build_android() {
  echo "== building the Android app"
  JAVA_HOME=${JAVA_HOME:-$(/usr/libexec/java_home -v 21 2>/dev/null || true)}
  export JAVA_HOME
  built cargo tauri android build --debug --apk --target aarch64 --config "$android_config"
  apk=$(find gen/android/app/build/outputs/apk -name '*-debug.apk' -print0 | xargs -0 ls -t | head -1)
  [[ -f $apk ]] || { echo "no APK under gen/android/app/build/outputs/apk" >&2; exit 1; }
}

# Waits until the emulator's webview shows the site, asked over the
# webview's debugging socket, whose /json lists the page, its address and
# its title. Cloudflare's challenge on the site's host ("Just a moment...")
# is not the site: a fresh emulator meets its "Verify you are human" box,
# which only a person passes (an islands app on this stack, 2026-10-29), so
# a device still on it at the end of the wait is skipped (return 2) with
# how to pass it once, its pass then kept as a cookie in the emulator.
site_on_emulator() {
  local device=$1 serial=$2 socket= port
  # `|| true`: grep's miss, before the webview is up, would end the script.
  for _ in $(seq 1 "$wait_for_site"); do
    socket=$("$adb" -s "$serial" shell cat /proc/net/unix | grep -oE 'webview_devtools_remote_[0-9]+' | head -1 || true)
    [[ -n $socket ]] && break
    sleep 1
  done
  if [[ -z $socket ]]; then
    echo "   $device: no webview to ask, gone on as it is"
    return 0
  fi
  port=$("$adb" -s "$serial" forward tcp:0 "localabstract:$socket")
  local page=
  for _ in $(seq 1 "$wait_for_site"); do
    page=$(curl -s "localhost:$port/json" | python3 -I -c '
import json, sys
for p in json.load(sys.stdin):
    if p.get("type") == "page" and sys.argv[1] in p.get("url", ""):
        print("challenge" if p.get("title", "").startswith("Just a moment") else "site")
        break' "$android_host" 2>/dev/null || true)
    [[ $page == site ]] && break
    sleep 1
  done
  "$adb" -s "$serial" forward --remove "tcp:$port"
  case $page in
    site) return 0 ;;
    challenge)
      echo "   $device: still on Cloudflare's challenge, skipped. Pass it once by hand:"
      echo "   $android_sdk/emulator/emulator -avd $device -gpu swiftshader_indirect   # open the app, tick the box, close the emulator"
      return 2
      ;;
    *) echo "   $device: the site not seen within ${wait_for_site}s, gone on as it is" ;;
  esac
}

# An emulator up with the app open on the site: started if off, with no
# window and the software GPU (macOS 27 draws no screen without it,
# docs/testing.md), the app installed and opened, the site waited for.
# Sets `serial`; returns 1 where the device is skipped (not on this Mac,
# would not start or install, still on Cloudflare's challenge), having said
# why.
emulator_up() {
  local avd=$1 seen=0
  serial=
  if ! grep -qx "$avd" <<<"$avds"; then
    echo "== $avd: not on this Mac, skipped"
    return 1
  fi
  echo "== $avd"
  serial=$(emulator_serial "$avd")
  if [[ -z $serial ]]; then
    "$android_sdk/emulator/emulator" -avd "$avd" -gpu swiftshader_indirect -no-window -no-audio >/dev/null 2>&1 &
    own_emulator_pid=$!
    for _ in $(seq 1 60); do
      serial=$(emulator_serial "$avd")
      [[ -n $serial && $("$adb" -s "$serial" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r') == 1 ]] && break
      sleep 3
    done
    if [[ -z $serial ]]; then
      echo "   $avd: did not start, skipped"
      shut_down
      return 1
    fi
    own_emulator=$serial
    # Awake for the run: a long wait let a tablet's screen go to sleep, its
    # last picture black but the battery (an islands app on this stack,
    # 2026-10-29). Only an emulator this run started, another project's
    # left as it is.
    "$adb" -s "$serial" shell svc power stayon true
    "$adb" -s "$serial" shell input keyevent KEYCODE_WAKEUP
  else
    # adb lists an emulator as a device before Android has finished
    # starting, one another project has just started too.
    until [[ $("$adb" -s "$serial" shell getprop sys.boot_completed | tr -d '\r') == 1 ]]; do
      sleep 2
    done
  fi
  # A fresh emulator says it has booted a moment before its package manager
  # takes an install (`adb install` failing with no reason on a first boot,
  # in an islands app on this stack, 2026-10-29): its answer waited for,
  # then the install tried up to five times.
  for _ in $(seq 1 30); do
    "$adb" -s "$serial" shell pm path android 2>/dev/null | grep -q '^package:' && break
    sleep 2
  done
  for try in 1 2 3 4 5; do
    "$adb" -s "$serial" install -r "$apk" >/dev/null 2>&1 && break
    if ((try == 5)); then
      echo "   $avd: the app would not install, skipped"
      stop_own_emulator "$avd"
      return 1
    fi
    sleep 5
  done
  # The Mac's own site: `localhost` on an emulator is the emulator, so the
  # site's port is carried over to the Mac's.
  if [[ $android_host == localhost* || $android_host == 127.0.0.1* ]]; then
    "$adb" -s "$serial" reverse "tcp:$site_port" "tcp:$site_port" >/dev/null
  fi
  "$adb" -s "$serial" shell am force-stop "$id"
  "$adb" -s "$serial" shell am start -n "$id/.MainActivity" >/dev/null
  site_on_emulator "$avd" "$serial" || seen=$?
  if ((seen == 2)); then
    stop_own_emulator "$avd"
    return 1
  fi
}

# The emulator stopped if this run started it, and gone before the next.
stop_own_emulator() {
  [[ -n $own_emulator ]] || return 0
  shut_down
  for _ in $(seq 1 20); do
    [[ -z $(emulator_serial "$1") ]] && break
    sleep 2
  done
}
