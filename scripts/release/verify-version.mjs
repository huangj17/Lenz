import { readFileSync, appendFileSync } from "node:fs";
import { parseArgs } from "node:util";
import { packageVersion } from "../windows/msix-package.mjs";

const json = (path) => JSON.parse(readFileSync(new URL(path, import.meta.url), "utf8"));
const npm = json("../../package.json");
const tauri = json("../../src-tauri/tauri.conf.json");
const cargo = readFileSync(new URL("../../src-tauri/Cargo.toml", import.meta.url), "utf8");
const rustVersion = cargo.split(/^\[package\]\s*$/m)[1]?.split(/^\[/m)[0]?.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
const { values } = parseArgs({ options: { tag: { type: "string" } } });
const tag = values.tag ?? (process.env.GITHUB_REF_TYPE === "tag" ? process.env.GITHUB_REF_NAME : `v${npm.version}`);
if (!/^v\d+\.\d+\.\d+$/.test(tag ?? "")) throw new Error("Release tags must use stable vMAJOR.MINOR.PATCH versions.");
const version = tag.slice(1);
packageVersion(version);
if ([npm.version, tauri.version, rustVersion].some((value) => value !== version)) {
  throw new Error(`Tag ${tag} must match package.json, Cargo.toml and tauri.conf.json (${npm.version}, ${rustVersion}, ${tauri.version}).`);
}
readFileSync(new URL(`../../docs/releases/${tag}.md`, import.meta.url));
if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `version=${version}\ntag=${tag}\n`);
console.log(`Release version verified: ${tag}`);
