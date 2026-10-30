# Adding web push

The starter has no push. This is how to add it to an app built on it, the
way an islands app on this stack has it (its `docs/architecture/push.md`),
with the mistakes it made on the way.

## Before you start

- **Cookie sessions first** (`security.md`, "When your app grows"). A push
  belongs to a signed-in browser: the service worker asks the site what to
  say with that person's session, and signing out of the browser stops it.
  The starter's auth is a JWT JSON API with no browser session.
- **A page of the app's own** for the switch, an account page. The starter
  has none.
- **Who gets pushes, and for what.** One event first (a new order, a held
  post), to the people who act on it alone.

## The shape that works

- **The push carries nothing.** It only wakes the device's service worker,
  which asks the site (`GET …/push/latest`, with the session) what to say
  and shows it. Nothing of the app passes through Apple's, Google's or
  Mozilla's push service, and the server needs no payload encryption.
- **Only a signed token:** VAPID (RFC 8292), an ES256 JWT for the push
  service's origin, signed with the site's key.
- **One notification at a time:** each kind under its own `tag`, so ten
  orders collapse into one, and a held post never replaces an order.
- **Off without a key.** No key set, no push: the switch keeps to the tab's
  own notification.

## The steps

1. **The key,** a P-256 key, PKCS#8 in base64url, one line. Made once and
   kept: every subscription is bound to it.

   ```bash
   openssl ecparam -genkey -name prime256v1 -noout | openssl pkcs8 -topk8 -nocrypt -outform DER | base64 | tr '+/' '-_' | tr -d '=\n'   # PUSH_KEY
   ```

   `settings.push.key: '<%= get_env(name="PUSH_KEY") %>'`, no default on
   staging and production; `default=""` elsewhere, which is off. Then add
   `PUSH_KEY` to all three `secrets.*.env.example` files.
2. **The devices table:** the push address alone, no keys, and the browser
   it was switched on in (a row of the session's browser cookie), deleted
   with it. A migration (the `migration` skill), queries in `sqlx::query!`.
3. **The sender:** built at boot where the key is set. Each push within
   5 s, all of them within 8 s, at once (`JoinSet`), in a task of its own
   after the event is kept, so a slow push service never holds it up. A
   device its service answers 404 or 410 for is forgotten. `reqwest` on the
   tree's `rustls`.
4. **The service worker:** served by a route with a CSP of its own
   (`connect-src 'self'`), never from `public/`. Every page's CSP gets
   `worker-src 'self'`, pinned in `tests/config.rs`.
5. **The switch,** an island on the app's account page. It says what it
   does: busy while the browser asks (no second tap), a line where the
   browser refused, a line where the browser has no notifications at all,
   and a sample notification once on.
6. **Sign-out** of a browser stops that browser's pushes, the person's
   other browsers ringing on.
7. **The tests:** request tests for the routes and the choice of devices,
   a unit test of the token, browser tests with a key of each lane's own,
   and the CSP line in `tests/config.rs`.

## Gotchas

- **`PUSH_KEY` in every example secrets file.** A server set up from files
  that lacked it never started: staging and production refuse to boot
  without it (2026-10-30). `tests/config.rs` now checks every variable a
  deployed config requires is in its example file.
- **Every line of a secrets file is a comment or `NAME=value`.** The deploy
  docs load it with `. ./secrets.env`, and one comment that lost its `#`
  stopped every command after it (2026-10-30). The same test checks it.
- **The service worker not under `public/`.** The files there are served
  under the files' CSP, which stopped it; and not under `src/`, which
  Tailwind reads.
- **A tap must show something.** A switch that did its work silently was
  taken for broken.
- **Sign-out must reach a device switched on without the browser cookie
  too.** One rang on after its person signed out.
- **jsonwebtoken 11 for the push's ES256, beside Loco's 10.** Version 10's
  `rust_crypto` pulled `crypto-bigint` 0.5 and its advisory; Loco keeps 10
  for its own HMAC tokens (`.cargo/audit.toml`).
- **A key of each browser lane's own** (`E2E_PUSH_KEY`, made by `run.sh`):
  a key in the developer's shell must never reach the tests.
- **An iPhone or iPad takes pushes only in the home-screen app,** iOS 16.4
  and up. In Safari the switch says how to add the app.
- **The app's webviews likely have no push and no `Notification`** (the
  Tauri app, unverified): there the switch stays hidden.
