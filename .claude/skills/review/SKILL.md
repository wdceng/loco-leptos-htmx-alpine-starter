---
name: review
description: Use when the maintainer asks for a review, an inspection or an audit of recent work in this starter ("inspect for DRY, redundant code, bugs, leaks", "one-job drift, security, accessibility, performance", "Rust superpowers", "fable do your thing"). An inline pass over the diff, findings by severity, no edit until their go.
---

# Review recent work

Inline, one focused pass: one subagent at most, a multi-agent workflow only on the maintainer's word, and an agent runs no cargo command.

## Scope

- The commits the maintainer names, a branch, or the whole project. Otherwise `git log --oneline -20`, and the commits since the last review's fixes; ask when the start is unclear.
- Plus what is not committed yet: `git status --short`, `git diff`.
- Read the diff (`git diff <base>..HEAD`), then each touched file in full around every change.

## Passes

1. **One-job drift**: one job done two ways, in the code or in what a visitor sees (the look, the motion, the words); [docs/one-job.md](../../../docs/one-job.md) first. Its kind named: DRY, logic or words written twice; single source of truth, one fact (a limit, a name, a size) decided in two places; snowflake, a one-off built beside the shared component. Also a piece the diff should have used (`render_page`, `Notice`, `FIELD`, `TITLE`, `paths::assets`, `sql::pool`, `settings::is_local`, `hashing`, `Page::answer_with`); a job with no row in docs/one-job.md; a drift that could come back, with its guard proposed for `tests/one_job.rs` (its pattern and its one home).
2. **Redundant**: dead code, a branch that cannot happen, a parameter nobody passes, a dependency or a `web-sys` feature nothing uses, a test that repeats another, a comment that says what the next line says, code that would be simpler doing the same thing.
3. **Bugs**: `unwrap()` or `expect()` on the request path; an error path that drops the user's input; two requests at once, a double click, two tabs, a preloaded page racing Alpine's state (`open = true`, never a toggle; `hx-sync="this:drop"` on a form); an app query not `sqlx::query!` on `sql::pool`; an inline `style=` or script; a link to `public/` not from `paths::assets`; an island outside `src/islands.rs`; a cargo-leptos command without `--split`; a quoting bug or an unguarded `rm` in a shell script (`e2e/run.sh`, `src-tauri/*.sh`, `tools/`).
4. **Security**: [docs/security.md](../../../docs/security.md) first. A session, header or CSP change without its line there; a value not bound in SQL; a rate limit skipped (`site_bucket`, `auth_bucket`, `files_bucket`); a secret in a log or a staged `secrets*.env`, a `#[instrument]` among them, which records every argument unless it `skip`s the password, the token, the key or the body.
5. **Leaks**: an island scheduling through Leptos's timer helpers (`set_timeout`, `set_interval`, `request_animation_frame`), whose callbacks the root owner keeps for the tab's life, where a `Job` makes its callback once; an island htmx removes without `$$dispose` called (`htmx-islands.js`); a window or document listener not removed in `on_cleanup`; a closure `forget()`-ed instead of dropped with the island's owner (`on_cleanup`); a `thread_local!` holding a page's state rather than the visit's; a process `run.sh`, `screens.sh` or `links.sh` starts and doesn't stop on every path.
6. **htmx and Alpine under the CSP**: `hx-on` or a `js:` value (they need `'unsafe-eval'`, so they never run); full Alpine instead of its CSP build; a global (`window`, `location`, `navigator`, `document`) in an Alpine expression; `Alpine.data` or a script of our own beyond the two helpers in `public/js/`; Alpine and an island on one element; an attribute meant for the children without `:inherited` (as the shell's `hx-boost:inherited`, `src/views/layout.rs`).
7. **Accessibility**: a control with no name; a live region missing or doubled; the focus lost after a swap (`Navigation`); a tap target under 44px (`min-h-11`); a refused form without `PageMeta::refused` and `aria-describedby`.
8. **Performance**: a query a row in a loop, where one read of every row it needs into a `HashMap`, compared in memory, and only the differences written would do; the same key looked up again a row, where `HashMap::entry` keeps its first answer; a list with no limit; an island not lazy that only some pages use, or rendered where it isn't needed (rendered means fetched); the browser half growing (a dependency or a `web-sys` feature added to what the `hydrate` build compiles).
9. **Async**:
   - independent queries awaited one after another, where one `try_join!` would do, each on a connection of its own, never two in one transaction, which runs them in turn; a chain that depends on its order an async block in the join;
   - CPU work (an encoder, a hash, an image resized, a parse of megabytes) in `spawn_blocking`, never on an async thread; Argon2 through `hashing` alone, made before a transaction; a `spawn_blocking` inside `try_join!` its `JoinError` flattened in the branch;
   - nothing blocking an async thread: `std::fs`, `std::thread::sleep` or a `std::sync::Mutex` held across work in a handler (`tokio::fs`, `tokio::time::sleep`; a lock held across an `.await` is a `tokio::sync::Mutex`, and then what queues behind it named), a check at boot excepted; Tokio's line, no more than 10 to 100 µs between two `.await`s, so a loop over many rows that awaits nothing blocks too;
   - a `spawn_blocking` cannot be aborted once it runs, and a shutdown waits for it: a long one has its own end (a deadline, a check between rows);
   - a `tokio::spawn` from a request, where work that must outlive it goes to a worker (the spawns at boot, `maintenance.rs`, `rate_limit.rs` and `deploy_checks.rs`, are the shape); nothing relying on work after the answer, which a shutdown cuts off; per-request state in a `task_local!`, where an extractor or the handler's argument would carry it in the open;
   - a timeout on anything outside the process; something heavy behind a `Semaphore`; many tasks a `JoinSet` under one `timeout`; writes that belong together one `sql::write`;
   - the tool a change needed and did not use: `join!` where no branch can fail, `select!` for the first of several (a clock, a shutdown), a channel between tasks (`mpsc` a queue, `broadcast` every reader each message, `watch` the latest value), a `CancellationToken` for a long job a shutdown or a person stops.
10. **Depth and props**: a repeated subtree inside an island's children not `.into_any()`, as `src/views/layout.rs` does, so the view's type stays shallow and compile times short; an island prop that carries content (text, markup, a list to draw), where `children` would render it on the server: a prop is serialized into the page as JSON and read again in the browser, so its data goes twice.
11. **Policy and pages**: a config change that alters policy (headers, CSP, rate limits, cache, timeout) without `tests/config.rs`; a layer added in code (`middleware::from_fn`, `route_layer`, a tower layer) where Loco's config already has one for the job, or one with no test pinning it; `config/dev-server.yaml` and `config/staging.yaml` changed apart; a new public page without its line in `/llms.txt` (`src/controllers/llms.rs`).
12. **Docs**: a sentence the change made false in CLAUDE.md, `docs/architecture.md`, the feature's doc or a comment; a section changed in one of `dev-server.md`, `staging.md`, `production.md` and not the other two; a decision without its dated line in `docs/decisions.md`.
13. **Tests**: a new branch with no request or unit test; browser behaviour with no e2e test (written, not run: the `e2e` skill); a control that changes before its answer with no in-between test (pass 17).
14. **Rust's own guarantees**: where the compiler could hold a rule a person holds now. A `String` or `bool` where an `enum` or newtype would make a wrong value impossible (a status as text, two flags that cannot both be true); a job's insides `pub` where private would keep the job to its one way; a `_ =>` that would swallow a new variant where listing them makes the compiler point at every place; an error logged and gone on from, or an `Option` turned into a default that hides the missing value, where `?` and a typed error would do; a `.clone()` on the request path where a borrow would do; a function that returns a new `String` for text it mostly leaves as it was, where `Cow<str>` would hand the input back; a string built once and cloned on many requests, where `Arc<str>` would share it; an index loop where an iterator says the intent; a result a caller must not drop without `#[must_use]`; a public enum other code matches on, where `#[non_exhaustive]` should keep a new variant from breaking it, or one it must match in full, where it should not; a public type without `Debug`, or an error type that says nothing of what failed.
15. **Axum**: input read by a typed extractor (`State`, `Path`, `Query`, `Form`, a struct with real types) where a handler parses strings by hand; the body's extractor (`Form`, `Json`, `Bytes`, `String`, `Multipart`) not the last argument, or two of them; an `Extension` where `State` would do, a missing one a `500` at run time where `State` fails the build; an `Option<…>` extractor read as "absent or wrong", where a malformed value still refuses the request; a check every handler repeats in its body (`else { return … }`) where an extractor of its own, `FromRequestParts` with `Rejection = Response`, would answer the 403 or 404 itself; a route that takes an upload without a body limit of its own (`uploads.rs`), or a small form under the upload's limit (axum's default is 2 MB); a large answer built whole in memory where a streamed body would do; a handler's error a typed one turned into the answer, never a `500` hiding a `400`.
16. **A framework's own tool**: Loco, Leptos and their crates each bring a tool for a job the starter already does its own way; one that appears is a second way (Rule no. 1), named with the one it would sit beside and asked about, never added quietly:
    - Loco's scheduler beside the nightly restart (`src/maintenance.rs`); its cache (`ctx.cache`) beside a `LazyLock`; its storage (`ctx.storage`) beside the uploads on disk (`src/uploads.rs`);
    - a Leptos server function (`#[server]`) beside Loco's controllers: a second door to the server, with its own address, outside the rate limiter and the session's checks;
    - Leptos's `Resource`, `Suspense` or `Action` beside htmx: data fetched in the browser and sent twice, and a bigger WASM bundle.
17. **One fact drawn twice**: a fact (a status, a choice, an on or off, a count) the server draws in several places or forms (a label, an icon, an `aria-label`, `aria-busy`, `disabled`, an open dialog), checked at three moments and in its test:
    - at the tap: what an island or Alpine changes before the answer, against every detail the server draws for that fact; a detail it doesn't switch shows the old state until the answer. The server draws every state and one attribute switches between them (a `data-*` with Tailwind's `group-data-*`, `aria-busy` with `busy:`, as `BUTTON` does), never a detail copied into the browser;
    - at the answer: every part of the page that shows the fact is in the swap, or it stays stale until a reload; two polls on one page are one clock;
    - at a failure: a failed or refused request leaves no browser guess on screen, back to the server's state in one place (as `BusyForm`'s `htmx:finally:request` does);
    - its test: an e2e test that holds the answer back and compares the part after the tap with the part after the answer.

Any pass or group runs alone: `/review <scope>, <passes> alone` (`/review the whole project, the async pass alone`). The whole project goes in groups, not all seventeen at once.

## Proof

Every finding is checked in the code before it is reported, with the case that fails, by reading or by a test that shows it; a doubt is reported as a doubt.

## Report

One table, most severe first:

| # | Kind | Where | What is wrong | Fix |
| --- | --- | --- | --- | --- |

The kind is a pass's name (DRY, single source of truth or snowflake for a drift); every file a clickable link with its line. Under it, what was checked and found clean, in one line. Then one question: "go all", "go 1, 3" or "skip". Nothing is edited before the go; after it, the fixes, the checks, and the commit line.
