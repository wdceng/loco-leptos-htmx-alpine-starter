# UX audit: the starter's own list

Read by `/ux-audit`. The starter has no pages of its own beyond sign-in and the account: an app built on it adds its journeys here.

## Users

A visitor on a phone, a visitor on a desktop, someone signing up and back in, an admin with the mailed code, someone on a slow network.

## Journeys

1. Home and navigation: the header, the footer, the skip link, a move in place (`#page`, boosted).
2. Signing up: register, "check your mail", the welcome mail, verify (the button, done, expired), the mail again.
3. Signing in: the password, the mailed code, the wait after wrong codes, signing out.
4. A forgotten password: the form, the mail, the reset (the form, done, expired).
5. The account page: its browsers, a new password, the password asked again before a sensitive step, the avatar, the account deleted (`Dialog`).
6. The notices: not found, an error, 429 with its wait, a 403 for another role.
7. The mails: subject, text and HTML (`src/views/mail.rs`), in a phone's mail app.
8. The app around the page: the start page and offline (`src-tauri/offline/index.html`), a link handed to the system (`tel:`, `mailto:`, another site), the status bar, the safe areas.

## Sizes and platforms

A phone upright and on its side, a tablet, 768 to 1023px (`md` to `lg`), a desktop; light and dark; the iOS and Android apps. Android's three buttons: no back swipe with them.

## The record

- `docs/look.md` (the parts, what floats over what), `docs/one-job.md`, `docs/architecture.md`.
- Not reported again: `docs/tried-and-dropped.md`, a dated call in `docs/decisions.md`.
- The safe areas: `public/js/safe-area.js`. Tap targets: `min-h-11`. Motion: `BUTTON`'s busy fade, Reduce Motion in `style/tailwind.css`.
- The words: `PageMeta::lang`, `src/views/error.rs`. A count: `Badge`. What a tap shows before the answer: `/review`, pass 17.
- The WASM has no budget yet (`.claude/review.md`, pass 8): its size read from a release build already there, none built for the audit.

## Rules

- English a visitor reads; an app built on it names its language here.

## Screens

- The lanes' kept pages (`target/e2e/<lane>/failures/`) and the app's pictures (`src-tauri/screens.sh`, only on the maintainer's word): the `e2e` skill.
- The hand checks: `docs/testing.md`.
