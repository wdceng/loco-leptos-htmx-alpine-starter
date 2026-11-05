# Security

What the template defends against, what was reviewed, what it accepts, and
what an app must add as it grows. How each layer works is in
`architecture.md`, "Security". How to report a hole: `SECURITY.md`.

**Keep this page true.** A change to auth, sessions, headers or the CSP
updates it in the same commit.

## What it defends against

| Threat | Defence | Where | Test |
|---|---|---|---|
| Script injection (XSS) | Leptos escapes every value; a CSP with a nonce per page, no `'unsafe-inline'`, no `'unsafe-eval'` | `src/render.rs`, `config/*.yaml` | `tests/config.rs`, `tests/requests/security_headers.rs` |
| Script gadgets (injected HTML turned into code by a library that reads attributes) | htmx without `hx-on`, Alpine's CSP build only, so no attribute is ever `eval`ed; no project JavaScript but the one glue file; the libraries vendored byte for byte from npm, checked before they came in | `src/views/layout.rs`, `public/vendor/` | `tests/config.rs`, `src/paths.rs` |
| SQL injection | `query!` and Sea-ORM, parameters only | `src/sql.rs`, `src/models/` | the compiler, `.sqlx/` |
| Clickjacking | `frame-ancestors 'none'`, `X-Frame-Options: DENY` | `config/*.yaml` | `tests/config.rs` |
| Downgrade, sniffing | HSTS with preload, `nosniff` | `config/*.yaml` | `tests/config.rs` |
| Cross-origin leaks | COOP, COEP, CORP, `Referrer-Policy` | `config/*.yaml` | `tests/config.rs` |
| Brute force, mail flooding | Rate limits per visitor, per /64 for IPv6; a stricter bucket shared by the auth forms' posts (sign-in, sign-up, forgotten password, a new welcome mail); no bucket state on an allowed answer, so a bot can't pace itself under the limit | `src/middleware/rate_limit.rs` | `tests/requests/rate_limit.rs` |
| Which emails have accounts | Every refused sign-in is the same page after one password check, a dummy one for an unknown email; a taken email signs up like a new one; the forgotten-password and new-mail forms answer every address alike | `src/controllers/login.rs`, `register.rs`, `forgot.rs`, `verify.rs`, `src/hashing.rs` | `tests/requests/login.rs`, `register.rs`, `forgot.rs`, `verify.rs` |
| Someone else's address | An unverified account can't sign in; the welcome mail's link opens a page whose button verifies, so a mail scanner that opens it verifies nothing; the link works for a day | `src/controllers/verify.rs`, `src/models/users.rs` | `tests/requests/verify.rs` |
| One mailbox, two accounts | An address is stored and looked up lowercase | `src/models/users.rs` | `tests/requests/register.rs` |
| Weak passwords | 8 to 128 characters, everywhere one is set | `src/models/users.rs` | `tests/requests/register.rs`, `reset.rs` |
| Hashing as a denial of service | Argon2id off the async threads, four at once | `src/hashing.rs` | `src/hashing.rs` |
| A reset token leaking | Works once, for 30 minutes; its page sends `no-referrer` and `no-store`; the log names its route, never the token | `src/controllers/reset.rs`, `src/middleware/request_log.rs` | `tests/requests/reset.rs`, `src/middleware/request_log.rs` |
| A token that can't be revoked | None: Loco's JWT API is unmounted, and no config carries a JWT key | `src/app.rs`, `config/*.yaml` | `tests/requests/not_found.rs`, `tests/config.rs` |
| An admin's password stolen | A second step after the password: six digits by mail, no session until they're right; the code kept as its digest with the cookie's secret, so a copy of the database reads none; 10 minutes, 5 tries across new codes, a new one at most once a minute, the code never in the mail's subject; the wrong codes counted per account across sign-ins (`auth_events`), so a right password brings no fresh tries: after 5 the account waits before a code is checked or mailed, a second doubling to 5 minutes, and the person is mailed at the fifth; a new password (`Session::after_new_password`) or the account's deletion ends any sign-in waiting | `src/models/sign_ins.rs`, `src/models/auth_events.rs`, `src/controllers/login.rs` | `tests/requests/sign_in_code.rs` |
| A member on an admin's page | The page takes `Admin`: nobody signed in goes to the sign-in, another role gets a 403; the role read from the database on every request, so a change holds at once; a sign-up is a member, an admin given from the command line alone | `src/controllers/signed.rs`, `src/models/roles.rs`, `src/tasks/user_create.rs` | `tests/requests/roles.rs`, `tests/tasks/user_create.rs` |
| A session left open elsewhere, or taken from an unlocked computer | The account page lists every browser signed in, by its `User-Agent` and last use, each signed out on its own or all but this one; a new password needs the current one and signs out every other browser; deleting the account asks for the password once the last proof is older than 15 minutes (`Session::proven`) | `src/controllers/account.rs`, `src/models/sessions.rs` | `tests/requests/account.rs`, `tests/models/sessions.rs` |
| A stolen session | A row per sign-in, closed at sign-out and by a new password; 12 hours; the row keeps the secret's SHA-256, so a copy of the database signs nobody in; the cookie `HttpOnly`, `SameSite=Lax`, `Secure` and `__Host-` over HTTPS; the page that names who is signed in `no-store` | `src/models/sessions.rs`, `src/cookies.rs`, `src/secret.rs`, `src/controllers/login.rs` | `tests/models/sessions.rs`, `tests/requests/login.rs`, `tests/requests/reset.rs` |
| Another site's page posting our forms (CSRF) | Every form's target refuses a post the browser says came from another site (`Sec-Fetch-Site`), before anything else; the cookie isn't sent with another site's post (`SameSite=Lax`) | `src/controllers/cross_site.rs` | `tests/requests/login.rs`, `register.rs`, `forgot.rs`, `reset.rs`, `verify.rs` |
| Big or slow requests | 64 KB body limit, 15 s timeout | `config/*.yaml` | `tests/config.rs` |
| A file uploaded to hurt the server | Only the avatar's route takes more than 64 KB, 10 MB at most, on the auth bucket; decoding is the check, a JPEG, PNG or WebP at most 6000 pixels a side, off the async threads; only the JPEG the server made is kept, named by its hash, so no name a visitor sends reaches the disk; only such names are served | `src/uploads.rs`, `src/controllers/account.rs`, `src/controllers/uploads.rs` | `src/uploads.rs`, `tests/requests/account.rs` |
| Bypassing the CDN | Firewall to Cloudflare only, the origin key, dropped from Caddy's log | `production.md`, "Caddy's log" in the three deploy docs | by hand |
| The box itself | SSH from one address, by key alone, three tries, no forwarding; the kernel drops faked senders and ICMP redirects and hides its addresses; one-time jobs off; the journal 30 days and at most 1 GB | `production.md`, "Firewall", "The box", "Log retention" | by hand (`journalctl -u ssh`, `sysctl`) |
| Another service on the box | Each unit's database its own user's alone (`0700`, `0600`); no capabilities, an ordinary service's system calls and sockets, no link-local address (a cloud's metadata service); the app on loopback even without its `BINDING`; Caddy's admin API a socket only `caddy` and root reach, so no process reads the origin key or loads a config; Caddy itself behind the same walls, its certificates the only thing it writes | the units, "Caddy's admin socket" and "Caddy's hardening" in `production.md`, `staging.md`, `dev-server.md`; `config/*.yaml` | `tests/config.rs`; by hand (`stat`, `systemctl is-active`, `ss -ltn`) |
| Secrets when deployed | Production's mail account required, and refused at boot while empty or a placeholder, its security contact too; no signing key anywhere to leak | `src/deploy_checks.rs`, `config/*.yaml` | `src/deploy_checks.rs`, `tests/config.rs` |
| Personal data in logs | No email address in any log line, and no token: a request is logged by its route; the journal keeps 30 days | `src/controllers/`, `src/middleware/request_log.rs`, `production.md` | `src/middleware/request_log.rs`, by hand |
| The welcome mail as someone's message | Its subject is the app's, never the name the visitor typed | `src/mailers/auth.rs` | `tests/requests/register.rs` |
| A link in the app opening a file or another app | The app hands the system only the schemes it names; any other goes nowhere | `src-tauri/src/lib.rs` | `src-tauri/src/lib.rs` |
| Known-vulnerable crates | `cargo audit` as the production deploy's first gate (`production.md`), and in CI when Actions is on | `.cargo/audit.toml` | the deploy chain |

## The review of 2026-10-18

| # | Finding | Status |
|---|---|---|
| 1 | Login messages differ by case | **Not a finding:** Loco's `unauthorized` always sends the same body; the message only reaches the log |
| 2 | Login timing tells whether an email has an account | **Fixed:** a dummy hash, built at boot |
| 3 | An unverified account can log in | **Fixed:** refused, as Loco's own comment on `verify` promises |
| 4 | Seven-day tokens, no revocation | **Partly fixed:** one day. Revocation needs cookie sessions (below) |
| 5 | The reset link leads to a 404 | **Fixed:** the `/reset` page |
| 6 | Email addresses in the logs | **Fixed:** the `pid` instead; 30 days of journal |
| 7 | No lockout across addresses | **Accepted** (below) |
| 8 | `/_health` and `/_readiness` are public | **Accepted** (below) |
| 9 | No `security.txt` | **Fixed:** `/.well-known/security.txt` |
| 10 | Two advisories ignored in `cargo audit` | **Accepted**, each with its reason in `.cargo/audit.toml` |
| 11 | Registration timing tells whether an email is taken | **Fixed:** hashed before the duplicate check |
| 12 | No password rules | **Fixed:** 8 to 128 characters |
| 13 | Argon2 on the async threads | **Fixed:** `src/hashing.rs` |

## The review of 2026-10-31

As an attacker would: every route, the token flows, the logs, the
configs, the app, the dependencies.

| # | Finding | Status |
|---|---|---|
| 1 | Account pre-hijack: anyone registers someone's address with a password of their own, and a mail scanner's GET on the verification link verifies it | **Fixed:** the mail links to `/verify`, whose button posts the token; the mail says to ignore it without a sign-up. A press on a mail nobody expected is accepted (below) |
| 2 | The magic link is spent by a GET, which answers with the JWT: a mail scanner that follows it first gets the token, the person a refusal | **Fixed** (2026-11-02): gone with Loco's auth API, unmounted; no page sends a magic link |
| 3 | Live tokens in the journal: Loco's logger wrote the whole address, the reset link's query included | **Fixed:** `src/middleware/request_log.rs` logs the route's pattern or the path, never a query |
| 4 | Addresses were case-sensitive: one mailbox, two accounts | **Fixed:** lowercase, stored and looked up |
| 5 | The welcome mail's subject carried the visitor's name, to any address | **Fixed:** the app's name |
| 6 | Staging and the dev server booted on a JWT secret in git when their secrets file had none | **Fixed:** no default, and refused empty or `replace_me` everywhere deployed; since 2026-11-02 no config has a JWT key at all |
| 7 | A verification link never expired | **Fixed:** a day; resending the mail gives a new one |
| 8 | A password reset left a pending magic link open | **Fixed:** the reset clears it |
| 9 | The app handed every unknown scheme to the system (`file:`, `intent:`) | **Fixed:** the schemes it names; the rest go nowhere |
| 10 | An API key per account that no route takes | **Accepted** (below) |
| 11 | `unsafe-libyaml` unmaintained, through Loco's YAML | **Accepted:** a warning, no advisory; re-check when Loco moves |
| 12 | The vendored htmx and Alpine | **Checked:** no advisory found. Checked again before going live (below) |
| 13 | Every allowed answer carried `x-ratelimit-limit` and `x-ratelimit-remaining`: a bot reads its tries left and paces itself just under the limit | **Fixed** (2026-11-02, from an islands app on this stack): only the 429 says how long to wait |

## Accepted risks

- **No lockout per account.** Limits are per visitor: ten posts to the
  auth forms at once, then one every 30 s, about 120 guesses an hour from
  one address.
  A lockout per account would let anyone lock a victim out.
- **Public health routes.** Loco's `/_health` and `/_readiness` say whether
  the database and queue are up. Behind Cloudflare and the origin key, only
  Cloudflare reaches them. Without a CDN, restrict them in Caddy.
- **`/up` answers any origin.** The app's start page reads it from
  outside the site (`Access-Control-Allow-Origin: *`). It says `ok` and
  nothing else, and no cookie is sent with it.
- **A few milliseconds in the sign-up, `/forgot` and a new welcome
  mail.** A real account, or a new one, costs a database write and a
  queued mail; an unknown or taken one costs neither. The mail itself is
  sent in the background.
- **Two ignored advisories.** The `rsa` timing issue (no RSA key exists
  here) and unmaintained compile-time crates. Re-check when loco-rs or
  Leptos changes them.
- **An API key per account, unused.** Loco's starter mints one at
  registration (`lo-<uuid>`); no route takes it. An app that offers API
  keys stores them hashed and lets a user replace one.
- **A press on a welcome mail nobody expected.** Someone who presses
  "Verify my email" on it verifies an account another registered with
  their address and a password of their own. The mail and the page say to
  ignore it; "forgot password" takes the account back. A new password at
  verification would close it, at the cost of every new user typing one
  twice.
- **A proxy's own logs.** The app logs no token; a proxy that logs
  addresses (a `log` line in Caddy's block, a log push at Cloudflare) holds
  them. The reset token works once, for 30 minutes.

## When your app grows

Each of these brings its own risks. The template has none of them yet.

- **A magic link that signs in:** it opens a page whose button posts the
  token, as `/verify` does, and the session starts there. Loco's
  `GET /api/auth/magic-link/{token}`, unmounted with its API, was spent by
  a mail scanner that followed it first, and answered it the JWT (the
  review of 2026-10-31, #2).
- **A client that wants tokens** (a phone app of its own, another
  server): mounting Loco's auth API again brings back a JWT key in every
  config, required and refused while a placeholder, and tokens that can't
  be revoked before they expire.
- **A second factor:** passkeys or an authenticator app, recovery codes.
  Passkeys inside the Android app, as an islands app on this stack found
  on a real phone, need all of:
  - the WebView's passkeys switched on in `MainActivity.kt`
    (`WEB_AUTHENTICATION_SUPPORT_FOR_APP`);
  - `/.well-known/assetlinks.json` naming the package and each key the
    builds are signed with, Play's app signing key for an app installed
    from Google Play among them, with **both** relations,
    `common.handle_all_urls` and `common.get_login_creds`: the second alone
    is refused, "RP ID cannot be validated" (50152);
  - the relying party accepting the app's own origin beside the site's,
    `android:apk-key-hash:` and the signing key's SHA-256 in base64url, one
    per key: the WebView signs with it, "origin does not match" otherwise;
  - a Google account on the phone, an emulator's too.

  The iPhone's, passkeys and Password AutoFill both, need (Apple's
  "Supporting passkeys", as that app found on a real iPhone):
  - a paid Apple team: a free one has no associated domains;
  - the app's entitlement naming the site, `webcredentials:<host>`;
  - the site naming the app in `/.well-known/apple-app-site-association`,
    `<team id>.<bundle id>`, both read from `tauri.conf.json` alone, so it
    names the app that is built. Served on every environment: it names the
    app, not a signing key. Apple's CDN reads it when the app is
    installed.
- **Web push:** on the sessions (`src/models/sessions.rs`), then `push.md`, the steps
  and the mistakes an islands app on this stack made adding it.
- **Your own htmx and Alpine attributes:** the libraries are in the
  template; what you write with them is yours. Addresses from `paths`,
  selectors and Alpine expressions as named constants, each Alpine
  expression proven in a browser under the CSP. A form that changes
  something calls `cross_site::refused` first, as every form here does.
- **Breached passwords:** the Have I Been Pwned range API, if the app wants
  NIST's full advice. An outbound call and a dependency: the app's choice.
- **A slowdown per account** for valuable accounts: a growing delay after
  failures, never a hard lock.
- **Uploads:** size and type limits, files stored outside the site folder,
  served with `Content-Disposition`.
- **Payments, health or other sensitive data:** their own rules apply; a
  pen test before going live.

## Before a pen test or going live

Nothing left of the template's domain, the security contact included:

```bash
grep -n "example\.com" config/production.yaml
```

No new advisories:

```bash
cargo audit
```

The nonce CSP is there:

```bash
curl -sI https://<domain>/ | grep -i content-security-policy
```

The real contact:

```bash
curl -s https://<domain>/.well-known/security.txt
```

- No advisory open for the vendored htmx and Alpine (`public/vendor/`):
  GitHub's advisories for `bigskysoftware/htmx` and `alpinejs/alpine`.
- The firewall and the origin key are set up (`production.md`).
- The journal keeps 30 days (`production.md`, "Log retention").
- GitHub's private vulnerability reporting is switched on, if the
  repository is public (`SECURITY.md` points to it).
