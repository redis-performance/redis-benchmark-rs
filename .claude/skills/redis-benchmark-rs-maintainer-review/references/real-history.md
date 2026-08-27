# Real history — redis-performance/redis-benchmark-rs

Mined via `gh pr list --repo redis-performance/redis-benchmark-rs --state all --limit 300`,
`gh api repos/redis-performance/redis-benchmark-rs/pulls/<n>/reviews`,
`.../pulls/<n>/comments`, `.../issues/<n>/comments` on all 4 PRs, and
`gh issue list --repo redis-performance/redis-benchmark-rs --state all`, as of 2026-08-27.
This is the complete record — not a sample.

## The honest headline

- **4 pull requests total**, all merged, all authored by **fcostaoliveira** (the repo's sole
  contributor to date).
- **Zero PR reviews, zero review comments, zero issue comments** on any of the 4 PRs.
- **Zero issues** have ever been opened on this repo.
- Every PR was **self-merged within minutes** of opening: PR#1 opened 11:43:09, merged
  11:46:28 (~3 min); PR#2 opened 12:50:04, merged 12:55:20 (~5 min); PR#3 opened 12:57:42,
  merged 12:59:20 (~2 min); PR#4 opened 14:19:28, merged 14:20:50 (~1 min) — all on the same
  day, 2026-06-24 (the repo's creation date).
- No `AGENTS.md`, no `CONTRIBUTING.md` exist in the repo. `README.md` exists and documents
  flags and a `[x]`/`[ ]` status/roadmap checklist.
- Every commit/PR carries a `🤖 Generated with [Claude Code]` trailer — this repo's own
  development has so far been Claude-Code-assisted by its single maintainer.

**What this means for review voice: there isn't one to imitate.** Do not write as if
channeling "how fcostaoliveira reviews" — he has never left a review comment on this repo.
The only real, observable pattern is what he writes in his own **PR descriptions** (below),
which is evidence of this project's authoring conventions, not of anyone's reviewing style.

## The one real, repeated convention: PR description shape

All four PR bodies independently follow a similar shape, worth citing as "every PR here has
done this" rather than as written doctrine (there is none):

- A `## Summary` (or, for the smallest fix, a one-paragraph explanation) describing *why*,
  not just *what*.
- An explicit statement of what was verified: unit + integration test counts (e.g. "14 unit +
  3 integration tests", "16 unit + 4 integration tests (live Redis)"), and `cargo fmt` /
  `clippy -D warnings` clean, stated in every single PR body without exception.
- Several PRs (PR#1, PR#2) go further and describe a specific end-to-end validation run
  against a real backend — PR#2 explicitly states testing "against live TLS Redis Enterprise
  DBs (6.4 cluster + 7.22 single-endpoint)" and reports a concrete measured result ("rate-
  limited GET held 13,100 ops/sec dead-on").
- PR#1's own description states the initial implementation was "Hardened via a 7-way
  adversarial review" before it was ever opened as a PR — this is the author's own account of
  pre-PR self-review, not evidence of anyone else reviewing it; cite it the same way, as
  provenance you should be precise about (this skill's sibling projects' skills make the same
  distinction for author-written design notes).

**Use this to calibrate what to ask for**: if a PR is missing a stated test count or
fmt/clippy status where every predecessor included one, it's fair to ask for it by pointing at
the real pattern ("every PR here so far has stated its test count and clippy/fmt status —
worth adding for this one too"), not by claiming CONTRIBUTING.md requires it (it doesn't
exist).

## What this repo's real history does NOT give you

Be upfront about these rather than filling the gap with invented precedent:

- **No evidenced reviewer voice, disagreement, or back-and-forth** of any kind — the record
  is 4/4 same-day self-merges with no comments.
- **No evidenced position on backward compatibility, deprecation, or breaking CLI-flag
  changes** — nothing in the 4 PRs changed an existing flag's meaning; all four either add
  new flags or fix internal behavior.
- **No CI failure-triage precedent** (contrast with redisbench-admin PR#458, which has one) —
  no PR here has recorded a CI-red-but-unrelated judgment call.
- **No stray-file / dead-code nitpick precedent** — none evidenced either way.
- **No precedent for reviewing a version bump / release-process PR** beyond PR#4's own
  packaging fix, which was a same-day self-fix to a broken el9 RPM build, not a reviewed
  release-process discussion.

If a PR under review raises one of these, say plainly that this repo's own history doesn't
give a citable precedent, and reason about it on technical merits instead of manufacturing one.
