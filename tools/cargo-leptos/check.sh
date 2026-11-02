#!/usr/bin/env bash
# Says whether the installed cargo-leptos has both fixes that lazy islands
# with hashed file names need (tools/cargo-leptos/install.sh). Exits 1 when
# it hasn't, so a deploy chain that starts with it stops before building.
# It sees the commit, not which wasm_split_cli_support the install resolved:
# install.sh's unlocked install is what brings 0.2.4.
set -euo pipefail

REV=0fb3de8665b1cd345a99fda8fd9d669bce57821c   # keep equal to install.sh

line="$(cargo install --list | grep '^cargo-leptos ' || true)"
if [ -z "$line" ]; then
  echo "cargo-leptos is not installed: run tools/cargo-leptos/install.sh" >&2
  exit 1
fi

# A git install names its address and commit:
# `cargo-leptos v0.3.11 (https://github.com/leptos-rs/cargo-leptos?rev=…#0fb3de86):`.
case "$line" in
  *github.com/leptos-rs/cargo-leptos*"#${REV:0:8}"*) ;;
  *)
    echo "cargo-leptos is not PR #691's commit ($line): split builds with hashed" >&2
    echo "names get stale or changing names. Run tools/cargo-leptos/install.sh" >&2
    exit 1
    ;;
esac
echo "cargo-leptos from PR #691's commit ${REV:0:8}: both fixes in"
