---
name: deps-check
description: Use when the maintainer asks to check the project's crates for bugs, issues or advisories ("check every crate we use", "is anything upstream hurting us", "what is fixed and not released", "what should we avoid"), or before a release. One pass over every dependency: the framework stacks through their own deps-check skills, then the project's other direct crates' upstream issues and every crate's advisories, each finding matched to the project's code and given one verdict (act now, avoid, wait, upgrade, nothing, unknown). Read-only; the project's list (.claude/deps.md) carries the watch list, the accepted advisories and the workarounds from run to run. Upgrades themselves are /cargo's.
---

# Dependency check

Every crate the project builds on, checked for what reaches it upstream: advisories, open bugs, bugs fixed but not released. Read-only: nothing is edited, upgraded, commented or filed.

With ultracode on, a workflow, one agent a stack or a tier, each finding read a second time; with it off, inline, one pass, one subagent at most (the maintainer's rules: `CLAUDE.local.md`, `~/.claude/CLAUDE.md`). An agent runs no cargo command.

## The project's list first

Read `.claude/deps.md` in the project's root: the crates it watches beyond the stacks and why, the advisories it accepted (with the file that ignores them), each workaround for an upstream bug with its canary test and the release that ends it, and the last run's date. The run starts from there. No list: say so, run anyway, and propose one at the end (a new project copies the starter's).

## Scope

- Every `Cargo.lock` of the project (the root's, an app's, the browser tests'), each its own workspace.
- **The stacks**, through their own skills, run first and reported in their tier: `/loco-deps-check`, `/leptos-deps-check`, `/tauri-deps-check`, `/htmx-deps-check`, `/alpine-deps-check`, each where the project uses it (the list). A crate one of them covers is not checked again here.
- **The project's own direct dependencies** no stack covers (the root's `[workspace.dependencies]` and each member's `[dependencies]`): their issues, in full.
- **Every crate in each lock**, direct or not: advisories and yanked versions alone. Hundreds of crates: their issue trackers are not read.
- Not here: which upgrades are safe (`/cargo`), the tools (`/tools`), a review of a crate's code (`cargo vet`, on the maintainer's word only).

## Steps

1. **Versions.** Each crate's locked version, and more than one version of a crate in a lock as a finding of its own:

   ```sh
   awk -v n="image" '$0=="name = \""n"\""{getline; gsub(/version = |"/,""); print; exit}' Cargo.lock
   ```

2. **Advisories,** in every workspace: `cargo audit` (and `cargo deny check advisories` where installed). Every result read; an ignored one checked against its reason in the project's audit file: does the reason still hold, has a fixed release come.
3. **Yanked:** `cargo audit` reports them; a yanked version in a lock is a finding.
4. **Each direct crate,** from its own manifest's `repository`, its current home (repos move: `gh api repos/<owner>/<repo> --jq .full_name`):
   - the locked release's date (crates.io), and the newer releases;
   - open issues, most recently active first, narrowed by the features and APIs the project uses;
   - issues closed since the locked release, and the PRs merged since it: fixed on main, released or not;
   - each candidate read in full before it is judged; the crate's source read in `~/.cargo/registry` at the locked version where the claim needs it.
5. **Match to the code.** A finding reaches the project only with evidence: the file and line that calls the code, or the config that turns it on. What cannot be shown either way is "unknown", never "no" (reachability tools say the same: an unanalysed path is unknown, not unreachable). Every "yes" is checked a second time by trying to refute it (the subagent).
6. **The list's workarounds:** each one's upstream fix, merged, released, two weeks old; its canary test still failing as it should.

Automated reachability (OSV-Scanner's Rust call analysis) is experimental and runs every dependency's `build.rs`: only on the maintainer's word.

## Verdicts

| Verdict | Meaning |
|---|---|
| act now | reaches the project, harmful, and something can be done today (a fixed release past its two weeks, a config, a workaround) |
| avoid | an API, feature or pattern to stay off until fixed: named, with a guard test proposed for the guard file |
| wait | fixed upstream, not usable yet: the release that has it, or "unreleased", and the date it qualifies |
| upgrade | the fix is in a release two weeks old: the version, handed to `/cargo`'s edits |
| nothing | does not reach the project, with the reason |
| unknown | neither shown, with what would settle it |

## Report

A table a tier (the stacks' in their own skills' shape, then the project's own, then the advisories):

| Crate (version) | Issue or advisory | Reaches us? | Evidence | Fix state | Verdict |
|---|---|---|---|---|---|

The evidence a clickable link with its line. Then:

- **Act now** and **avoid**: each a sub-task for the working session, with its test.
- **Wait**: each with its date, the release to watch, and the workaround that holds meanwhile.
- **The list's changes**: what `.claude/deps.md` should gain or lose (a new workaround and its canary, an accepted advisory, a crate to watch), proposed, edited on the go.
- **Not checked**: what was skipped, and why.

Then one question: "go all", "go 1, 3" or "skip". Nothing is edited before the go.

## The list's shape

`.claude/deps.md`, in the project's git, short:

- `## Stacks`: the stack skills that apply, each with what the project uses of it.
- `## Watched`: the crates beyond the stacks worth reading every run, each with the feature it serves.
- `## Accepted advisories`: the id, the file that ignores it, the release that ends it.
- `## Workarounds`: the upstream issue, the code that works around it, its canary test (failing once upstream fixes it), the release that ends it, what comes out then.
- `## Last run`: the date and the tip it ran against.

Task ids stay out of it where the project's guard keeps them in its plan alone.
