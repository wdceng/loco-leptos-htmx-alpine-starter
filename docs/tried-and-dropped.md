# Tried and dropped

What was tried and reverted, and why. Nothing here is suggested again
without a new reason.

- **A wrapper giving every idle callback a 2 s timeout**
  (`public/js/island-loader-timeout.js`, 2026-10-19, dropped the same
  day). It worked, loaded before Leptos's loader, but capped every
  `requestIdleCallback` on the page, htmx's and Alpine's too, and leaned
  on script order. The fork pin fixes the loader alone, as the islands
  app on this stack does (`decisions.md`).
- **`screens.sh --together`** (2026-10-29, dropped the same day): every
  device booted at once, then the pictures. All eight of an islands app
  on this stack took some 35 GB and overloaded a Mac. One device at a
  time is the only way.
- **The Tailwind extension's lint off** (2026-10-22, on again
  2026-10-28). Off, nothing in the editor flagged a class set twice or a
  bad `@apply`, and VS Code's own CSS parser still marked `source()` on
  the import as an error. Now on, the stylesheet in the extension's mode
  (`decisions.md`).
- **`cross` for the Linux server binary** (until 2026-10-21, its image
  pinned by digest in `Cross.toml`). It needed Docker and a 2.69 GB amd64
  image, emulated on Apple Silicon: 9m 39s for the fast profile where
  `cargo zigbuild` takes 2m 48s, and the binary needed the image's glibc
  (2.39). Replaced by `cargo zigbuild` (`decisions.md`).
- **A local environment named `dev-local`** (2026-10-06, back 2026-10-07).
  Loco falls back to `development` when `LOCO_ENV` is unset, so the new
  name needed `export LOCO_ENV=dev-local` in every shell for a name alone.
- **`viewport-fit=cover` with safe-area padding on the body** (dropped
  2026-10-05, back 2026-10-27 in another shape, `decisions.md`). Without
  cover, iOS keeps the page out of the notch and the home bar by itself.
  With it, WebKit sometimes updated the insets only on the next scroll
  after a rotation, leaving the page off-centre. Not seen in the iOS 27.2
  measurements that brought it back, but not tested for either: check a
  turn on a phone.
- **A house style in the template** (2026-10-04, reverted the same day):
  shared shape roles (squircles), icon rules, the Inter font. The template
  is public; an app brings its own look.
- **leptosfmt** (2026-10-15, version 0.1.33). It rebuilds `view!` from
  tokens, where comments don't exist, and puts them back by line: a comment
  inside a `class=format!(…)` was lost, and a closing tag moved into
  another comment. `cargo fmt` stays; `view!` is formatted by hand.
- **"Typo warnings" from the Tailwind extension** (claimed, checked
  2026-10-15). It has no check for a class that doesn't exist; its lints
  are conflicts and `@apply` misuse. A class without a hover preview is the
  only hint, and a misspelt class shows on the page.
- **Plain released cargo-leptos with `--split` and hashed names**
  (2026-10-15). 0.3.11 leaves the main script's name stale after a deploy
  (#690) and renames everything on every build (wasm-split-prototype #79).
  The patched build in `tools/cargo-leptos/` replaces it until both fixes
  are released.
