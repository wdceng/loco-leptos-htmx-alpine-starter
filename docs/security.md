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
| Brute force, mail flooding | Rate limits per visitor, per /64 for IPv6; a stricter bucket on `/api/auth` | `src/middleware/rate_limit.rs` | `tests/requests/rate_limit.rs` |
| Which emails have accounts | Every refused login is the same 401 after one password check, a dummy one for an unknown email; a taken email registers like a new one | `src/controllers/auth.rs`, `src/hashing.rs` | `tests/requests/auth.rs` |
| Someone else's address | An unverified account can't log in; the welcome mail's link opens a page whose button verifies, so a mail scanner that opens it verifies nothing; the link works for a day; a magic link verifies, as it reached the inbox | `src/controllers/verify.rs`, `src/controllers/auth.rs`, `src/models/users.rs` | `tests/requests/verify.rs`, `tests/requests/auth.rs` |
| One mailbox, two accounts | An address is stored and looked up lowercase | `src/models/users.rs` | `tests/requests/auth.rs` |
| Weak passwords | 8 to 128 characters, everywhere one is set | `src/models/users.rs` | `tests/requests/auth.rs` |
| Hashing as a denial of service | Argon2id off the async threads, four at once | `src/hashing.rs` | `src/hashing.rs` |
| A reset token leaking | Works once, for 30 minutes, and ends a pending magic link; its page sends `no-referrer` and `no-store`; the log names its route, never the token | `src/controllers/reset.rs`, `src/middleware/request_log.rs` | `tests/requests/reset.rs`, `src/middleware/request_log.rs` |
| A stolen token | JWTs live one day | `config/*.yaml` | `tests/config.rs` |
| Big or slow requests | 64 KB body limit, 15 s timeout | `config/*.yaml` | `tests/config.rs` |
| Bypassing the CDN | Firewall to Cloudflare only, the origin key | `production.md` | by hand |
| Secrets when deployed | No JWT secret from git: required, and refused at boot while empty or a placeholder, everywhere deployed; production's mail account and security contact too | `src/deploy_checks.rs`, `config/*.yaml` | `src/deploy_checks.rs`, `tests/config.rs` |
| Personal data in logs | No email address in any log line, and no token: a request is logged by its route; the journal keeps 30 days | `src/controllers/auth.rs`, `src/middleware/request_log.rs`, `production.md` | `src/middleware/request_log.rs`, by hand |
| The welcome mail as someone's message | Its subject is the app's, never the name the visitor typed | `src/mailers/auth.rs` | `tests/requests/auth.rs` |
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
| 2 | The magic link is spent by a GET, which answers with the JWT: a mail scanner that follows it first gets the token, the person a refusal | **Open:** its page needs cookie sessions (below) |
| 3 | Live tokens in the journal: Loco's logger wrote the whole address, the reset link's query included | **Fixed:** `src/middleware/request_log.rs` logs the route's pattern or the path, never a query |
| 4 | Addresses were case-sensitive: one mailbox, two accounts | **Fixed:** lowercase, stored and looked up |
| 5 | The welcome mail's subject carried the visitor's name, to any address | **Fixed:** the app's name |
| 6 | Staging and the dev server booted on a JWT secret in git when their secrets file had none | **Fixed:** no default, and refused empty or `replace-me` everywhere deployed |
| 7 | A verification link never expired | **Fixed:** a day; resending the mail gives a new one |
| 8 | A password reset left a pending magic link open | **Fixed:** the reset clears it |
| 9 | The app handed every unknown scheme to the system (`file:`, `intent:`) | **Fixed:** the schemes it names; the rest go nowhere |
| 10 | An API key per account that no route takes | **Accepted** (below) |
| 11 | `unsafe-libyaml` unmaintained, through Loco's YAML | **Accepted:** a warning, no advisory; re-check when Loco moves |
| 12 | The vendored htmx and Alpine | **Checked:** no advisory found. Checked again before going live (below) |

## Accepted risks

- **No lockout per account.** Limits are per visitor: ten auth calls at
  once, then one every 30 s, about 120 guesses an hour from one address.
  A lockout per account would let anyone lock a victim out.
- **Public health routes.** Loco's `/_health` and `/_readiness` say whether
  the database and queue are up. Behind Cloudflare and the origin key, only
  Cloudflare reaches them. Without a CDN, restrict them in Caddy.
- **`/up` answers any origin.** The app's start page reads it from
  outside the site (`Access-Control-Allow-Origin: *`). It says `ok` and
  nothing else, and no cookie is sent with it.
- **A few milliseconds in `forgot`, `magic-link` and `resend`.** A real
  account costs a database write and a queued mail; an unknown one costs
  neither. The mail itself is sent in the background.
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

- **Cookie sessions** (for logout, longer sessions, a staff side): a
  session store, `HttpOnly`, `Secure`, `SameSite=Lax` cookies, and **CSRF
  protection** on every form, an origin check at least. An islands app on
  this stack has both.
- **A magic link that signs in,** once cookie sessions are in: it opens a
  page whose button posts the token, as `/verify` does, and the session
  starts there. Today's `GET /api/auth/magic-link/{token}` is spent by a
  mail scanner that follows it first, and answers it the JWT (the review of
  2026-10-31, #2).
- **A second factor:** passkeys or an authenticator app, recovery codes.
- **Web push:** cookie sessions first (above), then `push.md`, the steps
  and the mistakes an islands app on this stack made adding it.
- **Your own htmx and Alpine attributes:** the libraries are in the
  template; what you write with them is yours. Addresses from `paths`,
  selectors and Alpine expressions as named constants, each Alpine
  expression proven in a browser under the CSP. A form that changes
  something needs CSRF protection once cookies are in (above).
- **Breached passwords:** the Have I Been Pwned range API, if the app wants
  NIST's full advice. An outbound call and a dependency: the app's choice.
- **A slowdown per account** for valuable accounts: a growing delay after
  failures, never a hard lock.
- **Uploads:** size and type limits, files stored outside the site folder,
  served with `Content-Disposition`.
- **Payments, health or other sensitive data:** their own rules apply; a
  pen test before going live.

## Before a pen test or going live

```bash
grep -n "example\.com" config/production.yaml   # nothing left, the security contact included
cargo audit                                     # no new advisories
curl -sI https://<domain>/ | grep -i content-security-policy   # the nonce CSP is there
curl -s https://<domain>/.well-known/security.txt              # the real contact
```

- No advisory open for the vendored htmx and Alpine (`public/vendor/`):
  GitHub's advisories for `bigskysoftware/htmx` and `alpinejs/alpine`.
- The firewall and the origin key are set up (`production.md`).
- The journal keeps 30 days (`production.md`, "Log retention").
- GitHub's private vulnerability reporting is switched on, if the
  repository is public (`SECURITY.md` points to it).
