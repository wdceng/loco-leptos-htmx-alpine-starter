#!/usr/bin/env bash
# The upstream fixes this template works around, and whether each has been
# released, so its workaround can go (docs/decisions.md, each "Way back").
# Reads GitHub's public API; no login needed.
#
#   1. htmx #4147: Chrome 150 to 156's parseHTMLUnsafe drops the comments
#      Leptos hydrates by. Workaround: parseHTMLUnsafe deleted, so htmx
#      parses with DOMParser (public/js/htmx-islands.js), until Chrome 156
#      is below the low bar. htmx's own fix in 4.0.1 (#4150) throws in
#      Chrome 124 to 145, #4182, which is what is watched now; the deleted
#      function keeps 4.0.1 off that path too, so the line stays for both.
#
# Gone with Leptos 0.9 and cargo-leptos 0.3.12 (2026-11-10): cargo-leptos
# #691 and wasm-split #79 (released in 0.3.12), leptos #4953 and #4978 (in
# 0.9.0-rc1).
#
# Exit 0: nothing to act on. Exit 2: something looks released or closed,
# see "act:" lines. Exit 1: an API could not be read.
set -euo pipefail

get() { curl -sfL -A "loco-leptos-htmx-alpine-starter upstream check" "$1" || { echo "cannot read $1" >&2; exit 1; }; }
field() { tr ',' '\n' | sed -n "s/.*\"$1\": *\"\{0,1\}\([^\",}]*\)\"\{0,1\}.*/\1/p" | head -1; }

# 1. htmx #4182: an issue, so closed is the signal; its outcome is read by
# hand. #4147 itself closed on 2026-10-18, Chrome 157 being the fix.
h_state="$(get https://api.github.com/repos/bigskysoftware/htmx/issues/4182 | field state)"
h_tag="$(get https://api.github.com/repos/bigskysoftware/htmx/releases/latest | field tag_name)"
echo "1. htmx #4182             issue $h_state, latest release $h_tag"
echo

if [ "$h_state" = "closed" ]; then
  echo "act: htmx #4182 is closed: read how; a release with its fix parses safely in Chrome 124 to 145, but the parseHTMLUnsafe workaround stays while Chrome 150 to 156 are above the low bar (docs/decisions.md)."
  exit 2
fi
echo "Nothing released yet: every workaround stays."
exit 0
