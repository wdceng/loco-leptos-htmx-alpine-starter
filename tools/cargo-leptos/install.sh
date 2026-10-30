#!/usr/bin/env bash
# Installs cargo-leptos with the two fixes that lazy islands with hashed file
# names need, until a release has both (docs/development.md, "Lazy islands"):
#   1. cargo-leptos PR #691 (issue #690): the main .js/.wasm keep stale names
#      after the split loader's name is written into them. Fixed at REV,
#      merged after the 0.3.11 release.
#   2. wasm-split-prototype issue #79: the loader's imports come out in
#      random order, so every build gets new names. Fixed in
#      wasm_split_cli_support 0.2.4.
# Without `--locked` on purpose: REV's own Cargo.lock holds
# wasm_split_cli_support 0.2.3, the unsorted one, while its Cargo.toml's
# "0.2.3" lets 0.2.4 in. The same install as the other apps on this stack,
# one cargo-leptos a machine. Run it again after this script changes;
# tools/cargo-leptos/check.sh says whether you need to.
set -euo pipefail

REV=0fb3de8665b1cd345a99fda8fd9d669bce57821c   # PR #691's last commit
HERE="$(cd "$(dirname "$0")" && pwd)"

cargo install --force --git https://github.com/leptos-rs/cargo-leptos --rev "$REV" cargo-leptos

"$HERE/check.sh"
