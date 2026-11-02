#!/usr/bin/env bash
# The app's links that leave it, tapped on the Android phone's emulator
# (android-phone-full), against the dev loop by default (`--https://…` for
# another site, as screens.sh): each must open its own app on the phone and
# leave the app on its page (src/lib.rs, `destination`), as an islands
# app on this stack checks it after the opener plugin's own script left
# every link to a new tab dead (2026-10-29). The starter's pages have no
# link that leaves the site, so five are put into the home page's `<main>`
# first: a call, a mail, a text, another site, and another site in a new
# tab. Each is tapped as a person taps (adb), found through the webview's
# debugging socket (Node, the stylesheet lint's, for its WebSocket). iOS
# needs a real tap no Simulator takes from here: by hand (docs/testing.md,
# The app on the phones). Exits 1 if any link failed.
set -euo pipefail

usage="usage: src-tauri/links.sh [--http(s)://address]"
site=
for option in "$@"; do
  case $option in
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
if ! command -v node >/dev/null; then
  echo "no node: links.sh reads the page with Node's WebSocket (docs/prerequisites.md)" >&2
  exit 1
fi
# The site, the build and the emulator up with the app open on it.
# shellcheck source=devices.sh
source "$(dirname "$0")/devices.sh"

avd=android-phone-full
build_android
emulator_up "$avd" </dev/null || exit 1

# The page asked over the webview's debugging socket: `page "<js>"` prints
# the expression's value as JSON, and nothing where the page did not
# answer (said on standard error, the run going on to say which link).
socket=$("$adb" -s "$serial" shell cat /proc/net/unix | grep -oE 'webview_devtools_remote_[0-9]+' | head -1)
port=$("$adb" -s "$serial" forward tcp:0 "localabstract:$socket")
page() {
  node --input-type=module -e '
const [port, expression] = process.argv.slice(1);
const pages = await (await fetch(`http://localhost:${port}/json`)).json();
const ws = new WebSocket(pages.find((p) => p.type === "page").webSocketDebuggerUrl);
ws.onopen = () => ws.send(JSON.stringify({ id: 1, method: "Runtime.evaluate", params: { expression, returnByValue: true, awaitPromise: true } }));
ws.onmessage = (m) => {
  const answer = JSON.parse(m.data);
  if (answer.id === 1) {
    console.log(JSON.stringify(answer.result?.result?.value ?? null));
    process.exit(0);
  }
};
setTimeout(() => process.exit(1), 10000);
' "$port" "$1" || echo "   the page did not answer: $1" >&2
}

# The app the phone shows on top, by its package; empty in the moment
# between two apps, which `set -e` must not take for a failure.
on_top() {
  "$adb" -s "$serial" shell dumpsys activity activities |
    grep -m1 topResumedActivity | grep -oE '[A-Za-z0-9_.]+/' | head -1 | tr -d / || true
}

# The five links in `<main>`, put in afresh before each tap: id, address,
# and whether it opens a new tab. Placeholders of the kind a page has.
links='[["e2e-tel","tel:+15550100",false],["e2e-mail","mailto:hello@example.com",false],["e2e-sms","sms:+15550100",false],["e2e-site","https://example.org/",false],["e2e-new-tab","https://example.org/",true]]'
put_links() {
  page "(() => { const main = document.querySelector('main'); if (!main) return false;
    for (const [id, href, blank] of $links) {
      document.getElementById(id)?.remove();
      const a = document.createElement('a');
      a.id = id; a.href = href; a.textContent = id;
      if (blank) a.target = '_blank';
      a.style.display = 'block'; a.style.padding = '16px';
      main.append(a);
    }
    return true; })()"
}

# The home page there, its `<main>` taking the links.
for _ in $(seq 1 "$wait_for_site"); do
  [[ $(put_links) == true ]] && break
  sleep 1
done
home=$(page 'location.pathname' | tr -d '"')
echo "== $home"

failed=0
# A link tapped: put in afresh, scrolled to the screen's middle, tapped at
# its centre in the screen's pixels (the webview lies under the status bar,
# so its top is the screen's), the app it opened waited for, then the app
# brought back and its page read.
tap() {
  local label=$1 selector=$2 expected=$3 at= last= top= back
  put_links >/dev/null
  # Read until it stands still, a second apart: the page lays out a while
  # after it changes, and a first tap once landed where the link had been.
  for _ in 1 2 3 4 5 6; do
    last=$at
    at=$(page "(() => { const e = document.querySelector('$selector'); if (!e) return ''; e.scrollIntoView({ block: 'center', behavior: 'instant' }); const b = e.getBoundingClientRect(); return Math.round((b.x + b.width / 2) * devicePixelRatio) + ' ' + Math.round((b.y + b.height / 2) * devicePixelRatio); })()" | tr -d '"')
    [[ -n $at && $at == "$last" ]] && break
    sleep 1
  done
  if [[ -z $at ]]; then
    echo "   $label: not on the page ($selector)"
    failed=1
    return 0
  fi
  # shellcheck disable=SC2086 # x and y, two words
  "$adb" -s "$serial" shell input tap $at
  # Up to 45 s for the expected app: Chrome's first start on a freshly
  # booted, software-drawn emulator took over 20, and the app the last link
  # opened can stand on top a moment longer (an islands app on this stack,
  # 2026-10-29).
  for _ in $(seq 1 45); do
    top=$(on_top)
    [[ $top == "$expected" ]] && break
    sleep 1
  done
  # The app that opened closed and the app brought back to the front,
  # waited for there: its page answers from behind too.
  [[ -n $top && $top != "$id" ]] && "$adb" -s "$serial" shell am force-stop "$top"
  "$adb" -s "$serial" shell am start -n "$id/.MainActivity" >/dev/null 2>&1
  for _ in $(seq 1 10); do
    [[ $(on_top) == "$id" ]] && break
    sleep 1
  done
  back=$(page 'location.pathname' | tr -d '"')
  if [[ $top == "$expected" && $back == "$home" ]]; then
    echo "   $label: $top, the app still on $home"
  else
    echo "   $label: ${top:-nothing} where $expected was expected, the app on ${back:-?}"
    failed=1
  fi
}

chrome=com.android.chrome
tap "a call" '#e2e-tel' com.google.android.dialer
tap "a mail" '#e2e-mail' com.google.android.gm
tap "a text" '#e2e-sms' com.google.android.apps.messaging
tap "another site" '#e2e-site' "$chrome"
tap "another site, a new tab" '#e2e-new-tab' "$chrome"

"$adb" -s "$serial" forward --remove "tcp:$port"
if ((failed)); then
  echo "== some links failed"
  exit 1
fi
echo "== every link left the app and the app stayed"
