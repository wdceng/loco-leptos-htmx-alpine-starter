# Adding web push

The starter has no push. This is how to add it to an app built on it, the
way an islands app on this stack has it (its `docs/architecture/push.md`),
with the mistakes it made on the way.

## Before you start

- **A push belongs to a signed-in browser,** a cookie session
  (`src/models/sessions.rs`): the service worker asks the site what to say
  with that person's session, and signing out of the browser stops it.
- **A page for the switch,** the account page.
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
   kept: every subscription is bound to it. It prints the `PUSH_KEY`:

   ```bash
   openssl ecparam -genkey -name prime256v1 -noout | openssl pkcs8 -topk8 -nocrypt -outform DER | base64 | tr '+/' '-_' | tr -d '=\n'
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
   `worker-src 'self'`, pinned in `tests/config.rs`. On a gated host its
   `…/push/latest` goes into the rule `Paths without a browser: skip
   challenge` (`staging.md`).
5. **The switch,** an island on the app's account page. It says what it
   does: busy while the browser asks (no second tap), a line where the
   browser refused, a line where the browser has no notifications at all,
   and a sample notification once on.
6. **Sign-out** of a browser stops that browser's pushes, the person's
   other browsers ringing on.
7. **The badge (optional):** the count waiting on the app's icon
   (`navigator.setAppBadge`), set by the service worker from the answer it
   shows and kept right by the page at every look; cleared at none and at
   sign-out. Only where the site runs as an installed app: an iPhone's
   home-screen app, Safari's in the Mac's Dock, an installed Chrome or
   Edge.
   - web-sys has no binding: the island calls it by name (`Reflect`).
   - The count as a plain number: a BigInt is refused.
   - The promise awaited, so a refusal stays out of the console, which the
     browser tests fail on.
8. **The tests:** request tests for the routes and the choice of devices,
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
- **The worker's fetch past the gate.** On a host behind Cloudflare's
  challenge the worker gets the challenge, not the answer, and has no
  person to pass it: its path went into the skip rule's list (2026-10-30).
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
- **The app's webviews have no Push API and no service worker** (the Tauri
  app, as an islands app on this stack found on both phones): Web Push
  can't reach it. Where the app is to be rung too:
  - Firebase Cloud Messaging's HTTP v1 API beside Web Push, one sender
    and one list of devices with a transport column;
  - `tauri-plugin-fcm`, which hands over a Firebase token on both phones,
    Firebase passing the iPhone's on to Apple's service. Plugins that hand
    over the iPhone's raw APNs token need a second sender and a key;
  - the push carries its words: no service worker asks the site for them;
  - the site may call only the plugin's commands, through one capability
    naming the site's origin, called from one place in its WASM;
  - the iPhone's: `aps-environment`, `UIBackgroundModes`
    `remote-notification`, and the team's APNs key uploaded to Firebase;
    Android's compiles against the app's own Android SDK;
  - the Simulator takes no push: the plugin won't register there;
  - a tapped push opens its page on Android through `MainActivity.kt`; on
    the iPhone the plugin hands the tap to nobody, so the app opens at its
    start until it gets a Swift hook of its own.
