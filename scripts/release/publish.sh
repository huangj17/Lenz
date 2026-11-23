#!/usr/bin/env bash
# Publishes collected installers to the GitHub Release for $RELEASE_TAG.
#
# Usage: publish.sh <artifact-directory>
# Env:   GH_TOKEN, RELEASE_TAG; ALLOW_OVERWRITE=true replaces assets of an already
#        published release (never set by the tag-triggered release workflow).
set -euo pipefail

input="${1:?artifact directory required}"
node scripts/release/collect-assets.mjs --input "$input" --output release

if is_draft=$(gh release view "$RELEASE_TAG" --json isDraft --jq '.isDraft' 2>/dev/null); then
  action=$(node scripts/release/release-policy.mjs action "$is_draft" "${ALLOW_OVERWRITE:-false}")
else
  action=$(node scripts/release/release-policy.mjs action none false)
  gh release create "$RELEASE_TAG" --verify-tag --draft --title "Lenz $RELEASE_TAG" --notes-file "docs/releases/$RELEASE_TAG.md"
fi
echo "Release $RELEASE_TAG: $action"

gh release upload "$RELEASE_TAG" release/* --clobber

latest_flag=$(gh release list --exclude-drafts --exclude-pre-releases --limit 1000 --json tagName --jq '.[].tagName' \
  | node scripts/release/release-policy.mjs latest-flag "$RELEASE_TAG")
gh release edit "$RELEASE_TAG" --draft=false "$latest_flag"
echo "Published $RELEASE_TAG ($latest_flag)"
