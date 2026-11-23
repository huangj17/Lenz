import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

const STABLE_TAG = /^v(\d+)\.(\d+)\.(\d+)$/;

/** Parses a stable vMAJOR.MINOR.PATCH tag; any other tag never competes for Latest. */
export function parseTag(tag) {
  const match = STABLE_TAG.exec(tag ?? "");
  return match ? match.slice(1).map(Number) : null;
}

function compare(a, b) {
  for (let i = 0; i < 3; i += 1) {
    if (a[i] !== b[i]) return a[i] - b[i];
  }
  return 0;
}

/**
 * Only the highest stable version may be marked Latest: the in-app update check reads
 * `/releases/latest`, so re-publishing an older tag must never hide a newer release.
 */
export function shouldMarkLatest(tag, publishedTags) {
  const own = parseTag(tag);
  if (!own) throw new Error(`Release tag ${tag} is not a stable vMAJOR.MINOR.PATCH version.`);
  return publishedTags
    .map(parseTag)
    .filter(Boolean)
    .every((other) => compare(own, other) >= 0);
}

/**
 * Assets of a published release are what users already downloaded and verified against
 * SHA256SUMS. Only a missing or draft release may receive new uploads, unless the
 * operator explicitly asked to replace a published one.
 */
export function releaseAction(existingIsDraft, allowOverwrite) {
  if (existingIsDraft === null) return "create";
  if (existingIsDraft) return "update-draft";
  if (allowOverwrite) return "overwrite";
  throw new Error("The release is already published. Refusing to replace its installers; rerun with overwrite enabled to replace them deliberately.");
}

function parseDraftState(value) {
  if (value === "none") return null;
  if (value === "true" || value === "false") return value === "true";
  throw new Error(`Unknown release draft state: ${value}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const [command, ...args] = process.argv.slice(2);
    if (command === "latest-flag") {
      // Published tags arrive one per line on stdin (`gh release list ... --jq '.[].tagName'`).
      const published = readFileSync(0, "utf8").split(/\r?\n/).filter(Boolean);
      console.log(shouldMarkLatest(args[0], published) ? "--latest" : "--latest=false");
    } else if (command === "action") {
      console.log(releaseAction(parseDraftState(args[0]), args[1] === "true"));
    } else {
      throw new Error("Usage: release-policy.mjs latest-flag <tag> | action <true|false|none> <allowOverwrite>");
    }
  } catch (error) {
    console.error(`::error::${error.message}`);
    process.exit(1);
  }
}
