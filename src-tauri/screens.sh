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
# The site, the builds, the emulators started and the devices shut down
# again: what links.sh shares.
# shellcheck source=devices.sh
source "$(dirname "$0")/devices.sh"

out="$here/target/screens"
rm -rf "$out"
mkdir -p "$out"

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

# Which of the devices this Mac has, so a platform with none is not built.
has_simulators=
for device in "${simulators[@]}"; do
  [[ -n $(simulator "$device") ]] && has_simulators=yes
done
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

if [[ -n $has_simulators ]]; then
  build_ios
fi
if [[ -n $has_emulators ]]; then
  build_android
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

# One emulator, up with the app on the site (devices.sh, emulator_up), then
# its three pictures, and stopped if this run started it, gone before the
# next starts.
on_emulator() {
  local avd=$1 at size width height x rotation
  emulator_up "$avd" || return 0
  at="$out/$(printf '%s' "$avd" | slug)"
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
