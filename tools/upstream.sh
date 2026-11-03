#!/usr/bin/env bash
# The upstream fixes this template works around, and whether each has been
# released, so its workaround can go (docs/decisions.md, each "Way back").
# Reads GitHub's and crates.io's public APIs; no login needed.
#
#   1. cargo-leptos PR #691 (issue #690): stale names after a hashed split
#      build. Workaround: cargo-leptos from #691's commit
#      (tools/cargo-leptos/).
#   2. wasm-split-prototype #79: the split loader's imports unsorted, new
#      names on every build. Fixed in wasm_split_cli_support 0.2.4, which
#      the same install takes unlocked; a release counts once its own lock
#      has it.
#   3. leptos PR #4953: an island's state disposed when it leaves the page.
#      Workaround: an island collector, until it is released.
#   4. htmx #4147: Chrome 150 to 156's parseHTMLUnsafe drops the comments
#      Leptos hydrates by. Workaround: parseHTMLUnsafe deleted, so htmx
#      parses with DOMParser (public/js/htmx-islands.js), until Chrome 156
#      is below the low bar. htmx's own fix in 4.0.1 (#4150) throws in
#      Chrome 124 to 145, #4182, which is what is watched now; the deleted
#      function keeps 4.0.1 off that path too, so the line stays for both.
#   5. leptos PR #4978 (issue #4959): the islands loader waits for idle time
#      with no timeout, so Chrome 114 to 123 can start no island.
#      Workaround: Leptos from a fork with the timeout ([patch.crates-io] in
#      Cargo.toml).
#
# Exit 0: nothing to act on. Exit 2: something looks released or closed,
# see "act:" lines. Exit 1: an API could not be read.
set -euo pipefail

PATCHED_SPLIT=0.2.3   # the unsorted wasm_split_cli_support; a release must lock a newer one

get() { curl -sfL -A "loco-leptos-htmx-alpine-starter upstream check" "$1" || { echo "cannot read $1" >&2; exit 1; }; }
field() { tr ',' '\n' | sed -n "s/.*\"$1\": *\"\{0,1\}\([^\",}]*\)\"\{0,1\}.*/\1/p" | head -1; }
nonull() { [ "$1" = "null" ] && echo "" || echo "$1"; }
newer() { [ "$1" != "$2" ] && [ "$(printf '%s\n%s\n' "$1" "$2" | sort -V | tail -1)" = "$1" ]; }

# Whether release tag $3 of repo $1 holds commit $2: on its line ("ahead"
# or "identical" in GitHub's compare), or cherry-picked onto another, as
# Leptos backports to 0.8. A release dated after the merge proves neither:
# leptos v0.8.23 came after #4953 and #4978 and holds none of them.
# Prints yes, backport or no.
has_commit() {
  local cmp
  cmp="$(get "https://api.github.com/repos/$1/compare/$2...$3")"
  case "$(printf '%s' "$cmp" | grep -o '"status": *"[a-z]*"' | head -1 | sed 's/.*"\([a-z]*\)"$/\1/')" in
    ahead | identical) echo yes ;;
    *) if printf '%s' "$cmp" | grep -q "cherry picked from commit $2"; then echo backport; else echo no; fi ;;
  esac
}

act=0

# 1. cargo-leptos #691, and 2. what the latest cargo-leptos release locks.
pr="$(get https://api.github.com/repos/leptos-rs/cargo-leptos/pulls/691)"
cl_merged="$(printf '%s' "$pr" | field merged)"
cl_merged_at="$(nonull "$(printf '%s' "$pr" | field merged_at)")"
release="$(get https://api.github.com/repos/leptos-rs/cargo-leptos/releases/latest)"
cl_tag="$(printf '%s' "$release" | field tag_name)"
cl_published="$(printf '%s' "$release" | field published_at)"
lock="$(get "https://raw.githubusercontent.com/leptos-rs/cargo-leptos/$cl_tag/Cargo.lock")"
cl_locks="$(printf '%s\n' "$lock" | grep -A1 '^name = "wasm_split_cli_support"' | sed -n 's/^version = "\(.*\)"/\1/p')"
split_issue="$(nonull "$(get https://api.github.com/repos/WorldSEnder/wasm-split-prototype/issues/79 | field state)")"
split_crate="$(get https://crates.io/api/v1/crates/wasm_split_cli_support | field max_stable_version)"

fix1=no
if [ "$cl_merged" = "true" ]; then
  fix1="$(has_commit leptos-rs/cargo-leptos "$(printf '%s' "$pr" | field merge_commit_sha)" "$cl_tag")"
fi
fix2=no
if newer "$cl_locks" "$PATCHED_SPLIT"; then fix2=yes; fi

echo "1. cargo-leptos #691      merged=$cl_merged${cl_merged_at:+ ($cl_merged_at)}, latest release $cl_tag ($cl_published): released=$fix1"
echo "2. wasm-split #79         issue $split_issue, crates.io $split_crate, $cl_tag locks $cl_locks: released=$fix2"

# 3. leptos #4953, against the latest leptos release.
lpr="$(get https://api.github.com/repos/leptos-rs/leptos/pulls/4953)"
l_merged="$(printf '%s' "$lpr" | field merged)"
l_merged_at="$(nonull "$(printf '%s' "$lpr" | field merged_at)")"
l_state="$(printf '%s' "$lpr" | field state)"
lrel="$(get https://api.github.com/repos/leptos-rs/leptos/releases/latest)"
l_tag="$(printf '%s' "$lrel" | field tag_name)"
fix3=no
if [ "$l_merged" = "true" ]; then
  fix3="$(has_commit leptos-rs/leptos "$(printf '%s' "$lpr" | field merge_commit_sha)" "$l_tag")"
fi
echo "3. leptos #4953           $l_state, merged=$l_merged${l_merged_at:+ ($l_merged_at)}, latest release $l_tag: released=$fix3"

# 4. htmx #4182: an issue, so closed is the signal; its outcome is read by
# hand. #4147 itself closed on 2026-10-18, Chrome 157 being the fix.
h_state="$(get https://api.github.com/repos/bigskysoftware/htmx/issues/4182 | field state)"
h_tag="$(get https://api.github.com/repos/bigskysoftware/htmx/releases/latest | field tag_name)"
echo "4. htmx #4182             issue $h_state, latest release $h_tag"

# 5. leptos #4978, the PR itself: issue #4959 was closed before it merged.
# Merged into `main`, the 0.9 line; the 0.8 line is released from
# `leptos_0.8_latest`, where it can only arrive as a backport.
idle="$(get https://api.github.com/repos/leptos-rs/leptos/pulls/4978)"
i_state="$(printf '%s' "$idle" | field state)"
i_merged="$(printf '%s' "$idle" | field merged)"
i_merged_at="$(nonull "$(printf '%s' "$idle" | field merged_at)")"
i_base="$(printf '%s' "$idle" | tr ',' '\n' | sed -n 's/.*"ref": *"\([^"]*\)".*/\1/p' | sed -n 2p)"
fix5=no
if [ "$i_merged" = "true" ]; then
  fix5="$(has_commit leptos-rs/leptos "$(printf '%s' "$idle" | field merge_commit_sha)" "$l_tag")"
fi
echo "5. leptos #4978           $i_state, merged=$i_merged${i_merged_at:+ ($i_merged_at)} into $i_base, latest release $l_tag: released=$fix5"
echo

if [ "$fix1" != no ] && [ "$fix2" = yes ]; then
  act=1
  cat <<EOF
act: cargo-leptos $cl_tag has both split fixes. Test the plain release, installed
     into a scratch folder, with the two-build check:
  cargo install --locked --root /tmp/cargo-leptos-plain cargo-leptos@${cl_tag#v} &&
  cp target/release/hash.txt /tmp/hash.txt.saved 2>/dev/null; for run in 1 2; do PATH=/tmp/cargo-leptos-plain/bin:\$PATH LEPTOS_HASH_FILES=true LEPTOS_SITE_ROOT=target/site-check cargo leptos build --release --split --frontend-only >/dev/null && ls target/site-check/pkg | sort > /tmp/names-\$run.txt; done &&
  diff /tmp/names-1.txt /tmp/names-2.txt && grep -oh '__wasm_split\.[A-Za-z0-9_-]*\.js' target/site-check/pkg/app.*.js | head -1 | xargs -I{} test -f target/site-check/pkg/{} && echo "PLAIN $cl_tag PASSES";
  cp /tmp/hash.txt.saved target/release/hash.txt 2>/dev/null; rm -rf target/site-check
     If it passes: "Way back" for cargo-leptos from #691's commit in docs/decisions.md.
EOF
fi
if [ "$fix3" != no ]; then
  act=1
  echo "act: leptos $l_tag disposes islands (#4953, $fix3): the island collector can give way to \$\$dispose (docs/decisions.md)."
fi
if [ "$fix5" != no ]; then
  act=1
  echo "act: leptos $l_tag holds #4978 ($fix5): move Cargo.toml to it and drop the fork pin (docs/decisions.md)."
fi
if [ "$h_state" = "closed" ]; then
  act=1
  echo "act: htmx #4182 is closed: read how; a release with its fix parses safely in Chrome 124 to 145, but the parseHTMLUnsafe workaround stays while Chrome 150 to 156 are above the low bar (docs/decisions.md)."
fi

if [ "$act" = 0 ]; then
  echo "Nothing released yet: every workaround stays."
  exit 0
fi
exit 2
