---
name: redis-benchmark-rs-maintainer-review
description: Review a redis-performance/redis-benchmark-rs pull request, branch, or diff in a way that's grounded in this specific repo's real (very thin) history — a single-maintainer, self-merged Rust project with zero recorded human review comments to date. Use this whenever the user asks to review a redis-benchmark-rs PR, asks whether it would pass real review or get merged, wants a repo-specific pre-merge check, or is deciding accept/reject on a redis-performance/redis-benchmark-rs PR. Prefer this over a generic code-review skill for anything touching redis-performance/redis-benchmark-rs — the generic skill doesn't know this project's real risk areas (TLS crypto-provider init, cluster CROSSSLOT/pipeline routing, rate-limiter drift, key-range partitioning, multi-distro packaging) or that its review history is honestly close to nonexistent.
---

# redis-benchmark-rs review

## Honesty warning: read this before writing anything

This repo has **no review history to imitate.** As of this skill's writing (mined
`gh pr list --repo redis-performance/redis-benchmark-rs --state all` and
`gh api .../pulls/<n>/reviews`, `/comments`, `/issues/<n>/comments` on all 4 PRs, plus
`gh issue list`): the repo has exactly **4 pull requests**, all authored by
**fcostaoliveira**, all self-merged **within minutes** of opening (2–5 minutes on PR#1/#2/#4,
under 2 minutes on PR#3), **zero** PR reviews, **zero** review comments, **zero** issue
comments, and **zero** issues ever opened. There is no `AGENTS.md` or `CONTRIBUTING.md` in
the repo. Unlike `redisbench-admin` (thin, but has one real multi-point review to study) or
`memtier_benchmark` (a dense, long-tenured review culture), this repo has **no comparison
class at all** for "how a real reviewer here responds." Do not invent one. Do not manufacture
a "maintainer voice," a house style, or a pattern of what gets nitpicked — there is no
evidence for any of it. Say so plainly if asked, and review on technical merits grounded in
this codebase's own real, self-authored bugfix history instead (see
`references/real-history.md`).

This also means: don't try to imitate a person. Write as a careful, first-pass technical
reviewer who has actually read this small codebase, not as an impression of
fcostaoliveira — the record gives you a self-merge pattern and some PR-description
conventions (see `references/real-history.md`), not a reviewing voice.

## Scope gate

If the PR's content falls entirely outside anything this skill's taxonomy covers (e.g. it
touches no Rust source under `src/` or `tests/`, no CI/release/packaging surface) say so in
one sentence and treat it as out of scope rather than force-fitting the checklist below.
Given the repo is 8 source files and ~1,240 lines of Rust plus a CI/release pipeline, most
real PRs will hit at least one category.

## Process

1. **Get the material.** `gh pr view <n> --repo redis-performance/redis-benchmark-rs
   --json body,commits,files,author` and `gh pr diff <n> --repo redis-performance/redis-benchmark-rs`.
   Read the PR description in full — every merged PR here so far has included a `## Summary`
   (or plain prose) section plus a validation note (test counts, `cargo fmt`/`clippy`
   status, and often a manual end-to-end run against a real Redis/TLS endpoint); if the
   author already states what they tested, acknowledge that rather than re-demanding it.

2. **Work the checklist** in `references/known-risk-areas.md` — this project's own real,
   self-authored bugfixes (not reviewer catches; there have been none) that define its actual
   recurring failure modes: TLS `CryptoProvider` initialization, cluster-mode CROSSSLOT /
   pipeline-depth handling, key-range partitioning across concurrent connections, rate-limiter
   drift correction, and multi-distro packaging (`el8`/`el9`/`jammy`/`noble`). Give these real
   weight — they are the only categories with actual evidence behind them in this repo.

3. **Check the quality bar every real PR here states it met**, per
   `references/real-history.md`: `cargo fmt` clean, `clippy -D warnings` clean, and a stated
   unit+integration test count. This is a self-imposed convention repeated in all four PR
   descriptions, not a documented CONTRIBUTING.md rule (there isn't one) — cite it as "every
   PR here has stated this," not as written policy.

4. **Write the review.** A few sentences to a short numbered list, matching the size of real
   PRs here (each PR so far is a focused, single-purpose change, not a sprawling one).
   Open with a clear, unmissable automated marker (see the workflow prompt). Where you have a
   real, on-point concern from `known-risk-areas.md`, name the specific code path
   (e.g. "does `--rate-limiting` still drift-correct after a pipeline batch is sent late" or
   "does this preserve the min/max partitioning across connections your PR#2 fix added").
   Where you don't have a real precedent for something, say so honestly rather than
   inventing one, and reason about the change on its technical merits instead.
   Hedge like an honest reviewer who isn't certain: "worth checking", "I think", "not
   necessarily a blocker". Never fabricate confidence the record doesn't support.

5. **Land on a verdict** in plain prose at the end — never a literal "Verdict" label, a
   bolded summary line, or a "TL;DR" block. This repo has no evidenced convention either way
   for how a review ends (there have been none), so default to the same plain-prose-ending
   convention used by this skill's sibling projects rather than inventing a stylized format.

## What NOT to do

- Don't write a generic "code review essay" with formal headers like "Correctness",
  "Security", "Performance".
- Don't invent a reviewer personality, a house style, or a "this is what fcostaoliveira
  always says" voice — there is no such data. The one available real pattern is what
  PR *descriptions* (self-authored, pre-review) tend to include; see `references/real-history.md`.
- Don't claim a category is "this project's convention" unless `references/real-history.md`
  or `references/known-risk-areas.md` actually cites it. If you're reasoning from first
  principles because the repo's history doesn't cover something, say so.
- Don't apply memtier_benchmark's or redisbench-admin's Python/C taxonomies wholesale — this
  is a Rust (tokio + `redis` crate + rustls) codebase with a different real risk surface.
- Don't close with a labeled, bolded verdict block — end in plain prose.
- Don't literally `@`-mention any GitHub username, ever.
