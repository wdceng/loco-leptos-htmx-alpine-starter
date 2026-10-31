# Staging

|            |                                          |
| ---------- | ---------------------------------------- |
| Server     | `<server>`, the SSH login, e.g. `root@203.0.113.10` |
| Directory  | `/srv/app/stg`                           |
| Unit, user | `app-stg.service`, `app-stg`             |
| Port       | 5151                                     |
| URL        | `https://<domain>`, e.g. `https://stg.example.com` |

Replace `<server>` and `<domain>` in the commands below. The directory, unit,
user and port are suggestions; if you change one, change it everywhere in
this file.

Deployed: the `app` binary, `hash.txt` next to it, `config/staging.yaml`,
`site/`, `secrets.env`.

Staging is production's twin. The differences:

| | staging | production |
|---|---|---|
| Binary profile | `release`: production's exact binary | `release` |
| Secrets | `JWT_SECRET` required; mail placeholders, so a file without mail boots | no defaults, a missing one stops the boot |
| Static cache | 60 s | one year |
| Search engines | `Disallow: /` and `X-Robots-Tag: noindex, nofollow` | `Allow: /` |
| Logs | compact, debug, backtraces on | JSON, info |
| Before a deploy | checks | checks, `cargo audit`, no placeholders |

The firewall, the log retention, the origin key and the mail account are
set up once per server and described in `docs/production.md`.

## Deploy

### Checks

```bash
cargo clippy --all-targets && cargo test &&
cargo clippy --lib --target wasm32-unknown-unknown --no-default-features --features hydrate -- -D warnings
```

The template's placeholders are fine on staging, not on production.

Try the release build locally first (http://localhost:5153). It builds into
`target/site-release`, so the watch loop on 5150 keeps running untouched:

```bash
LEPTOS_HASH_FILES=true LEPTOS_SITE_ROOT=target/site-release cargo leptos build --release --split &&
LEPTOS_SITE_ROOT=target/site-release PORT=5153 ./target/release/app start
```

### Build and deploy

```bash
test -f secrets.staging.env && tools/cargo-leptos/check.sh &&
LEPTOS_HASH_FILES=true LEPTOS_SITE_ROOT=target/site-release cargo leptos build --release --split --frontend-only &&
CARGO_TARGET_DIR=target/zig cargo zigbuild --release --target x86_64-unknown-linux-gnu.2.31 &&
rm -rf dist && mkdir dist &&
cp target/zig/x86_64-unknown-linux-gnu/release/app dist/app &&
cp target/release/hash.txt dist/hash.txt &&
cp -r target/site-release dist/site &&
ssh <server> "mkdir -p /srv/app/stg/config" &&
ssh <server> "systemctl stop app-stg" &&
scp dist/app <server>:/srv/app/stg/app &&
scp dist/hash.txt <server>:/srv/app/stg/hash.txt &&
ssh <server> "rm -rf /srv/app/stg/site" &&
scp -r dist/site <server>:/srv/app/stg/ &&
ssh <server> "find /srv/app/stg/site -name '.DS_Store' -delete" &&
scp config/staging.yaml <server>:/srv/app/stg/config/ &&
scp secrets.staging.env <server>:/srv/app/stg/secrets.env &&
ssh <server> "chmod 600 /srv/app/stg/secrets.env && systemctl start app-stg" &&
ssh <server> "journalctl -u app-stg -f"
```

- A failing step stops the chain. A failed build never touches the server.
- **No `secrets.staging.env`, no deploy.** The first line stops the chain
  before anything is built; without it the upload near the end would fail
  with the unit already stopped.
- **No cargo-leptos with the split fixes, no deploy.** `tools/cargo-leptos/check.sh` stops
  the chain when this machine's cargo-leptos lacks the split fixes
  (`development.md`, "Lazy islands").
- `--release`, as production: staging runs the binary production will.
  Work in progress goes to the dev server, which builds faster.
- `.2.31` is the oldest glibc the binary needs (Step 0 in
  `docs/production.md`). `target/zig` keeps this build apart from the
  Mac's own.
- `hash.txt` must travel with the binary and match `site/`, or the app
  won't start.
- The old `site/` is deleted first, so files you removed locally don't
  linger.

### Smoke test

A Cloudflare challenge blocks curl from outside, so test on the server:

```bash
ssh <server> 'cd /srv/app/stg && for p in / /robots.txt $(ls site/pkg | grep -v "\.ts$" | sed "s|^|/pkg/|"); do printf "%-40s " "$p"; curl -s -o /dev/null -w "%{http_code}\n" "http://127.0.0.1:5151$p"; done && curl -s http://127.0.0.1:5151/ | grep -o "href=\"/pkg/[^\"]*\"" && curl -s http://127.0.0.1:5151/robots.txt && curl -sI http://127.0.0.1:5151/ | grep -i x-robots-tag'
```

All 200, the linked `href` values among the listed files, `Disallow: /` and
`X-Robots-Tag: noindex, nofollow`. The app also logs in to SMTP once at
every start: `ssh <server> "journalctl -u app-stg -b | grep 'smtp login check'"`.
On placeholder mail settings that's a warning, which is expected until
staging should send real mail. Then open `https://<domain>` with the
browser console open: no errors.

### Config-only change

```bash
scp config/staging.yaml <server>:/srv/app/stg/config/ &&
ssh <server> "systemctl restart app-stg"
```

## Unit commands

```bash
ssh <server> "systemctl restart app-stg"
ssh <server> "systemctl status app-stg"
ssh <server> "journalctl -u app-stg -f"
ssh <server> "systemctl stop app-stg"             # down until the next reboot
ssh <server> "systemctl disable --now app-stg"    # down across reboots
ssh <server> "systemctl enable --now app-stg"     # back
```

## Server

Same box, Caddy and Cloudflare setup as production (`docs/production.md`),
plus:

- **A gate in front.** In the Cloudflare zone: Security rules, custom rule
  `Staging hosts: challenge`, hostname is in a list that includes
  `<domain>`, action Managed Challenge. Visitors pass a check, scanners
  don't.
- **Paths without a browser skip the gate.** What fetches them has no
  person to pass a challenge: the app's start page reads `/up` with a
  `fetch`. A custom rule placed first, `Paths without a browser: skip
  challenge`: hostname is in the same list **and** URI Path is in a list,
  action Skip, all remaining custom rules:

  ```
  (http.host in {"<domain>"} and http.request.uri.path in {"/up"})
  ```

  Check: `curl -s https://<domain>/up` prints `ok`.
- **A new such path goes into that list,** never a rule of its own: an
  image a mail links (a mail program's image proxy fetches it), a service
  worker's fetch (`push.md`), a `/.well-known/` file another company reads
  (Android's app links). An islands app on this stack met four, one at a
  time.
- To see what the app sends without Cloudflare's headers:
  `curl -sI http://127.0.0.1:5151/` on the server.

## Environment variables

In the unit: `LOCO_ENV`, `BINDING`, `PORT`, `LEPTOS_*`, `DATABASE_URL`. The
domain is `server.host` in `config/staging.yaml`; `HOST=` in the unit
overrides it. Change one:

```bash
ssh -t <server> "nano /etc/systemd/system/app-stg.service" &&
ssh <server> "systemctl daemon-reload && systemctl restart app-stg"
```

- Keep `BINDING=127.0.0.1`. `0.0.0.0` would bypass Caddy.
- Secrets live in `secrets.staging.env` in the repo root on your machine.
  It's git-ignored and every deploy uploads it as `secrets.env`. Never use
  production's file here.
- `JWT_SECRET` is required: without it the app refuses to boot. Without
  the `MAILER_*` lines it boots on the placeholders in
  `config/staging.yaml`, and mail fails to send. A missing file stops the
  unit.

Create it once from its example, which explains every key, with its own
`JWT_SECRET`. Uncomment the `MAILER_*` lines when staging should send real
mail (details in `docs/production.md`):

```bash
umask 077 && cp secrets.staging.env.example secrets.staging.env && printf 'JWT_SECRET=%s\n' "$(openssl rand -base64 64 | tr -d '\n')" >> secrets.staging.env
```

- **`JWT_SECRET` is 64 bytes,** as RFC 7518 asks of an HS512 key, Loco's.
  `openssl` wraps its output at 64 characters; `tr` joins it into one line.

## One-time setup

```text
/srv/app/stg/
├── app          Linux binary
├── hash.txt     next to the binary
├── config/      staging.yaml only
├── site/        target/site-release
└── secrets.env  root-only, loaded by the unit
```

Everything is root's and read-only for the app. The only writable place is
`/var/lib/app-stg`, where the SQLite file lives.

### Step 0: Port free, glibc new enough?

```bash
ssh <server> "ss -tlnp | grep ':5151\b' || echo '5151 free'"
ssh <server> "ldd --version | head -1"
```

A different port goes in two places only: `PORT=` in the unit and
`reverse_proxy` in Caddy. For glibc, see Step 0 in `docs/production.md`.

### Step 1: Directory, user, files

```bash
ssh <server> "mkdir -p /srv/app/stg/config && useradd --system --user-group --no-create-home --shell /usr/sbin/nologin app-stg"
```

Build `dist/` with lines 2 to 7 of the deploy chain, then:

```bash
scp dist/app <server>:/srv/app/stg/app &&
scp dist/hash.txt <server>:/srv/app/stg/hash.txt &&
scp -r dist/site <server>:/srv/app/stg/ &&
scp config/staging.yaml <server>:/srv/app/stg/config/ &&
scp secrets.staging.env <server>:/srv/app/stg/secrets.env &&
ssh <server> "chmod +x /srv/app/stg/app && chmod 600 /srv/app/stg/secrets.env"
```

### Step 2: The unit

```bash
ssh <server> -t "nano /etc/systemd/system/app-stg.service"
```

```ini
[Unit]
Description=app staging
After=network.target

[Service]
Type=simple
User=app-stg
WorkingDirectory=/srv/app/stg
ExecStart=/srv/app/stg/app start
Restart=always
RestartSec=5
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
StateDirectory=app-stg
StateDirectoryMode=0700
UMask=0077
CapabilityBoundingSet=
RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6 AF_NETLINK
SystemCallFilter=@system-service
SystemCallErrorNumber=EPERM
ProtectKernelTunables=yes
ProtectKernelModules=yes
ProtectKernelLogs=yes
ProtectControlGroups=yes
RestrictNamespaces=yes
RestrictRealtime=yes
RestrictSUIDSGID=yes
LockPersonality=yes
ProtectProc=invisible
Environment=DATABASE_URL=sqlite:///var/lib/app-stg/app_staging.sqlite?mode=rwc
Environment=LOCO_ENV=staging
Environment=BINDING=127.0.0.1
Environment=PORT=5151
Environment=LEPTOS_OUTPUT_NAME=app
Environment=LEPTOS_SITE_ROOT=site
Environment=LEPTOS_SITE_PKG_DIR=pkg
Environment=LEPTOS_ENV=PROD
EnvironmentFile=/srv/app/stg/secrets.env

[Install]
WantedBy=multi-user.target
```

- **Keep `start`.** Without it the binary prints its help and
  `Restart=always` loops.
- **Keep `Restart=always`.** The app stops itself every night at
  `nightly_restart.hour` (03:00 UTC as shipped) and the unit brings it
  back.
- **The lines from `StateDirectoryMode` on keep the service to what it
  needs.** Its database readable by its own user alone (three units share
  the box), no capabilities, an ordinary service's system calls and
  sockets. A call refused answers `Operation not permitted` (the security
  review of 2026-10-31).
- A server set up before them: the lines in, then the files closed once:

```bash
ssh <server> "chmod 700 /var/lib/app-stg && find /var/lib/app-stg -maxdepth 1 -name 'app_staging.sqlite*' -exec chmod 600 {} + && systemctl daemon-reload && systemctl try-restart app-stg && systemctl is-active app-stg && stat -c '%a %U %n' /var/lib/app-stg /var/lib/app-stg/*"   # active; 700 the folder, 600 the database; a stopped unit stays stopped
```

### Step 3: Start

```bash
ssh <server> "systemctl daemon-reload && systemctl enable --now app-stg && systemctl status app-stg"
```

| Boot error | Cause |
|---|---|
| `the site folder ... is missing` | `site/` not uploaded next to the binary |
| `a hashed build must be deployed together with its hash file` | `hash.txt` missing |
| `the hash file is stale` | `hash.txt` and `site/` from different builds |
| a `get_env` error naming `JWT_SECRET` | `secrets.env` without its `JWT_SECRET` line |
| `staging won't start: JWT_SECRET empty or still replace-me` | the line is there, the key isn't (`src/deploy_checks.rs`) |
| `settings.mail.from is not a sender` | `MAILER_FROM` must read `Name <address>` |
| `invalid settings: block: failed to parse timezone` | `nightly_restart.zone` isn't an IANA name (`UTC`, `Europe/Zagreb`) |
| `unable to open database file` | `StateDirectory=` or `DATABASE_URL=` missing |
| `Operation not permitted` | a hardening line refuses a call: `journalctl -u app-stg` names it, then that line out |
| `version GLIBC_2.31 not found` | server too old, see Step 0 |

### Step 4: Caddy

Cloudflare DNS: an `A` record for `<domain>` to the server, proxied. The
origin key must already be set up (`docs/production.md`).

Add this block to `/etc/caddy/Caddyfile` (never replace the file):

```bash
ssh <server> -t "nano /etc/caddy/Caddyfile"
```

```text
<domain>:80, <domain>:443 {
        import origin_check
        reverse_proxy 127.0.0.1:5151 {
                header_up -X-Origin-Key
        }
        encode gzip zstd
}
```

```bash
ssh <server> "caddy validate --config /etc/caddy/Caddyfile && systemctl reload caddy"
```

Then add `<domain>` to the Cloudflare rules `Staging hosts: challenge` and
`Paths without a browser: skip challenge`. Until then the host is open to
scanners.

## Manual run (without systemd)

```bash
ssh <server> -t "systemctl stop app-stg && cd /srv/app/stg && { pkill -f '^/srv/app/stg/app ' || true; } && set -a && . ./secrets.env && set +a && LOCO_ENV=staging BINDING=127.0.0.1 PORT=5151 LEPTOS_OUTPUT_NAME=app LEPTOS_SITE_ROOT=site LEPTOS_SITE_PKG_DIR=pkg LEPTOS_ENV=PROD DATABASE_URL=sqlite:///var/lib/app-stg/app_staging.sqlite?mode=rwc nohup runuser -u app-stg -- /srv/app/stg/app start > /srv/app/stg/app.log 2>&1 & sleep 1; tail -f /srv/app/stg/app.log"
```

**Always `pkill -f '^/full/path '`.** A bare `pkill app` also kills
production.
