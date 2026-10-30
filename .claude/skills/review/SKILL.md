---
name: review
description: Use when Davor asks for a review, an inspection or an audit of recent work in this starter ("inspect for DRY, redundant code, bugs, leaks", "one-job drift, security, accessibility, performance", "Rust superpowers", "fable do your thing"). An inline pass over the diff, findings by severity, no edit until his go.
---

# Review recent work

Inline, one focused pass (CLAUDE.md; Davor's own rules): no Workflow, one subagent at most, and an agent runs no cargo command.

## Scope

- The commits Davor names, a branch, or the whole project. Otherwise `git log --oneline -20`, and the commits since the last review's fixes; ask when the start is unclear.
- Plus what is not committed yet: `git status --short`, `git diff`.
- Read the diff (`git diff <base>..HEAD`), then each touched file in full around every change.

## Passes

1. **One-job drift**: one job done two ways, in the code or in what a visitor sees (the look, the motion, the words); [docs/one-job.md](../../../docs/one-job.md) first. Its kind named: DRY, logic or words written twice; single source of truth, one fact (a limit, a name, a size) decided in two places; snowflake, a one-off built beside the shared component. Also a piece the diff should have used (`render_page`, `Notice`, `FIELD`, `TITLE`, `paths::assets`, `sql::pool`, `settings::is_local`, `hashing`, `Page::answer_with`); a job with no row in docs/one-job.md; a drift that could come back, with its guard proposed for `tests/one_job.rs` (its pattern and its one home).
2. **Redundant**: dead code, a branch that cannot happen, a parameter nobody passes, a test that repeats another, code that would be simpler doing the same thing.
3. **Bugs**: `unwrap()` or `expect()` on the request path; an error path that drops the user's input; two requests at once, a double click, two tabs, a preloaded page racing Alpine's state (`open = true`, never a toggle; `hx-sync="this:drop"` on a form); an app query not `sqlx::query!` on `sql::pool`; an inline `style=` or script; `hx-on` or full Alpine (the CSP refuses `'unsafe-eval'`); a link to `public/` not from `paths::assets`; an island outside `src/islands.rs`; a cargo-leptos command without `--split`.
4. **Security**: [docs/security.md](../../../docs/security.md) first. A session, header or CSP change without its line there; a value not bound in SQL; a rate limit skipped (`site_bucket`, `auth_bucket`, `files_bucket`); a secret in a log or a staged `secrets*.env`.
5. **Leaks**: an island whose body skips `collected`; a window listener not removed in `on_cleanup`; a closure forgotten (`forget()`) instead of `kept`; a `thread_local!` holding a page's state rather than the visit's.
6. **Accessibility**: a control with no name; a live region missing or doubled; the focus lost after a swap (`Navigation`); a tap target under 44px (`min-h-11`); a refused form without `PageMeta::refused` and `aria-describedby`.
7. **Performance**: queries in a loop; a list with no limit; an island not lazy that only some pages use, or rendered where it isn't needed (rendered means fetched); the WASM bundle growing.
8. **Async**: Argon2 through `hashing`, never on the request's own thread; independent queries awaited one after another; work that must finish in its own task; a timeout on anything outside the process.
9. **Depth**: a repeated subtree inside an island's children not `.into_any()`, as `src/collect.rs` and `src/views/layout.rs` do, so the view's type stays shallow and compile times short.
10. **Policy and pages**: a config change that alters policy (headers, CSP, rate limits, cache, timeout) without `tests/config.rs`; `config/dev-server.yaml` and `config/staging.yaml` changed apart; a new public page without its line in `/llms.txt` (`src/controllers/llms.rs`).
11. **Docs**: a sentence the change made false in CLAUDE.md, `docs/architecture.md` or the feature's doc; a section changed in one of `dev-server.md`, `staging.md`, `production.md` and not the other two; a decision without its dated line in `docs/decisions.md`.
12. **Tests**: a new branch with no request or unit test; browser behaviour with no e2e test (written, not run).
13. **Rust's own guarantees**: where the compiler could hold a rule a person holds now. A `String` or `bool` where an `enum` or newtype would make a wrong value impossible (a status as text, two flags that cannot both be true); a job's insides `pub` where private would keep the job to its one way; a `_ =>` that would swallow a new variant where listing them makes the compiler point at every place; an error logged and gone on from, or an `Option` turned into a default that hides the missing value, where `?` and a typed error would do; a `.clone()` on the request path where a borrow would do; an index loop where an iterator says the intent.

Any pass or group runs alone: `/review <scope>, <passes> alone` (`/review the whole project, the async pass alone`). The whole project goes in groups, not all thirteen at once.

## Proof

Every finding is checked in the code before it is reported, with the case that fails; a doubt is reported as a doubt.

## Report

One table, most severe first:

| # | Kind | Where | What is wrong | Fix |
| --- | --- | --- | --- | --- |

The kind is a pass's name (DRY, single source of truth or snowflake for a drift); every file a clickable link with its line. Then one question: "go all", "go 1, 3" or "skip". Nothing is edited before the go; after it, the fixes, the checks, and the commit line.
