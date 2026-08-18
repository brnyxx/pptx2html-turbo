# Release Notes Workflow

This directory holds the human-maintained release-note inputs used before a tag or publish decision.

## Files

- `unreleased-draft.md` — the release body for the version that is prepared but not yet tagged.
- `pre-release-checklist.md` — the operator-facing checklist to run before tagging or publishing.
- `v2.0.0-validation.md` — the reproducibility boundary and recorded proxy evidence for the v2.0.0 release candidate.

## Intended Flow

1. Record shipped scope under `CHANGELOG.md` `Unreleased` while work is in flight, then promote it to a versioned heading when the release is prepared.
2. Update `unreleased-draft.md` so it can be copied into a GitHub release body with minimal editing.
3. Run `pre-release-checklist.md` against the current tree, including the npm dry-run path for the intended version line (`workflow_dispatch` when permissions allow it, otherwise local `npm publish --dry-run` from the prepared package directory).
4. Only after human approval, create the release tag and let `.github/workflows/release.yml` attach the validated artifacts.

## Important Note

The current release workflow still uses `generate_release_notes: true` for GitHub's automatic notes.
That means this directory is the human-curated source for release wording, but it is **not** consumed automatically by the workflow.
If you want the polished draft to appear in GitHub Releases, copy or adapt `unreleased-draft.md` when preparing the final release.
