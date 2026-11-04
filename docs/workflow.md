# Workflow

From `docs/design.md` DD-10. This is the trunk-based branching and commit
discipline for the Rust + Tauri rebuild.

## Branching

- `main` is the only long-lived branch.
- One short-lived branch per ticket → PR into `main` → **rebase-merge**, so
  every commit survives.
- Milestones are tags (`m1`, `m2`).
- Branch protection on `main`: PR required, linear history, CI required.

## Commit rules

1. One logical change per commit. If the message needs "and", split it.
2. Target about 200 changed lines or fewer, excluding lockfiles and
   generated code.
3. One crate per commit where possible; the scope names it, e.g.
   `feat(domain): add Reps newtype`.
4. Kinds never mix: dependency additions, generated code, formatting,
   refactors and behaviour changes are separate commits.
5. Every commit builds and passes tests. Check with
   `git rebase -x "cargo test"` before pushing.
6. The commit body explains why, and links the ADR or ticket.
7. A PR is one ticket, about 3–8 commits.
8. CI enforces the Conventional Commit format; size is a review habit.
