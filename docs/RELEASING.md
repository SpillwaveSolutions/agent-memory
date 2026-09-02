# Releasing Agent Memory

How a version becomes a GitHub Release. The pipeline has guardrails because
a bare `git tag` from a stale local ref shipped v3.1.0 against `acc7294`
(Cargo.toml still `2.7.0`, none of Phases 54–57 present) on 2026-09-01. It
was public for 17 minutes. The four-binary archive check added in #37 held;
nothing else in the pipeline did.

## Procedure

Always tag an explicit SHA that is already on `origin/main`. Never tag
`HEAD` of a local branch.

```bash
git fetch origin
git checkout main
git pull --ff-only origin main

# Confirm the commit you intend to ship:
git log -1 --oneline
grep -A2 '\[workspace.package\]' Cargo.toml   # version must equal the tag minus v

# Tag that SHA, not a local name that might have drifted:
SHA="$(git rev-parse origin/main)"
git tag -a vX.Y.Z "$SHA" -m "Release vX.Y.Z"
git rev-parse 'vX.Y.Z^{commit}'   # must equal $SHA
git push origin vX.Y.Z
```

zsh: quote the peel (`'vX.Y.Z^{commit}'`). `^` is history expansion, and
with `interactivecomments` a bare `#` starts a comment — both will silently
rewrite the command. bash users can copy the block as-is.

Pushing a tag matching `v[0-9]+.[0-9]+.[0-9]+` starts
[`.github/workflows/release.yml`](../.github/workflows/release.yml).

`workflow_dispatch` with a version is an alternative. It still has to pass
the same guards, and it tags `$GITHUB_SHA` (the commit the workflow ran on)
rather than whatever happens to be `HEAD` on the runner.

## What the pipeline refuses

A `guard` job runs **before any platform build** and fails the release when:

1. The candidate commit is not an ancestor of `origin/main`. A tag cut from
   a feature branch, a stale local ref that was never merged, or a rewritten
   history cannot ship.
2. `workspace.package.version` in the root `Cargo.toml` is not exactly the
   tag minus `v`. This is the check that would have stopped the 2026-09-01
   incident: tag `v3.1.0`, crate version `2.7.0`.
3. `CHANGELOG.md` has no `## vX.Y.Z` section. Release notes are that section,
   not GitHub's auto-list of PR titles.

After the builds, the publish job additionally:

- Runs only if **every** matrix build succeeded. A missing platform fails
  the release instead of publishing three archives. (`if: success()`, not
  `if: always()`.)
- Requires all five archives (`linux-x86_64`, `linux-aarch64`,
  `macos-x86_64`, `macos-aarch64`, `windows-x86_64`) to be present.
- Attaches `SHA256SUMS.txt`.

## Dry-run (how to verify the guards live)

The tag pattern does **not** match prerelease suffixes, so `v9.9.9-test`
will not even start the workflow. Do not push a real `v9.9.9` tag to try
it — if the guards ever regress, that would publish.

To exercise the guards against GitHub without publishing:

1. `workflow_dispatch` with version `9.9.9` and **`dry_run: true`** from a
   feature branch. The guard job must fail (`Cargo.toml` is not 9.9.9, and
   the commit is not on main). Nothing is tagged or published.
2. The same dispatch from `main` with a version that disagrees with
   `Cargo.toml` must fail the version check.

The unit tests in `scripts/release-guards-test.sh` cover both failure
modes locally and run on every PR (`Release Guard Scripts` in `ci.yml`).

## Version bump checklist

Before tagging:

- [ ] `workspace.package.version` in `Cargo.toml` equals the intended tag
- [ ] `CHANGELOG.md` has a `## vX.Y.Z` section whose body is true of the
      tagged commit
- [ ] `task pr-precheck` was green on the PR that landed on main
- [ ] Tag with the explicit-SHA form above, not `git tag -a vX.Y.Z`
