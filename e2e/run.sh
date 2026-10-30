#!/usr/bin/env bash
# The browser tests (docs/testing.md, "Browser tests"), in the lanes an
# islands app on this stack runs them in:
#
#   e2e/run.sh chrome|edge|firefox|safari [cargo nextest's filter and flags]
#   e2e/run.sh iphone|ipad|ipad-air|android|android-tablet [cargo nextest's filter and flags]
#   e2e/run.sh desktop|phones|tablets|main|all [cargo nextest's filter and flags]
#   e2e/run.sh chrome-low|firefox-low|iphone-low|ipad-low|ipad-air-low|android-low|android-tablet-low|low [...]
#   e2e/run.sh chrome-beta|edge-beta|firefox-beta|safari-beta|beta [...]
#
# Builds the site into target/e2e (its own target folder and site, so the
# watch loop's binary and target/site are never touched) and runs the tests
# in e2e/tests through cargo-nextest, one at a time, a failure tried once
# more (e2e/.config/nextest.toml), in a browser's lane: its own site with a
# fresh database (LOCO_ENV=e2e, config/e2e.yaml) and its own driver, each on
# the lane's ports, its files in target/e2e/<browser>/ (the database, the
# server's and the driver's logs, what a failed test's page showed in
# failures/), all stopped whatever the tests did.
#
#   browser         site  driver
#   chrome          5160  9515
#   edge            5161  9516
#   firefox         5162  9517
#   safari          5163  9518
#   iphone          5164  9519  Safari, iOS Simulator
#   ipad            5165  9520  Safari, iOS Simulator
#   android         5166  9521  Chrome, emulator 5560
#   android-tablet  5167  9522  Chrome, emulator 5562
#   ipad-air        5168  9523  Safari, iOS Simulator
#
# The low bar: each lane's twin on the oldest version the site supports,
# about 30 months back (docs/testing.md, "The Low Bar"), the same tests,
# another binary (`E2E_BINARY`) or device. Edge and the Mac's Safari have
# none: no old Edge to download, and Safari comes with macOS.
#
#   chrome-low          5169  9524  Chrome for Testing 114, the first with popovers
#   firefox-low         5170  9525  Firefox 128, Tailwind v4's floor
#   iphone-low          5171  9526  Safari, iOS 17.5 Simulator
#   ipad-low            5172  9527  Safari, iOS 17.5 Simulator
#   ipad-air-low        5173  9528  Safari, iOS 17.5 Simulator
#   android-low         5174  9529  Chrome 124, Android 15, emulator 5564
#   android-tablet-low  5175  9530  Chrome 124, Android 15, emulator 5566
#
# Their browsers in E2E_LOW (~/Library/Caches/e2e-floor), their iOS
# E2E_IOS_LOW, their AVDs android-phone-low and android-tablet-low
# (docs/prerequisites.md, the low bar).
#
# The beta bar: the four desktop lanes' twins on the browsers' next
# releases, each its own binary and driver, to see a browser's change
# before visitors do. No device: a phone's browser is the same engine as a
# desktop's, and an iOS beta comes only with an Xcode beta.
#
#   chrome-beta         5176  9531  Chrome for Testing, the Beta channel
#   edge-beta           5177  9532  Edge Beta
#   firefox-beta        5178  9533  Firefox Developer Edition
#   safari-beta         5179  9534  Safari Technology Preview
#
# Chrome for Testing and the drivers in E2E_BETA (~/Library/Caches/e2e-beta):
# the highest chrome-<N> there with its chromedriver-<N>, Edge Beta with the
# msedgedriver-<N> of its major version; Edge Beta, Firefox Developer
# Edition and Safari Technology Preview in /Applications, the last driven
# by its own safaridriver (docs/prerequisites.md, the beta bar).
#
# The devices' lanes start their device and shut it down after: an iOS
# Simulator (E2E_IOS, the iOS whose Simulator safaridriver can drive;
# E2E_IPHONE, E2E_IPAD, E2E_IPAD_AIR, the devices on it), an Android
# emulator (the AVDs android-phone and android-tablet, docs/prerequisites.md)
# with the lane's port reversed to the Mac, so its Chrome reaches the site
# at the same address, and a chromedriver of that Chrome's own version.
#
# `desktop` builds once, then runs the four desktop browsers side by side,
# `phones` the two phones, `tablets` the three tablets, `main` the phones
# and the desktop browsers, `all` every top lane, all nine at once on an M1
# with 48 GB or more, else `main` and then `tablets`; each lane's output in
# target/e2e/<browser>/run.txt, and one summary at the end of what each
# passed, failed and skipped. `low` runs the low bar's seven in two
# batches, its phones and desktop browsers and then its tablets. `beta`
# runs the beta bar's four side by side. Never two families at once, nor
# beside another project's lanes: they share the devices. E2E_LANES runs
# the lanes it names instead, in order:
# E2E_LANES="android chrome" e2e/run.sh main.
#
# CHROMEDRIVER names Chrome's driver, else the one in
# ~/Library/Caches/chromedriver (docs/prerequisites.md), else `chromedriver`
# on the PATH; its version must match Chrome's. E2E_HEADED=1 shows the
# browser's window; Safari always does.
set -euo pipefail

browser=${1:?"usage: e2e/run.sh chrome|edge|firefox|safari|iphone|ipad|ipad-air|android|android-tablet|desktop|phones|tablets|main|all, a low-bar lane (the name and -low) or low, a beta lane (a desktop browser's name and -beta) or beta [cargo nextest arguments]"}
shift
# A low-bar or beta lane does what its top-bar twin does, `kind` its twin's
# name.
kind=${browser%-low}
kind=${kind%-beta}

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
e2e="$root/target/e2e"
export CARGO_TARGET_DIR="$e2e"
export LEPTOS_SITE_ROOT=target/e2e/site
export LOCO_ENV=e2e

# The site and the tests, built once; a lane a family run starts skips it
# (E2E_BUILT), several builds at once waiting on one another's lock.
# cargo-leptos from #691's commit (CLAUDE.md, Islands), and `--split`: the
# real lazy path.
build() {
  tools/cargo-leptos/check.sh
  cargo leptos build --split
  cargo test --manifest-path e2e/Cargo.toml --no-run
}

# The families' lanes, the devices first, each family longest first: a
# device boots slower beside lanes already running, and the short lanes
# fill in at the end rather than one long lane running alone.
desktop=(firefox safari chrome edge)
phones=(android iphone)
main=(android iphone firefox safari chrome edge)
tablets=(android-tablet ipad ipad-air)
low_main=(android-low iphone-low firefox-low chrome-low)
low_tablets=(android-tablet-low ipad-low ipad-air-low)
beta=(firefox-beta safari-beta chrome-beta edge-beta)
# The Mac it runs on: an M1 with 48 GB or more takes all nine top lanes at
# once, the devices first, every gap doubled, since an M1 boots a device
# slower than an M5 under the same load; any other Mac `main` and then
# `tablets`, nine at once having run a 24 GB Mac out of memory (the islands
# app's run, 2026-10-12). The low bar runs in its two batches on every Mac:
# its seven at once lost an emulator. E2E_DEVICE_GAPS and E2E_STAGGER
# override the gaps.
memory_gb=$(($(sysctl -n hw.memsize 2>/dev/null || echo 0) / 1073741824))
if [[ $(sysctl -n machdep.cpu.brand_string 2>/dev/null) == *"Apple M1"* ]] && ((memory_gb >= 48)); then
  all_at_once=(android-tablet ipad android iphone ipad-air firefox safari chrome edge)
  default_gaps="20 30 40 40 40" default_stagger=5
else
  all_at_once=()
  default_gaps="10 15 20 20 20" default_stagger=3
fi
if [[ -n ${E2E_LANES:-} ]]; then
  read -ra lanes_named <<<"$E2E_LANES"
fi

# Whether a lane starts a device, a Simulator or an emulator.
is_device() {
  local twin=${1%-low}
  [[ $twin == iphone || $twin == ipad || $twin == ipad-air || $twin == android || $twin == android-tablet ]]
}

# A lane's line from nextest's summary in its output: a skipped test is a
# pass to the runner, counted apart from the lane's skipped.txt
# (e2e/src/page.rs, `Page::skip`), a test that passed on its retry flaky;
# then the tests that failed or were flaky, and where the pages they left
# are kept.
summary() {
  local lane=$1 out=$2
  local line passed flaky failed skipped=0
  line=$(grep -sE '^ *Summary \[' "$out" | tail -1 || true)
  passed=$(grep -oE '[0-9]+ passed' <<<"$line" | grep -oE '[0-9]+' || echo 0)
  flaky=$(grep -oE '[0-9]+ flaky' <<<"$line" | grep -oE '[0-9]+' || echo 0)
  failed=$(grep -oE '[0-9]+ (failed|timed out)' <<<"$line" | grep -oE '[0-9]+' | paste -sd+ - | bc || echo 0)
  if [[ -f $e2e/$lane/skipped.txt ]]; then
    skipped=$(sort -u "$e2e/$lane/skipped.txt" | wc -l | tr -d ' ')
  fi
  echo "== $lane: $((passed - flaky - skipped)) passed, $((flaky)) flaky, $((${failed:-0})) failed, $skipped skipped"
  # A lane whose driver stopped opening sessions (e2e/src/session.rs): said
  # once, with why, its failures the list's to read, not a line each.
  if [[ -f $e2e/$lane/no-session ]]; then
    echo "   BROKEN: no session on this lane since $(cat "$e2e/$lane/no-session")"
    echo "   the tests after it failed at once; the list: target/e2e/$lane/nextest.txt"
  else
    sed -n '/^ *Summary \[/,$p' "$out" 2>/dev/null | grep -E '^ *(FAIL|FLAKY|TIMEOUT|SIG[A-Z]+) ' || true
  fi
  # The five slowest, so a test that grows slow shows without a search.
  grep -sE '^ +PASS \[' "$out" | sed -E 's/^ +PASS \[ *([0-9.]+)s\] \([^)]*\) [^ ]+ /   \1 s  /' |
    sort -rn | head -5 || true
  if [[ -n $(ls -A "$e2e/$lane/failures" 2>/dev/null) ]]; then
    echo "   what the failed pages showed: target/e2e/$lane/failures/"
  fi
}

# Why a lane stopped before its tests, in its own words: the first line of
# its run.txt that says so, never the last.
stopped_because='^(port .* is taken|the site or the driver did not start|Safari is open|Safari Technology Preview is open|no |the .* did not boot|did not boot |chromedriver |the emulator crashed|the low bar.s Chrome)'

# Each lane's boot said as soon as it is known, while the others still
# start (the lane's boot.txt, written by the lane below): "booted ipad-air
# in 34 s", or "did not boot" with why.
watch_boots() {
  local names=("$@") told=() name said
  # Ended with the run too (`$$`, the run's own shell): a run stopped by
  # Ctrl-C or a kill leaves no lane to write its line.
  while ((${#told[@]} < ${#names[@]})) && kill -0 $$ 2>/dev/null; do
    for name in "${names[@]}"; do
      [[ " ${told[*]} " == *" $name "* ]] && continue
      [[ -s $e2e/$name/boot.txt ]] || continue
      said=$(cat "$e2e/$name/boot.txt")
      if [[ $said == *"run.txt)" ]]; then
        said="did not boot $name: $(grep -m1 -E "$stopped_because" "$e2e/$name/run.txt" 2>/dev/null | cut -c1-200 || true)"
      fi
      echo "== $said"
      told+=("$name")
    done
    sleep 2
  done
}

# A family's lanes side by side, each one's output in its run.txt; `failed`
# set when any lane failed.
family() {
  local browsers=("$@")
  local lanes=()
  for each in "${browsers[@]}"; do rm -f "$e2e/$each/boot.txt"; done
  watch_boots "${browsers[@]}" &
  local watching=$!
  # The gap after each device's lane, one a device in their order, growing
  # as each later device boots beside more already running.
  local device_gaps=(${E2E_DEVICE_GAPS:-$default_gaps})
  local device=0
  for each in "${browsers[@]}"; do
    mkdir -p "$e2e/$each"
    E2E_BUILT=1 "$0" "$each" "${cargo_args[@]}" >"$e2e/$each/run.txt" 2>&1 &
    lanes+=($!)
    # Started apart, as a ramp-up, not in one breath: after a device's lane
    # the next of E2E_DEVICE_GAPS, after a browser's E2E_STAGGER.
    if [[ $each != "${browsers[-1]}" ]]; then
      if is_device "$each"; then
        sleep "${device_gaps[device]:-${device_gaps[-1]}}"
        device=$((device + 1))
      else
        sleep "${E2E_STAGGER:-$default_stagger}"
      fi
    fi
  done
  for at in "${!browsers[@]}"; do
    wait "${lanes[$at]}" || failed=1
  done
  # The watcher's last turn, then stopped: a lane killed outright never
  # writes its line, and the run does not wait on it.
  for _ in 1 2 3 4 5; do kill -0 "$watching" 2>/dev/null || break; sleep 1; done
  kill "$watching" 2>/dev/null || true
  wait "$watching" 2>/dev/null || true
}

if [[ $browser =~ ^(desktop|phones|tablets|main|all|low|beta)$ ]]; then
  cargo_args=("$@")
  build
  failed=0
  if [[ -n ${lanes_named[*]:-} ]]; then
    browsers=("${lanes_named[@]}") && family "${lanes_named[@]}"
  else
    case $browser in
      desktop) browsers=("${desktop[@]}") && family "${desktop[@]}" ;;
      phones) browsers=("${phones[@]}") && family "${phones[@]}" ;;
      tablets) browsers=("${tablets[@]}") && family "${tablets[@]}" ;;
      main) browsers=("${main[@]}") && family "${main[@]}" ;;
      # Every top lane: all nine at once on a big M1, else `main` and then
      # `tablets`, never more than three devices at once.
      all)
        if [[ -n ${all_at_once[*]:-} ]]; then
          browsers=("${all_at_once[@]}") && family "${all_at_once[@]}"
        else
          browsers=("${main[@]}" "${tablets[@]}")
          family "${main[@]}"
          family "${tablets[@]}"
        fi
        ;;
      low)
        browsers=("${low_main[@]}" "${low_tablets[@]}")
        family "${low_main[@]}"
        family "${low_tablets[@]}"
        ;;
      beta) browsers=("${beta[@]}") && family "${beta[@]}" ;;
    esac
  fi
  for each in "${browsers[@]}"; do
    out="$e2e/$each/run.txt"
    summary "$each" "$e2e/$each/nextest.txt"
    grep -E 'panicked' -A2 "$out" | grep -vE '^--|RUST_BACKTRACE' | cut -c1-400 || true
    # A lane that stopped before its tests (a port taken, its boot) or
    # whose emulator crashed.
    grep -E "$stopped_because" "$out" || true
  done
  exit "$failed"
fi

# The lanes' numbers, as the islands app on this stack has them.
case $browser in
  chrome) n=0 ;;
  edge) n=1 ;;
  firefox) n=2 ;;
  safari) n=3 ;;
  iphone) n=4 ;;
  ipad) n=5 ;;
  android) n=6 ;;
  android-tablet) n=7 ;;
  ipad-air) n=8 ;;
  chrome-low) n=9 ;;
  firefox-low) n=10 ;;
  iphone-low) n=11 ;;
  ipad-low) n=12 ;;
  ipad-air-low) n=13 ;;
  android-low) n=14 ;;
  android-tablet-low) n=15 ;;
  chrome-beta) n=16 ;;
  edge-beta) n=17 ;;
  firefox-beta) n=18 ;;
  safari-beta) n=19 ;;
  *)
    echo "unknown browser: $browser (chrome, edge, firefox, safari, iphone, ipad, ipad-air, android, android-tablet, desktop, phones, tablets, main or all; chrome, firefox and the devices also with -low, or low; the four desktop browsers also with -beta, or beta)" >&2
    exit 2
    ;;
esac
site=$((5160 + n))
driver_port=$((9515 + n))
lane="$e2e/$browser"
# The lane's boot, timed from here and said in its boot.txt (`watch_boots`,
# above): "booted" once its boot test passed, else "did not boot",
# whatever stopped it first, a port, Safari, its device, its driver.
booting=$SECONDS
mkdir -p "$lane"
# The last run's results too, so a lane stopped before its tests shows none.
rm -f "$lane/boot.txt" "$lane/boot-run.txt" "$lane/boot-run-again.txt" "$lane/boot-why.txt" "$lane/nextest.txt"
not_booted() {
  [[ -s $lane/boot.txt ]] || echo "did not boot $browser (target/e2e/$browser/run.txt)" >"$lane/boot.txt"
}
trap not_booted EXIT

case $kind in
  chrome)
    chromedriver=${CHROMEDRIVER:-$HOME/Library/Caches/chromedriver/chromedriver-mac-arm64/chromedriver}
    [[ -x $chromedriver ]] || chromedriver=chromedriver
    driver=("$chromedriver" --port="$driver_port")
    ;;
  edge) driver=(msedgedriver --port="$driver_port") ;;
  firefox) driver=(geckodriver --port "$driver_port") ;;
  safari | iphone | ipad | ipad-air) driver=(safaridriver -p "$driver_port") ;;
  # The emulator's Chrome's own version, known once it has booted
  # (`start_device`).
  android | android-tablet) driver=() ;;
esac
# The low bar's desktop browsers, the binaries downloaded for them
# (docs/testing.md, "The Low Bar"), and Chrome's own driver of its
# version; geckodriver drives Firefox from 115, the installed one serves.
low_browsers=${E2E_LOW:-$HOME/Library/Caches/e2e-floor}
E2E_BINARY=
case $browser in
  chrome-low)
    E2E_BINARY="$low_browsers/chrome-114/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing"
    driver=("$low_browsers/chromedriver-114/chromedriver" --port="$driver_port")
    ;;
  firefox-low) E2E_BINARY="$low_browsers/firefox-128/Firefox.app/Contents/MacOS/firefox" ;;
esac
# The beta bar's: Chrome for Testing's highest chrome-<N> with its own
# chromedriver; Edge Beta with the msedgedriver of its major version, which
# its own updates outrun; Firefox Developer Edition through the installed
# geckodriver; Safari Technology Preview through its own safaridriver,
# which takes it by name, not by path.
beta_browsers=${E2E_BETA:-$HOME/Library/Caches/e2e-beta}
case $browser in
  chrome-beta)
    chrome_beta=$(find "$beta_browsers" -maxdepth 1 -name 'chrome-[0-9]*' 2>/dev/null | sort -t- -k2 -n | tail -1)
    E2E_BINARY="$chrome_beta/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing"
    driver=("$beta_browsers/chromedriver-${chrome_beta##*-}/chromedriver-mac-arm64/chromedriver" --port="$driver_port")
    ;;
  edge-beta)
    E2E_BINARY="/Applications/Microsoft Edge Beta.app/Contents/MacOS/Microsoft Edge Beta"
    edge_beta=$(defaults read "/Applications/Microsoft Edge Beta.app/Contents/Info.plist" CFBundleShortVersionString 2>/dev/null || true)
    driver=("$beta_browsers/msedgedriver-${edge_beta%%.*}/msedgedriver" --port="$driver_port")
    ;;
  firefox-beta) E2E_BINARY="/Applications/Firefox Developer Edition.app/Contents/MacOS/firefox" ;;
  safari-beta)
    E2E_BINARY="Safari Technology Preview"
    driver=("/Applications/Safari Technology Preview.app/Contents/MacOS/safaridriver" -p "$driver_port")
    ;;
esac
if [[ -n $E2E_BINARY && $browser != safari-beta && ! -x $E2E_BINARY ]]; then
  echo "no browser at $E2E_BINARY (docs/prerequisites.md, the low bar or the beta bar)" >&2
  exit 1
fi
# A path, or a name on the PATH; an emulator's lane finds its own later.
if ((${#driver[@]})) && ! command -v "${driver[0]}" >/dev/null; then
  echo "no driver at ${driver[0]} (docs/prerequisites.md)" >&2
  exit 1
fi
export E2E_BINARY

# The devices.
android_sdk=${ANDROID_SDK_ROOT:-/opt/homebrew/share/android-commandlinetools}
adb="$android_sdk/platform-tools/adb"
case $browser in
  iphone) device_name=${E2E_IPHONE:-iPhone 18 Pro} ;;
  ipad) device_name=${E2E_IPAD:-iPad Pro 13-inch (M5)} ;;
  # Upright, as a Simulator boots: 820 pixels, the middle layout.
  ipad-air) device_name=${E2E_IPAD_AIR:-iPad Air 11-inch (M4)} ;;
  android) avd=android-phone emulator_port=5560 ;;
  android-tablet) avd=android-tablet emulator_port=5562 ;;
  # The low bar's: the devices iOS 17.5 came with, the iPad Air upright as
  # the top bar's; the Android 15 image's emulators, its Chrome 124.
  iphone-low) device_name=${E2E_IPHONE_LOW:-iPhone 15} ;;
  ipad-low) device_name=${E2E_IPAD_LOW:-iPad Pro 13-inch (M4)} ;;
  ipad-air-low) device_name=${E2E_IPAD_AIR_LOW:-iPad Air 11-inch (M2)} ;;
  android-low) avd=android-phone-low emulator_port=5564 ;;
  android-tablet-low) avd=android-tablet-low emulator_port=5566 ;;
esac
device=

# Boot the lane's device and set `device` to its id, the Simulator's UDID or
# the emulator's adb serial; for Android, the chromedriver of its Chrome's
# version (`CHROMEDRIVER_ANDROID` to name another).
start_device() {
  case $kind in
    iphone | ipad | ipad-air)
      local runtime=${E2E_IOS:-27.2}
      [[ $browser == *-low ]] && runtime=${E2E_IOS_LOW:-17.5}
      device=$(xcrun simctl list devices -j | python3 -I -c '
import json, sys
runtime, name = sys.argv[1], sys.argv[2]
key = "com.apple.CoreSimulator.SimRuntime.iOS-" + runtime.replace(".", "-")
found = [d["udid"] for d in json.load(sys.stdin)["devices"].get(key, []) if d["name"] == name and d["isAvailable"]]
print(found[0] if found else "")' "$runtime" "$device_name")
      if [[ -z $device ]]; then
        echo "no $device_name on the iOS $runtime Simulator (docs/prerequisites.md, the devices)" >&2
        exit 1
      fi
      local said
      said=$(xcrun simctl boot "$device" 2>&1) || true
      if ! said+=$(xcrun simctl bootstatus "$device" -b 2>&1); then
        # Once more after the Simulator service starts afresh, and only when
        # the boot's own words say the runtime is not there (SimError 401,
        # "runtime profile not found"): after a Mac's restart the service
        # can come up before the iOS image is mounted. Any other failure (a
        # slow first boot) fails this lane alone, as a restart would disturb
        # the iOS lanes beside it.
        if [[ $said != *"code=401"* && $said != *"runtime profile not found"* ]]; then
          echo "the $device_name Simulator did not boot: $(tail -3 <<<"$said")" >&2
          exit 1
        fi
        echo "the $device_name Simulator did not boot, its runtime not found; the Simulator service restarted, booting again" >&2
        killall -9 com.apple.CoreSimulator.CoreSimulatorService 2>/dev/null || true
        sleep 3
        xcrun simctl boot "$device" 2>/dev/null || true
        if ! xcrun simctl bootstatus "$device" -b >/dev/null 2>&1; then
          echo "the $device_name Simulator did not boot" >&2
          exit 1
        fi
      fi
      ;;
    android | android-tablet)
      if [[ ! -d $HOME/.android/avd/$avd.avd ]]; then
        echo "no Android emulator $avd (docs/prerequisites.md, the devices)" >&2
        exit 1
      fi
      device=emulator-$emulator_port
      # With no window, on the Mac's GPU (`-gpu host`): `auto` drew in
      # software, on the cores the tests need. Its Vulkan off, Android's
      # Chrome drawing through OpenGL ES: through Vulkan, MoltenVK crashed
      # the emulator mid-lane now and then (`emulator_crash` below). A cold
      # boot (`-no-snapshot`): resumed from the state the lane before left,
      # Chrome came back on the day before's clock and lost its connection.
      "$android_sdk/emulator/emulator" -avd "$avd" -port "$emulator_port" \
        -no-window -no-audio -no-boot-anim -no-snapshot -gpu host -feature -Vulkan >"$lane/emulator.log" 2>&1 &
      emulator_pid=$!
      pids+=($!)
      local waited=0
      until [[ $("$adb" -s "$device" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r') == 1 ]]; do
        if ((waited >= 180)); then
          echo "the $avd emulator did not boot; $lane/emulator.log:" >&2
          tail -20 "$lane/emulator.log" >&2
          exit 1
        fi
        sleep 2
        waited=$((waited + 2))
      done
      # The emulator's localhost is its own: the lane's port reversed to
      # the Mac's, so its Chrome reaches the site at the same address.
      "$adb" -s "$device" reverse "tcp:$site" "tcp:$site" >/dev/null
      local chrome
      chrome=$("$adb" -s "$device" shell dumpsys package com.android.chrome | grep -m1 -o 'versionName=[0-9]*' | cut -d= -f2)
      # The low bar's Chrome is the image's own 124, the Play Store off: on
      # 2026-10-29 it had moved both low emulators to 156, and the lanes ran
      # on 156's driver without a word (docs/testing.md, The Low Bar).
      if [[ $browser == *-low && $chrome != "${E2E_ANDROID_LOW_CHROME:-124}" ]]; then
        echo "the low bar's Chrome is $chrome on $avd, not ${E2E_ANDROID_LOW_CHROME:-124} (docs/testing.md, The Low Bar)" >&2
        exit 1
      fi
      local chromedriver=${CHROMEDRIVER_ANDROID:-$HOME/Library/Caches/chromedriver-android/$chrome/chromedriver-mac-arm64/chromedriver}
      if [[ ! -x $chromedriver ]]; then
        echo "chromedriver $chrome for the emulator's Chrome is missing: $chromedriver (docs/prerequisites.md, the devices)" >&2
        exit 1
      fi
      driver=("$chromedriver" --port="$driver_port")
      ;;
  esac
}

# The lane's emulator crashed during the run: said with the crash's top
# frame and its report, so a lane that lost its sessions says why. macOS's
# report (~/Library/Logs/DiagnosticReports/qemu-system-*.ips, newer than
# the lane's start mark) names the process and its parent, so another
# Android lane's crash beside it is not taken for this one's.
emulator_pid=
emulator_crash() {
  [[ -n $emulator_pid ]] || return 0
  find "$HOME/Library/Logs/DiagnosticReports" -name 'qemu-system-*.ips' -newer "$lane/started" 2>/dev/null |
    python3 -I -c '
import json, sys
pid = int(sys.argv[1])
for path in sys.stdin.read().split():
    try:
        report = json.loads(open(path).read().split("\n", 1)[1])
    except (OSError, ValueError, IndexError):
        continue
    if pid not in (report.get("pid"), report.get("parentPid")):
        continue
    frames = [t for t in report.get("threads", []) if t.get("triggered")]
    top = frames[0]["frames"][0].get("symbol", "?") if frames and frames[0].get("frames") else "?"
    print(f"the emulator crashed: {top} ({path})")' "$emulator_pid"
}

# What a Simulator lane saw when its boot test failed, into its
# boot-why.txt: the Simulator's state, what safaridriver and Apple's
# WebDriver service logged (safaridriver writes nothing to driver.log), and
# the crash and fault reports written since the lane began.
ios_boot_why() {
  {
    echo "== the $device_name Simulator"
    xcrun simctl list devices | grep -F "$device" || echo "$device not listed"
    echo
    echo "== safaridriver and WebDriver in the system log, the last 5 minutes"
    log show --last 5m --style compact \
      --predicate 'process == "safaridriver" OR process CONTAINS "WebDriver" OR subsystem CONTAINS "WebDriver"' \
      2>/dev/null | tail -60
    echo
    echo "== crash and fault reports since the lane began"
    find "$HOME/Library/Logs/DiagnosticReports" -newer "$lane/started" \( -name '*WebDriver*' \
      -o -name '*safaridriver*' -o -name '*Simulator*' -o -name 'launchd_sim*' -o -name 'MobileSafari*' \) 2>/dev/null
  } >"$lane/boot-why.txt" 2>&1
}

# Shut the lane's device down.
stop_device() {
  case $kind in
    iphone | ipad | ipad-air) [[ -n $device ]] && xcrun simctl shutdown "$device" 2>/dev/null || true ;;
    android | android-tablet) [[ -n $device ]] && "$adb" -s "$device" emu kill >/dev/null 2>&1 || true ;;
  esac
}

# safaridriver's session waits for good behind an open Safari's windows: the
# run asks for Safari to be quit rather than closing someone's windows. The
# beta lane's Safari is Safari Technology Preview, an app of its own.
safari_app=
case $browser in
  safari) safari_app=Safari ;;
  safari-beta) safari_app="Safari Technology Preview" ;;
esac
if [[ -n $safari_app ]] && pgrep -xq "$safari_app"; then
  echo "$safari_app is open: quit it first (its windows hold up the test session)" >&2
  exit 1
fi

for port in "$site" "$driver_port"; do
  if lsof -nP -iTCP:"$port" -sTCP:LISTEN >/dev/null 2>&1; then
    echo "port $port is taken: a run still going, or something else on it" >&2
    exit 1
  fi
done

if [[ -z ${E2E_BUILT:-} ]]; then
  build
  # The boot timed from here, not counting the build.
  booting=$SECONDS
fi

# A fresh lane: its database (config/e2e.yaml, E2E_LANE_DIR), the pages a
# failed test left, the skipped tests, and the broken lane's mark and its
# count (e2e/src/session.rs), one run's never stopping the next. The start
# mark: the reports newer than it are this lane's.
rm -f "$lane/app_e2e.sqlite" "$lane"/app_e2e.sqlite-* "$lane/skipped.txt"
rm -f "$lane/no-session" "$lane/session-failures"
rm -rf "$lane/failures"
touch "$lane/started"

pids=()
stop() {
  stop_device
  for pid in "${pids[@]}"; do kill "$pid" 2>/dev/null || true; done
  wait 2>/dev/null || true
  # Safari stays open after the driver's sessions; it was closed when the
  # run began (above), so the one open now is the run's own. Quit as an app
  # would be, which a driven Safari can ignore, then ended.
  if [[ -n $safari_app ]]; then
    osascript -e "quit app \"$safari_app\"" 2>/dev/null || true
    sleep 1
    pkill -x "$safari_app" 2>/dev/null || true
  fi
  # This trap took `not_booted`'s place.
  not_booted
}
trap stop EXIT

E2E_PORT=$site E2E_LANE_DIR="target/e2e/$browser" "$e2e/debug/app" start >"$lane/server.log" 2>&1 &
pids+=($!)
start_device
"${driver[@]}" >"$lane/driver.log" 2>&1 &
driver_pid=$!
pids+=($!)

# The site and the driver up within a minute, or the lane stops with their
# last words.
wait_up() {
  local waited=0
  until curl -sf "http://localhost:$site/_ping" >/dev/null && curl -sf "http://localhost:$driver_port/status" >/dev/null; do
    if ((waited >= 60)); then
      echo "the site or the driver did not start; $lane/server.log and driver.log:" >&2
      tail -20 "$lane/server.log" "$lane/driver.log" >&2
      exit 1
    fi
    sleep 1
    waited=$((waited + 1))
  done
}
wait_up

export E2E_BROWSER=$browser E2E_BASE="http://localhost:$site" E2E_DRIVER="http://localhost:$driver_port" \
  E2E_DIR=$lane E2E_DEVICE=$device E2E_ADB=$adb

# The boot test alone (e2e/tests/boot.rs), its output into `$1`, nextest's
# flags after it (`--retries 0`).
boot_test() {
  local out=$1
  shift
  cargo nextest run --manifest-path e2e/Cargo.toml --profile "$browser" "$@" \
    -E 'test(=boot::the_lane_opens_its_home_page)' >"$lane/$out" 2>&1
}

# The Simulator and its driver started afresh: the driver stopped, the
# Simulator shut down and booted again (`start_device`), a new driver on
# the same port, both waited for. A driver that answered its status and
# then never the first session's request did the same on nextest's own
# second try, which reuses both.
restart_ios_lane() {
  kill "$driver_pid" 2>/dev/null || true
  wait "$driver_pid" 2>/dev/null || true
  # Its port free before the new one takes it, ten seconds at most.
  local waited=0
  while lsof -nP -iTCP:"$driver_port" -sTCP:LISTEN >/dev/null 2>&1 && ((waited < 10)); do
    sleep 1
    waited=$((waited + 1))
  done
  xcrun simctl shutdown "$device" 2>/dev/null || true
  start_device
  "${driver[@]}" >>"$lane/driver.log" 2>&1 &
  driver_pid=$!
  pids+=($!)
  wait_up
}

# The boot first, alone: a session and the home page, or the lane stops
# here with why, rather than every test failing on it. A Simulator lane
# tries once, then once more with the Simulator and its driver started
# afresh, what it saw kept in its boot-why.txt (`ios_boot_why`); every
# other lane takes nextest's own second try.
case $kind in
  iphone | ipad | ipad-air)
    if boot_test boot-run.txt --retries 0; then
      booted=yes
    else
      ios_boot_why
      echo "the $browser lane's first session failed: the Simulator and its driver started afresh, the boot tried again (target/e2e/$browser/boot-why.txt)" >&2
      restart_ios_lane
      boot_test boot-run-again.txt && booted="yes, after a restart" || booted=
    fi
    ;;
  *) boot_test boot-run.txt && booted=yes || booted= ;;
esac
if [[ $booted == yes ]]; then
  echo "booted $browser in $((SECONDS - booting)) s" | tee "$lane/boot.txt"
elif [[ -n $booted ]]; then
  echo "booted $browser in $((SECONDS - booting)) s, after a restart (target/e2e/$browser/boot-why.txt)" | tee "$lane/boot.txt"
else
  last=boot-run.txt
  [[ -s $lane/boot-run-again.txt ]] && last=boot-run-again.txt
  why=$(grep -A1 -m1 'panicked at' "$lane/$last" | tail -1 | sed -E 's/^ +//' | cut -c1-200 || true)
  more=
  [[ -s $lane/boot-why.txt ]] && more=" (target/e2e/$browser/boot-why.txt)"
  echo "did not boot $browser: ${why:-see target/e2e/$browser/$last}$more" | tee "$lane/boot.txt" >&2
  exit 1
fi

# The tests, the boot not again where no arguments were given. Given any,
# it runs once more: nextest joins several `-E` with "or", so "not the
# boot" can't be added beside a filter of the caller's.
filter=()
(($#)) || filter=(-E 'not test(/^boot::/)')
cargo nextest run --manifest-path e2e/Cargo.toml --profile "$browser" "${filter[@]}" "$@" 2>&1 |
  tee "$lane/nextest.txt" || failed=1
emulator_crash

# The tests that don't apply to this browser, and why.
if [[ -s $lane/skipped.txt ]]; then
  echo
  echo "Skipped, $(sort -u "$lane/skipped.txt" | wc -l | tr -d ' ') ($lane/skipped.txt):"
  sort -u "$lane/skipped.txt"
fi
echo
summary "$browser" "$lane/nextest.txt"
exit "${failed:-0}"
