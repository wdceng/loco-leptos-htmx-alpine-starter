#!/usr/bin/env bash
# The app on the eight devices below and on this Mac (on_mac, last; Windows
# left out, no machine here runs it), one device at a time: the phones'
# apps built first, then each device booted if it is off, the app
# installed, opened and
# photographed into src-tauri/target/screens/, and the device shut down
# again if this run booted it, before the next one starts (all booted at
# once took a Mac's load to 115, in an islands app on this stack,
# 2026-10-29). Each picture named by its device and how it stood
# (docs/testing.md, The app on the phones): <device>-upright.png and
# <device>-sideways.png at the page's top, and on an emulator
# <device>-bottom.png at the page's foot. The folder emptied first; a device
# this Mac does not have skipped; one another project left running used as
# it is and left running. An emulator is driven by adb, which turns and
# swipes; a Simulator by simctl, which has no command for either (driving
# the Simulator app through System Events lost its device windows, in an
# islands app on this stack, 2026-10-29), so a Simulator gets its upright
# picture alone, unless the run is asked for its turn (`--turn`): then after
# the first picture it waits for that Simulator to be turned by hand (⌘← in
# DeviceHub) and takes a second. A picture is taken once the site is there
# (the Simulator's WebKit log, the emulator's webview asked over its
# debugging socket) and, on an emulator, once the screen has stopped
# changing (the webview draws a while after a turn or a swipe, white
# meanwhile); SCREENS_WAIT the longest wait for the site in seconds, 60 by
# default. The site the app opens is the dev loop's, the development
# config's own address (config/development.yaml, `host`), unless one is
# given (`--https://example.com`); the app built with it for this run
# alone, its start page and tauri.conf.json left as they are.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
cd "$here"
usage="usage: src-tauri/screens.sh [--turn] [--http(s)://address]"

# `--turn`: each Simulator's sideways picture too, after a turn by hand.
# `--<address>`: the site, one a run.
turn=
site=
for option in "$@"; do
  case $option in
    --turn) turn=yes ;;
    --http://* | --https://*)
      if [[ -n $site ]]; then
        echo "one address a run; $usage" >&2
        exit 2
      fi
      site=${option#--}
      ;;
    *)
      echo "$usage" >&2
      exit 2
      ;;
  esac
done
if [[ -n $turn && ! -t 0 ]]; then
  echo "--turn asks in a terminal: run it in one" >&2
  exit 2
fi
site=${site:-$(awk '/^  host:/ { print $2; exit }' "$here/../config/development.yaml")}
site=${site%/}
# The site's host and port, which the emulators' webview is asked for, and
# a `localhost` an emulator reaches through adb (on_emulator).
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

out="$here/target/screens"
rm -rf "$out"
mkdir -p "$out"
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

# A picture's name from a device's: "iPad Pro 13-inch (M5)" is ipad-pro-13-inch-m5.
slug() { tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9]+/-/g; s/^-+|-+$//g'; }

# The devices, by the names their Simulators and emulators have
# (docs/prerequisites.md). The emulators are the pictures' own, at their
# devices' full density, so a camera cutout, set in the real screen's
# pixels, comes out its real size; the shared `android-phone` and
# `android-tablet` run at half density and double it (docs/testing.md, The
# app on the phones).
simulators=("iPhone 18 Pro" "iPhone 17e" "iPad Pro 13-inch (M5)" "iPad Air 11-inch (M4)" "iPad mini (A17 Pro)" "iPhone Duo")
emulators=(android-phone-full android-tablet-full)

# The device this run booted and has not shut down yet, shut down whatever
# happens: a Simulator's udid, an emulator's serial or the Mac's app's
# process.
own_sim=
own_emulator=
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
  [[ -n $own_mac ]] && kill "$own_mac" 2>/dev/null
  own_sim=
  own_emulator=
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

# Which of the devices this Mac has, so a platform with none is not built.
has_simulators=
for device in "${simulators[@]}"; do
  [[ -n $(simulator "$device") ]] && has_simulators=yes
done
avds=$("$android_sdk/emulator/emulator" -list-avds)
has_emulators=
for avd in "${emulators[@]}"; do
  grep -qx "$avd" <<<"$avds" && has_emulators=yes
done
# The Mac's own app, but not over ssh: a picture of a window needs the
# Screen Recording permission, which a session without a screen cannot be
# asked for.
has_mac=
if [[ -z ${SSH_CONNECTION:-} ]]; then
  has_mac=yes
else
  echo "== the Mac's app: skipped over ssh, no Screen Recording permission to ask for"
fi
if [[ -z $has_simulators && -z $has_emulators && -z $has_mac ]]; then
  echo "none of the devices is on this Mac (docs/testing.md, The app on the phones)" >&2
  exit 1
fi

# The builds' own output in a log of its own, shown only when a build fails.
build_log="$here/target/screens-build.log"
built() {
  if ! "$@" >>"$build_log" 2>&1; then
    cat "$build_log" >&2
    echo "the build failed: $* (its output above, and in $build_log)" >&2
    exit 1
  fi
}
: >"$build_log"

# The run's site in an app: a copy of the start page with its SITE, and
# tauri.conf.json's CSP with its connect-src, both merged over the config
# for these builds alone (the CLI's `--config`); one for Apple's (the
# Simulators and the Mac) and one for Android's, their addresses differing
# for a `.local` name (above). Prints the config's path.
site_config() {
  local address=$1 platform=$2
  local start="$here/target/screens-start-$platform" config="$here/target/screens-config-$platform.json"
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
# must be running").
if [[ -n $has_simulators ]]; then
  echo "== building the iOS app"
  built cargo tauri ios build --debug --target aarch64-sim --config "$config"
  app="gen/apple/build/arm64-sim/$name.app"
  [[ -d $app ]] || { echo "no app at $app" >&2; exit 1; }
fi

# The Android app, a debug APK for the emulators' arm64, built with Java 21:
# the project's Gradle (9.6.1) runs on Java up to 26, and a Mac's default
# Java 27 failed it ("Unsupported class file major version 71", an islands
# app on this stack, 2026-10-29). A JAVA_HOME already set is kept.
if [[ -n $has_emulators ]]; then
  echo "== building the Android app"
  JAVA_HOME=${JAVA_HOME:-$(/usr/libexec/java_home -v 21 2>/dev/null || true)}
  export JAVA_HOME
  built cargo tauri android build --debug --apk --target aarch64 --config "$android_config"
  apk=$(find gen/android/app/build/outputs/apk -name '*-debug.apk' -print0 | xargs -0 ls -t | head -1)
  [[ -f $apk ]] || { echo "no APK under gen/android/app/build/outputs/apk" >&2; exit 1; }
fi

# A picture's file, moved to the name of how the device stood, read from
# its own width and height: <device>-upright.png or <device>-sideways.png.
by_position() {
  local picture=$1 at=$2 width height
  width=$(sips -g pixelWidth "$picture" | awk '/pixelWidth/ { print $2 }')
  height=$(sips -g pixelHeight "$picture" | awk '/pixelHeight/ { print $2 }')
  if ((height >= width)); then mv "$picture" "$at-upright.png"; else mv "$picture" "$at-sideways.png"; fi
}

# Waits until a WebKit webview has finished the site after the start page
# and stopped there: WebKit's log, the main frame's loads since the app was
# opened, two at least and none more for 6 s. A device that has not yet
# passed Cloudflare's challenge on the site's host loads the challenge in
# between (a fresh iPad mini, in an islands app on this stack, 2026-10-29),
# so two alone are not the site. The log a Simulator's (`xcrun simctl
# spawn <udid>` before `log`) or this Mac's, narrowed to the app's own
# process, Safari's loads left out.
site_on_webkit() {
  local device=$1 since=$2 narrow=$3 loads last=0 still=0
  shift 3
  for _ in $(seq 1 $((wait_for_site / 3))); do
    loads=$("$@" log show --start "$since" --style compact \
      --predicate "subsystem == \"com.apple.WebKit\" AND category == \"Loading\"$narrow" 2>/dev/null |
      grep -c 'didFinishLoadForFrame.*isMainFrame=1' || true)
    if ((loads >= 2 && loads == last)); then
      still=$((still + 1))
      ((still >= 2)) && return 0
    else
      still=0
    fi
    last=$loads
    sleep 3
  done
  echo "   $device: the site not seen within ${wait_for_site}s, photographed as it is"
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
    echo "   $device: no webview to ask, photographed as it is"
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
    *) echo "   $device: the site not seen within ${wait_for_site}s, photographed as it is" ;;
  esac
}

# The emulator's picture once its screen has stopped changing: two seconds
# apart until two in a row are the same, up to 20 s.
settled() {
  local serial=$1 picture=$2
  local last="$picture.last"
  "$adb" -s "$serial" exec-out screencap -p >"$picture"
  for _ in 1 2 3 4 5 6 7 8 9 10; do
    sleep 2
    mv "$picture" "$last"
    "$adb" -s "$serial" exec-out screencap -p >"$picture"
    cmp -s "$picture" "$last" && break
  done
  rm -f "$last"
}

# A Simulator's pictures, one a screen it has, a dark one dropped: the
# iPhone Duo, Apple's foldable, has two, and after a cold boot it stands
# folded, its inner screen black, the one simctl takes by default (an
# islands app on this stack, 2026-10-29). A dark screen's picture is a
# fraction of a lit one's bytes, under a twentieth of a byte a pixel. One
# lit screen keeps the device's name; several each get their width too.
shoot_simulator() {
  local udid=$1 at=$2 screen lit=() shot width height
  local screens
  # The device's own screens, by their names: `primary`, and a foldable's
  # `primary-1`; never the TV-out, wireless or resizable ones simctl also
  # lists, whose picture never comes.
  screens=$(xcrun simctl io "$udid" enumerate 2>/dev/null | awk '/Device Name: primary/ { print $3 }')
  if (($(wc -l <<<"$screens") < 2)); then
    xcrun simctl io "$udid" screenshot "$at-shot.png" >/dev/null 2>&1
    by_position "$at-shot.png" "$at"
    return 0
  fi
  for screen in $screens; do
    shot="$at-shot-$screen.png"
    xcrun simctl io "$udid" screenshot --display="$screen" "$shot" >/dev/null 2>&1 || continue
    width=$(sips -g pixelWidth "$shot" | awk '/pixelWidth/ { print $2 }')
    height=$(sips -g pixelHeight "$shot" | awk '/pixelHeight/ { print $2 }')
    if (($(stat -f %z "$shot") < width * height / 20)); then rm -f "$shot"; else lit+=("$shot"); fi
  done
  if ((${#lit[@]} == 1)); then
    by_position "${lit[0]}" "$at"
  else
    for shot in ${lit[@]+"${lit[@]}"}; do
      by_position "$shot" "$at-$(sips -g pixelWidth "$shot" | awk '/pixelWidth/ { print $2 }')"
    done
  fi
}

# One Simulator: booted if off, the app opened, its picture as it stands,
# then with --turn a second once someone has turned it, shut down if this
# run booted it.
on_simulator() {
  local device=$1 udid state at since
  read -r udid state <<<"$(simulator "$device")" || true
  if [[ -z ${udid:-} ]]; then
    echo "== $device: not on this Mac, skipped"
    return 0
  fi
  echo "== $device"
  if [[ $state != Booted ]]; then
    xcrun simctl boot "$udid"
    own_sim=$udid
  fi
  # Booted is listed before the device has finished starting, ours or one
  # another project has just booted.
  xcrun simctl bootstatus "$udid" -b >/dev/null
  # A moment for the home screen to settle: an app opened at once on a
  # fresh boot went down with it (an iPad Air, in an islands app on this
  # stack, 2026-10-29).
  [[ -n $own_sim ]] && sleep 5
  at="$out/$(printf '%s' "$device" | slug)"
  xcrun simctl install "$udid" "$app"
  # Opened, the site waited for, and opened once more where the app is no
  # longer running by then.
  for _ in 1 2; do
    xcrun simctl terminate "$udid" "$id" 2>/dev/null || true
    since=$(date '+%Y-%m-%d %H:%M:%S')
    xcrun simctl launch "$udid" "$id" >/dev/null
    site_on_webkit "$device" "$since" "" xcrun simctl spawn "$udid"
    xcrun simctl spawn "$udid" launchctl list 2>/dev/null | grep -q "UIKitApplication:$id" && break
    echo "   $device: the app was gone by its picture, opened again"
  done
  shoot_simulator "$udid" "$at"
  if [[ -n $turn ]]; then
    read -rp "   Turn the $device a quarter (⌘← in DeviceHub), then press Enter: "
    sleep 3
    shoot_simulator "$udid" "$at"
  fi
  shut_down
}

# One emulator: started if off, with no window and the software GPU (macOS
# 27 draws no screen without it, docs/testing.md), then its three pictures,
# and stopped if this run started it, gone before the next starts.
on_emulator() {
  local avd=$1 serial at size width height x rotation
  if ! grep -qx "$avd" <<<"$avds"; then
    echo "== $avd: not on this Mac, skipped"
    return 0
  fi
  echo "== $avd"
  serial=$(emulator_serial "$avd")
  if [[ -z $serial ]]; then
    "$android_sdk/emulator/emulator" -avd "$avd" -gpu swiftshader_indirect -no-window -no-audio >/dev/null 2>&1 &
    for _ in $(seq 1 60); do
      serial=$(emulator_serial "$avd")
      [[ -n $serial && $("$adb" -s "$serial" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r') == 1 ]] && break
      sleep 3
    done
    if [[ -z $serial ]]; then
      echo "   $avd: did not start, skipped"
      return 0
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
  at="$out/$(printf '%s' "$avd" | slug)"
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
      return 0
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
  local seen=0
  site_on_emulator "$avd" "$serial" || seen=$?
  if ((seen == 2)); then
    stop_own_emulator "$avd"
    return 0
  fi
  settled "$serial" "$at-shot.png"
  by_position "$at-shot.png" "$at"
  # A quarter turn and back: a phone on its side, a tablet, sideways by
  # nature, upright. Auto-rotate off for it, then as it was ("null": never
  # set).
  rotation=$("$adb" -s "$serial" shell settings get system accelerometer_rotation | tr -d '\r')
  "$adb" -s "$serial" shell settings put system accelerometer_rotation 0
  "$adb" -s "$serial" shell settings put system user_rotation 1
  sleep 3
  settled "$serial" "$at-shot.png"
  by_position "$at-shot.png" "$at"
  "$adb" -s "$serial" shell settings put system user_rotation 0
  if [[ $rotation == null ]]; then
    "$adb" -s "$serial" shell settings delete system accelerometer_rotation >/dev/null
  else
    "$adb" -s "$serial" shell settings put system accelerometer_rotation "$rotation"
  fi
  sleep 3
  settled "$serial" "$at-shot.png"
  rm -f "$at-shot.png"
  # Flings up to the page's foot, at the screen's right edge, where no link
  # stands: one landed as a tap on a footer's phone number (an islands app
  # on this stack, 2026-10-29).
  size=$("$adb" -s "$serial" shell wm size | grep -oE '[0-9]+x[0-9]+' | tail -1)
  width=${size%x*}
  height=${size#*x}
  x=$((width * 19 / 20))
  for _ in 1 2 3 4 5 6 7 8 9 10 11 12; do
    "$adb" -s "$serial" shell input swipe "$x" $((height * 3 / 4)) "$x" $((height / 4)) 100
  done
  sleep 2
  settled "$serial" "$at-bottom.png"
  stop_own_emulator "$avd"
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

# The app's window on this Mac, by the process that owns it, asked of the
# window server through JavaScript for Automation (Python's Quartz bridge
# is not on a stock Mac); empty until the window is up. The largest on the
# screen: the process also owns a menu bar's strips and a hidden 500×500
# window (an islands app on this stack, 2026-10-29).
window_of() {
  osascript -l JavaScript -e "ObjC.import('CoreGraphics');
const area = (w) => w.kCGWindowBounds.Width * w.kCGWindowBounds.Height;
const w = ObjC.deepUnwrap(ObjC.castRefToObject(\$.CGWindowListCopyWindowInfo(0, 0)))
  .filter((w) => w.kCGWindowOwnerPID == $1 && w.kCGWindowLayer == 0 && w.kCGWindowIsOnscreen)
  .sort((a, b) => area(b) - area(a))[0];
w ? String(w.kCGWindowNumber) : ''" 2>/dev/null || true
}

# The Mac's app at two window sizes, tauri.conf.json's own and its smallest
# (`minWidth`, `minHeight`): built once a size, the window given whole in a
# config of its own, since a merged config replaces a list rather than
# merging into it; opened, the site waited for in WebKit's log as on a
# Simulator, the window alone photographed (no shadow), quit.
# macos-<width>x<height>.png in points.
on_mac() {
  local size width height size_config bin pid since window at
  for size in $(python3 -I -c '
import json, sys
w = json.load(open(sys.argv[1]))["app"]["windows"][0]
print("%dx%d %dx%d" % (w["width"], w["height"], w["minWidth"], w["minHeight"]))' tauri.conf.json); do
    width=${size%x*}
    height=${size#*x}
    echo "== the Mac's app, $size"
    size_config="$here/target/screens-config-$size.json"
    python3 -I -c '
import json, sys
conf, run, width, height, out = sys.argv[1:]
merged = json.load(open(run))
window = dict(json.load(open(conf))["app"]["windows"][0], width=int(width), height=int(height))
merged["app"]["windows"] = [window]
json.dump(merged, open(out, "w"))' tauri.conf.json "$config" "$width" "$height" "$size_config"
    built cargo tauri build --debug --bundles app --config "$size_config"
    bin="target/debug/bundle/macos/$name.app/Contents/MacOS/$(conf mainBinaryName)"
    since=$(date '+%Y-%m-%d %H:%M:%S')
    "$bin" >/dev/null 2>&1 &
    pid=$!
    own_mac=$pid
    site_on_webkit "the Mac's app" "$since" " AND processID == $pid"
    window=
    for _ in $(seq 1 20); do
      window=$(window_of "$pid")
      [[ -n $window ]] && break
      sleep 1
    done
    at="$out/macos-$size.png"
    if [[ -z $window ]]; then
      echo "   the Mac's app: no window to photograph, skipped"
    elif ! screencapture -o -x -l "$window" "$at" 2>/dev/null || [[ ! -s $at ]]; then
      echo "   the Mac's app: no picture; allow Screen Recording for this terminal (System Settings, Privacy & Security)"
    fi
    shut_down
    wait "$pid" 2>/dev/null || true
  done
}

# One device at a time. The Simulators keep the terminal for the turn's
# question; the emulators get no input, since `adb shell` reads standard
# input and would swallow what is typed.
for device in ${has_simulators:+"${simulators[@]}"}; do
  on_simulator "$device"
done
for avd in ${has_emulators:+"${emulators[@]}"}; do
  on_emulator "$avd" </dev/null
done
[[ -n $has_mac ]] && on_mac

echo "== the pictures: $out"
open "$out"
