#!/usr/bin/env bash
# The browser tests (docs/testing.md, "Browser tests"):
#
#   e2e/run.sh chrome|edge|firefox|safari [cargo nextest's filter and flags]
#   e2e/run.sh chrome-low|firefox-low [cargo nextest's filter and flags]
#   e2e/run.sh desktop|low [cargo nextest's filter and flags]
#
# Builds the site into target/e2e (its own target folder and site, so the
# watch loop's binary and target/site are never touched) and runs the tests
# in e2e/tests through cargo-nextest, one at a time, a failure tried once
# more (e2e/.config/nextest.toml), in a browser's lane: its own site with a
# fresh database (LOCO_ENV=e2e, config/e2e.yaml) and its own driver, each on
# the lane's ports, its files in target/e2e/<browser>/ (the database, the
# server's and the driver's logs, what a failed test's page showed in
# failures/), all stopped whatever the tests did. Taken from an islands app
# on this stack, which runs the same lanes.
#
#   browser      site  driver
#   chrome       5160  9515
#   edge         5161  9516
#   firefox      5162  9517
#   safari       5163  9518
#
# The low bar: a lane's twin on the oldest version the site supports
# (docs/testing.md, "The Low Bar"), the same tests, another binary
# (`E2E_BINARY`), from E2E_LOW (~/Library/Caches/e2e-floor). Edge and
# Safari have none: no old Edge to download, and Safari comes with macOS.
#
#   chrome-low   5169  9524  Chrome for Testing 114, the first with popovers
#   firefox-low  5170  9525  Firefox 128, Tailwind v4's floor
#
# `desktop` builds once, then runs the four browsers side by side, `low`
# the low bar's two; each lane's output in target/e2e/<browser>/run.txt,
# and one summary at the end of what each passed, failed and skipped.
# E2E_LANES runs the lanes it names instead, in order:
# E2E_LANES="chrome firefox" e2e/run.sh desktop.
#
# CHROMEDRIVER names Chrome's driver, else the one in
# ~/Library/Caches/chromedriver (docs/testing.md), else `chromedriver` on
# the PATH; its version must match Chrome's. E2E_HEADED=1 shows the
# browser's window; Safari always does.
set -euo pipefail

browser=${1:?"usage: e2e/run.sh chrome|edge|firefox|safari|chrome-low|firefox-low|desktop|low [cargo nextest arguments]"}
shift
# A low-bar lane does what its top-bar twin does, `kind` its twin's name.
kind=${browser%-low}

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
e2e="$root/target/e2e"
export CARGO_TARGET_DIR="$e2e"
export LEPTOS_SITE_ROOT=target/e2e/site
export LOCO_ENV=e2e

# The site and the tests, built once; a lane a family run starts skips it
# (E2E_BUILT), several builds at once waiting on one another's lock. The
# patched cargo-leptos (CLAUDE.md, Islands), and `--split`: the real lazy
# path.
build() {
  tools/cargo-leptos/check.sh
  cargo leptos build --split
  cargo test --manifest-path e2e/Cargo.toml --no-run
}

desktop=(firefox safari chrome edge)
low=(firefox-low chrome-low)
# Started apart, as a ramp-up, not in one breath; an M1 starts a browser
# slower than an M5 under the same load. E2E_STAGGER overrides it.
if [[ $(sysctl -n machdep.cpu.brand_string 2>/dev/null) == *"Apple M1"* ]]; then
  default_stagger=5
else
  default_stagger=3
fi
if [[ -n ${E2E_LANES:-} ]]; then
  read -ra lanes_named <<<"$E2E_LANES"
fi

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
stopped_because='^(port .* is taken|the site or the driver did not start|Safari is open|no |did not boot )'

# Each lane's boot said as soon as it is known, while the others still
# start (the lane's boot.txt, written by the lane below): "booted safari in
# 21 s", or "did not boot" with why.
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
  for each in "${browsers[@]}"; do
    mkdir -p "$e2e/$each"
    E2E_BUILT=1 "$0" "$each" "${cargo_args[@]}" >"$e2e/$each/run.txt" 2>&1 &
    lanes+=($!)
    if [[ $each != "${browsers[-1]}" ]]; then
      sleep "${E2E_STAGGER:-$default_stagger}"
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

if [[ $browser =~ ^(desktop|low)$ ]]; then
  cargo_args=("$@")
  build
  failed=0
  if [[ -n ${lanes_named[*]:-} ]]; then
    browsers=("${lanes_named[@]}")
  elif [[ $browser == desktop ]]; then
    browsers=("${desktop[@]}")
  else
    browsers=("${low[@]}")
  fi
  family "${browsers[@]}"
  for each in "${browsers[@]}"; do
    out="$e2e/$each/run.txt"
    summary "$each" "$e2e/$each/nextest.txt"
    grep -E 'panicked' -A2 "$out" | grep -vE '^--|RUST_BACKTRACE' | cut -c1-400 || true
    # A lane that stopped before its tests (a port taken, its boot).
    grep -E "$stopped_because" "$out" || true
  done
  exit "$failed"
fi

# The lanes' numbers, as the islands app on this stack has them: the gaps
# are its device lanes', which the starter doesn't run (yet).
case $browser in
  chrome) n=0 ;;
  edge) n=1 ;;
  firefox) n=2 ;;
  safari) n=3 ;;
  chrome-low) n=9 ;;
  firefox-low) n=10 ;;
  *)
    echo "unknown browser: $browser (chrome, edge, firefox or safari, chrome-low or firefox-low, desktop or low)" >&2
    exit 2
    ;;
esac
site=$((5160 + n))
driver_port=$((9515 + n))
lane="$e2e/$browser"
# The lane's boot, timed from here and said in its boot.txt (`watch_boots`,
# above): "booted" once its boot test passed, else "did not boot",
# whatever stopped it first.
booting=$SECONDS
mkdir -p "$lane"
# The last run's results too, so a lane stopped before its tests shows none.
rm -f "$lane/boot.txt" "$lane/boot-run.txt" "$lane/nextest.txt"
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
  safari) driver=(safaridriver -p "$driver_port") ;;
esac
# The low bar's browsers, the binaries downloaded for them
# (docs/testing.md, "The Low Bar"), and Chrome's own driver of its version;
# geckodriver drives Firefox from 115, the installed one serves.
low_browsers=${E2E_LOW:-$HOME/Library/Caches/e2e-floor}
E2E_BINARY=
case $browser in
  chrome-low)
    E2E_BINARY="$low_browsers/chrome-114/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing"
    driver=("$low_browsers/chromedriver-114/chromedriver" --port="$driver_port")
    ;;
  firefox-low) E2E_BINARY="$low_browsers/firefox-128/Firefox.app/Contents/MacOS/firefox" ;;
esac
if [[ -n $E2E_BINARY && ! -x $E2E_BINARY ]]; then
  echo "no browser at $E2E_BINARY (docs/testing.md, \"The Low Bar\")" >&2
  exit 1
fi
if ! command -v "${driver[0]}" >/dev/null; then
  echo "no driver at ${driver[0]} (docs/prerequisites.md)" >&2
  exit 1
fi
export E2E_BINARY

# safaridriver's session waits for good behind an open Safari's windows: the
# run asks for Safari to be quit rather than closing someone's windows.
if [[ $kind == safari ]] && pgrep -xq Safari; then
  echo "Safari is open: quit it first (its windows hold up the test session)" >&2
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
# count (e2e/src/session.rs), one run's never stopping the next.
rm -f "$lane/app_e2e.sqlite" "$lane"/app_e2e.sqlite-* "$lane/skipped.txt"
rm -f "$lane/no-session" "$lane/session-failures"
rm -rf "$lane/failures"

pids=()
stop() {
  for pid in "${pids[@]}"; do kill "$pid" 2>/dev/null || true; done
  wait 2>/dev/null || true
  # Safari stays open after the driver's sessions; it was closed when the
  # run began (above), so the one open now is the run's own. Quit as an app
  # would be, which a driven Safari can ignore, then ended.
  if [[ $kind == safari ]]; then
    osascript -e 'quit app "Safari"' 2>/dev/null || true
    sleep 1
    pkill -x Safari 2>/dev/null || true
  fi
  # This trap took `not_booted`'s place.
  not_booted
}
trap stop EXIT

E2E_PORT=$site E2E_LANE_DIR="target/e2e/$browser" "$e2e/debug/app" start >"$lane/server.log" 2>&1 &
pids+=($!)
"${driver[@]}" >"$lane/driver.log" 2>&1 &
pids+=($!)

# The site and the driver up within a minute, or the lane stops with their
# last words.
waited=0
until curl -sf "http://localhost:$site/_ping" >/dev/null && curl -sf "http://localhost:$driver_port/status" >/dev/null; do
  if ((waited >= 60)); then
    echo "the site or the driver did not start; $lane/server.log and driver.log:" >&2
    tail -20 "$lane/server.log" "$lane/driver.log" >&2
    exit 1
  fi
  sleep 1
  waited=$((waited + 1))
done

export E2E_BROWSER=$browser E2E_BASE="http://localhost:$site" E2E_DRIVER="http://localhost:$driver_port" E2E_DIR=$lane

# The boot first, alone (e2e/tests/boot.rs): a session and the home page,
# or the lane stops here with why, rather than every test failing on it.
if cargo nextest run --manifest-path e2e/Cargo.toml --profile "$browser" \
  -E 'test(=boot::the_lane_opens_its_home_page)' >"$lane/boot-run.txt" 2>&1; then
  echo "booted $browser in $((SECONDS - booting)) s" | tee "$lane/boot.txt"
else
  why=$(grep -A1 -m1 'panicked at' "$lane/boot-run.txt" | tail -1 | sed -E 's/^ +//' | cut -c1-200 || true)
  echo "did not boot $browser: ${why:-see target/e2e/$browser/boot-run.txt}" | tee "$lane/boot.txt" >&2
  exit 1
fi

# The tests, the boot not again where no arguments were given. Given any,
# it runs once more: nextest joins several `-E` with "or", so "not the
# boot" can't be added beside a filter of the caller's.
filter=()
(($#)) || filter=(-E 'not test(/^boot::/)')
cargo nextest run --manifest-path e2e/Cargo.toml --profile "$browser" "${filter[@]}" "$@" 2>&1 |
  tee "$lane/nextest.txt" || failed=1

# The tests that don't apply to this browser, and why.
if [[ -s $lane/skipped.txt ]]; then
  echo
  echo "Skipped, $(sort -u "$lane/skipped.txt" | wc -l | tr -d ' ') ($lane/skipped.txt):"
  sort -u "$lane/skipped.txt"
fi
echo
summary "$browser" "$lane/nextest.txt"
exit "${failed:-0}"
