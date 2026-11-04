# One job, one implementation

Every job the app does has one component, constant or helper. Something
that does an existing job is built from it, never from classes or code of
its own, and a change to how the job looks or works is made there, for
every use. Look a job up here before adding a component, a constant or a
helper. A new job gets a row when it's built.

Why: code written over many sessions drifts. Without this list the same
button, row or check gets written a second and a third time, each a little
different, and a change reaches only one of them. Two ways for one job is
a **one-job break** (CLAUDE.md, Rule no. 1).

- **How to use it:** find the job, build from its row. A plan names the
  rows it checked.
- **Guarded:** a row marked so has a test in `tests/one_job.rs` that fails
  on the job's pattern anywhere but its home.

## Pages

| Job | Use | Where |
|---|---|---|
| Render a page | `render_page` (CSP nonce, shell, status) | `src/render.rs` |
| Whether a request asks for a page or a plain answer (`Accept` names HTML): the not-found page, the error pages. Guarded | `accepts_html` | `src/render.rs` |
| The document around every page: head, header, footer, skip link | `shell` | `src/views/layout.rs` |
| A page's language, title, description, robots tag, canonical address | `PageMeta` | `src/views/layout.rs` |
| A link preview (Open Graph, X's card): every page's, from the shell | `shell` with `Address`; the picture `OG_IMAGE`, the language `OG_LOCALE` | `src/views/layout.rs` |
| A value each app fills in: `TODO` in it, its file on the checklist. Guarded | "Start a new project"; production refuses a name or description still `TODO` (`still_todo`) | `docs/development.md`, `src/deploy_checks.rs`, `tests/config.rs` |
| The browser tab's title: the page's name, then the app's. `PageMeta::title` holds the page's name alone, empty for home. Guarded | `full_title` | `src/views/layout.rs` |
| A page's heading and the line under it. Guarded | `TITLE`, `LEAD` | `src/views/layout.rs` |
| A page that only tells something: heading, one line, the way home (not found, errors, 429, the reset and verify links' answers, "check your mail"). Guarded | `Notice`; an error's words `wording` | `src/views/layout.rs`, `src/views/error.rs` |
| A dialog over the page, and the step it asks; its Cancel; a step beside a form, never a new tab (`no_page_opens_a_new_tab`) | `Dialog`, `DialogCancel` (a native popover, Cancel owned by no form for Safari 17), `DANGER_BUTTON`, `QUIET_BUTTON`, as the account's deletion | `src/views/parts.rs` |
| A part folded under its heading | `Fold` (`<details>`) | `src/views/parts.rs` |
| A count beside what it counts | `Badge` | `src/views/parts.rs` |
| What floats over what: every `z-` | the scale in `docs/look.md` | `docs/look.md` |
| The app's name, anywhere | `APP_NAME` | `src/views/layout.rs` |
| The site in one sentence (meta description, manifest, llms.txt) | `DESCRIPTION` | `src/controllers/home.rs` |
| The centred content column | `COLUMN` | `src/views/layout.rs` |
| The `<body>` classes | `BODY` | `src/views/layout.rs` |
| The not-found page | `NotFoundPage`, a `Notice` | `src/views/not_found.rs` |
| The 429 page, with the wait filled in | `too_many_requests::document`, `WAIT` | `src/views/too_many_requests.rs` |
| A page a mail's link opens with a token in its address: `no-referrer`, `no-store` | `render_token_page` | `src/render.rs` |
| A page no copy is kept of (`no-store`): one that names who is signed in | `render_private_page` | `src/render.rs` |
| A password reset from the mail's link: the form, done, expired | `ResetForm`, `ResetDone`, `ResetExpired`; the address `reset::PATH` | `src/views/reset.rs`, `src/controllers/reset.rs` |
| Email verification from the welcome mail's link: the button, done, no longer works | `VerifyForm`, `VerifyDone`, `VerifyExpired`; the address `verify::PATH` | `src/views/verify.rs`, `src/controllers/verify.rs` |
| The welcome mail again, with a fresh link: `/verify` without a token | `ResendForm`, `ResendSent`; the post `verify::RESEND` | `src/views/verify.rs`, `src/controllers/verify.rs` |
| Sign in, and who is signed in with the way out | `LoginForm`, `SignedIn`; the addresses `login::PATH`, `login::LOGOUT` | `src/views/login.rs`, `src/controllers/login.rs` |
| The signed-in user's own page: their browsers, a new password, the account deleted | `AccountPage`, `AccountDeleted`; the addresses in `controllers::account` | `src/views/account.rs`, `src/controllers/account.rs` |
| A browser's name, from its `User-Agent` | `label_of`, `label_from` | `src/models/sessions.rs` |
| A sign-in's end, a session opened and its cookie handed over, after the password or its code | `open_session` | `src/controllers/login.rs` |
| A second step after the password: the mailed code, its tries and a new one | `SignIn` (`sign_ins`, the `signin` cookie), `CodeForm`, `AuthMailer::sign_in_code` | `src/models/sign_ins.rs`, `src/controllers/login.rs`, `src/views/login.rs` |
| A code drawn, two digests compared | `secret::drawn`, `secret::same` | `src/secret.rs` |
| What a user may do; a page for one role. Another role's request is a 403, the error page's | `Role` (`users.role`, read by SQLx); the extractor `Admin`, one an app adds a role | `src/models/roles.rs`, `src/controllers/signed.rs` |
| Asking for the password before a sensitive step, unless the person proved who they are within 15 minutes | `Session::proven`, `Session::prove` (`PROVEN_MINUTES`) | `src/models/sessions.rs` |
| Sign up: the form, "check your mail" | `RegisterForm`, `RegisterSent`; the address `register::PATH` | `src/views/register.rs`, `src/controllers/register.rs` |
| A forgotten password: the form that mails the reset link | `ForgotForm`, `ForgotSent`; the address `forgot::PATH` | `src/views/forgot.rs`, `src/controllers/forgot.rs` |
| A link on its own line, the next step from a page; an auth form's ways on, under it | `LINK`; `NextSteps` | `src/views/layout.rs` |
| A password's rule (8 to 128 characters), anywhere one is set | `password_problem` | `src/models/users.rs` |
| A new account's name (2 to 100 characters) and email, checked before anything else | `name_or_email_problem`, from `Validator` | `src/models/users.rs` |
| An email address as it is stored and looked up: lowercase | `normalized_email` | `src/models/users.rs` |
| A request's line in the log: its route's pattern or its path, never a query | `RequestLog`, `logged_path` | `src/middleware/request_log.rs` |
| Hash or check a password | `hashing::hash`, `verify`, `verify_dummy` | `src/hashing.rs` |
| A page AI assistants should know about | a line in `/llms.txt` | `src/controllers/llms.rs` |

## Look

| Job | Use | Where |
|---|---|---|
| A colour in a view | the role tokens `primary`, `surface`, `ink`, `ink-muted`, `danger`, never a scale step | `style/tailwind.css` |
| A text field, and its refused look. Guarded | `FIELD`, red from its own `aria-invalid` | `src/views/layout.rs` |
| A refused form: its title, its line, the field marked as it's left. Guarded | `PageMeta::refused`, `PROBLEM`, `aria-describedby`, Alpine's `x-on:change`, as `ResetForm` | `src/views/layout.rs`, `src/views/reset.rs` |
| A grid's columns. Guarded | `grid-cols-N` or `minmax(0,1fr)`, never a plain `1fr` | `tests/one_job.rs` |
| A colour where CSS can't reach (`theme-color`, the manifest). The Android app reads the `theme-color` tag for its status bar, never a copy of its own | `SURFACE_HEX`, `BRAND_HEX` | `src/views/layout.rs` |
| A form's submit button while htmx sends it. Guarded (no `disabled`, every form's `hx-sync`) | `BUTTON`: the `busy:` variant, delayed 200 ms (`busy:delay-200`), never `disabled`; the form `hx-sync="this:drop"` and `aria-busy` through Alpine, as `ResetForm` | `src/views/layout.rs`, `style/tailwind.css`, `src/views/reset.rs` |
| A click on what loads a page sets Alpine's state, never flips it. Guarded | `open = true`, never `open = !open` | `tests/requests/races.rs` |
| A tap outside closes at its end. Guarded | `pointerup`, never `pointerdown` | `tests/one_job.rs` |
| Less motion when the visitor asks | the one Reduce Motion rule, never `motion-reduce:` per class | `style/tailwind.css` |
| A control a finger taps | at least 44px tall (`min-h-11`), as `ResetForm`'s field and button | `e2e/tests/health.rs` checks |
| Room at the top in the home-screen app (a control under the clock) | the `standalone:` variant, on a phone: `standalone:max-md:pt-6` | `style/tailwind.css` |
| A gap at the page's foot, clear of the home indicator or gesture bar (anything floating there). Guarded | `calc(0.75rem + var(--safe-area-bottom))`; never `standalone:`, which misses the app's webview | `style/tailwind.css` |
| A gap at a side, clear of a turned phone's camera. Guarded | `max(gap, var(--safe-area-left))` and `…-right`, as `COLUMN`; never `env(safe-area-inset-*)` | `style/tailwind.css`, `src/views/layout.rs` |
| A gap at the page's top, clear of a status bar over it (the app's webview on a phone). Guarded | `calc(gap + var(--safe-area-top))`, as the header, the skip link and the 429 page; beside `standalone:`, never instead of it | `style/tailwind.css`, `src/views/layout.rs` |
| Which side of a turned phone keeps its inset | `data-safe-area` on `<html>`, set by the helper; a change glides over 500ms (`@property`) | `public/js/safe-area.js` |
| The desktop's lines, `md` and `lg` | Tailwind's widths plus a height of 480px, `max-md`/`max-lg` their opposites; a width check anywhere else follows them | `style/tailwind.css` |

## Files and addresses

| Job | Use | Where |
|---|---|---|
| Link a file in `public/` | its constant in `paths::assets` (generated, versioned) | `src/paths.rs`, `build.rs` |
| The icons and the manifest | `paths::FAVICON*`, `APPLE_TOUCH_ICON`, `MANIFEST` | `src/paths.rs` |
| The stylesheet's address, hashed or plain | `Assets::stylesheet` | `src/assets.rs` |
| The build's name, in the head and every page's `X-Build` | `Assets::build`, `BUILD_META`, `BUILD_HEADER` | `src/assets.rs`, `src/islands.rs`, `src/render.rs` |

## Browser

| Job | Use | Where |
|---|---|---|
| Show whether islands run (example lazy island) | `IslandsCheck` | `src/islands.rs` |
| Move between pages without a reload | a plain link or form inside the shell's `#page`, which boosts it; never a hand-written `hx-get` for a page | `src/views/layout.rs` |
| htmx's settings | `HTMX_CONFIG` | `src/views/layout.rs` |
| A status shown as a page in the region | `PAGE_STATUSES` | `src/islands.rs` |
| What a move needs beyond htmx: focus, announcement, Back's scroll, a newer build, a gateway's error | `Navigation` | `src/islands.rs` |
| Free an island once it has left the page | its whole body inside `collected(move \|\| { … })` | `src/collect.rs` |
| Wake islands htmx swaps in, and htmx's Chrome workaround | `public/js/htmx-islands.js`, the glue | `public/js/` |
| Make Leptos's island loader start when Chrome gives no idle time | the Leptos fork's loader (`[patch.crates-io]`) | `Cargo.toml` |
| The app: a window on the live site, on desktops and phones | `src-tauri/`, `run` in `src/lib.rs`, its start page `offline/index.html`, the app's one script; the phones' projects in `gen/` | `src-tauri/` |
| A link that leaves the site in the app (a call, a mail, a text, another app, another site), handed to the system; any other scheme nowhere | `destination` in `src/lib.rs`, the system's schemes `SYSTEM_SCHEMES`, the site from `site_of` | `src-tauri/` |
| Pictures of the app on each device `testing.md` lists, one at a time, booted where it is off | `screens.sh` | `src-tauri/` |
| The app's links that leave it, tapped on the Android emulator | `links.sh` | `src-tauri/` |
| What the device scripts share: the site, the builds with it, an emulator up with the app open, the devices shut down | `devices.sh` (sourced) | `src-tauri/` |
| The Linux app, an AppImage built in a Docker box of Ubuntu 22.04 | `linux.sh`, `linux/Dockerfile` | `src-tauri/` |
| The site on a phone without a store | the home-screen app (the web manifest), beside the store app by choice (`decisions.md`) | `src/controllers/manifest.rs` |
| "Is the site up?" for another origin (the app's start page) | `/up`, `up::PATH`; Loco's `/_ping` for everything else, since it allows no other origin | `src/controllers/up.rs` |
| A library in the page | a vendored file in `public/vendor/`, its version in its name, loaded by `Scripts` | `src/views/layout.rs` |
| A browser test: a session in the lane's browser, its page, waits, memory | `browse`, `Page` (`open`, `until`, `still`, `move_to`, `held`) | `e2e/src/` |
| Which browser a lane drives, and what it can do (DevTools or not) | `Browser` (`has_devtools`) | `e2e/src/browser.rs` |
| A browser test that doesn't apply to this browser, said and counted | `Page::skip` | `e2e/src/page.rs` |
| A press in a browser test: a mouse's, held for the preload, never the driver's element; its hover and its held button alone | `Page::press`, `Page::hover`, `Page::hold`, `Page::let_go` | `e2e/src/page.rs` |
| One of the site's scripts refused in a browser test, its dependants' errors expected | `Page::block`, `Page::allow_errors` | `e2e/src/devtools.rs` |
| The visitor's settings in a browser test (Reduce Motion) | `Page::emulate` | `e2e/src/devtools.rs` |
| The accessibility audit | `Page::axe` (axe-core in `e2e/vendor/`) | `e2e/src/devtools.rs` |
| The pages and links browser tests share | `RESET`, `HOME_LINK` | `e2e/src/lib.rs` |
| A failed load a browser test causes on purpose | `Page::allow_failed_load` | `e2e/src/devtools.rs` |
| An answer of htmx's changed in a browser test (a deploy, a gateway's error, a 500, a slow answer), in every browser | `Page::answer_with` (`window.fetch` wrapped) | `e2e/src/page.rs` |

## Server

| Job | Use | Where |
|---|---|---|
| A query of the app's own | `sqlx::query!` on `sql::pool(&ctx)` | `src/sql.rs` |
| A transaction of the app's own. Guarded | `sql::write`: `BEGIN IMMEDIATE`, a connection a full disk left stuck closed (sqlx #4434) | `src/sql.rs` |
| A picture someone uploads: checked, made with fixed settings, kept outside the site's folder, named by its hash, served for a year | `uploads::avatar`, `keep`, `forget`, `name_of`; `settings.uploads.path`; served by `controllers::uploads` | `src/uploads.rs`, `src/controllers/uploads.rs` |
| A photo shrunk in the browser before it goes | a file field's `data-shrink` and `data-shrink-under`, the `shrink` the `Navigation` island arms | `src/islands/shrink.rs` |
| A user's avatar | `avatars::of`, `set`, `in_use`; the folder `Avatars` | `src/models/avatars.rs` |
| A secret handed to a browser, kept as its digest | `secret::fresh`, `digest`, `is_secret` | `src/secret.rs` |
| A cookie written or read | `cookies::set_cookie`, `store`, `value` | `src/cookies.rs` |
| A signed-in session: opened, found, closed | `Session` | `src/models/sessions.rs` |
| "Is this a developer's machine or the tests?" (development, test, e2e) | `settings::is_local` | `src/settings.rs` |
| The app's typed settings | `Settings` | `src/settings.rs` |
| A rate-limit bucket | `site_bucket`, `auth_bucket`, `files_bucket`; on an auth form's POST alone, `auth_limited` | `src/middleware/rate_limit.rs` |
| Who is signed in | `Signed`, a handler's extractor (nobody: to `/login`); `signed_in` where a page shows a visitor something else | `src/controllers/signed.rs` |
| A post another site's page sent, refused | `cross_site::refused`, first in every form's target | `src/controllers/cross_site.rs` |
| A visitor's address, behind the proxy or not | `VisitorIp`, `key_source` | `src/middleware/rate_limit.rs` |

## Mail

| Job | Use | Where |
|---|---|---|
| A mail's HTML part | a component in `views/mail.rs`, wrapped by `document` | `src/views/mail.rs` |
| Send a mail: sender, link origin, subject and text | `AuthMailer`'s `send` | `src/mailers/auth.rs` |
