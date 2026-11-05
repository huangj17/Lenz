import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { selectLocalSigning, verifyApp } from "./macos-signing.mjs";

try {
  if (process.platform !== "darwin") throw new Error("Local macOS signing requires macOS.");
  const identityFile = resolve(homedir(), "Library/Application Support/Lenz/Signing/identity.json");
  let savedIdentity;
  if (existsSync(identityFile)) {
    savedIdentity = JSON.parse(readFileSync(identityFile, "utf8")).identity;
    if (savedIdentity === undefined) throw new Error("Stored local signing identity is missing its original certificate fingerprint.");
  }
  const signing = selectLocalSigning(process.argv[2], savedIdentity, process.env.APPLE_SIGNING_IDENTITY);
  const identities = spawnSync("security", ["find-identity", "-v", "-p", "codesigning"], { encoding: "utf8" });
  if (identities.error || identities.status !== 0 || !identities.stdout.includes(signing.identity)) {
    throw new Error("The fixed local signing identity is unavailable. Restore that certificate and private key; this build will not create a replacement identity.");
  }
  const root = fileURLToPath(new URL("../../", import.meta.url));
  // Use an existing local identity. This entry point never creates certificates,
  // changes keychain trust, installs the app, or modifies its privacy permissions.
  const env = { ...process.env, APPLE_SIGNING_IDENTITY: signing.identity };
  for (const name of ["APPLE_CERTIFICATE", "APPLE_CERTIFICATE_PASSWORD", "APPLE_ID", "APPLE_PASSWORD",
    "APPLE_TEAM_ID", "APPLE_API_KEY", "APPLE_API_ISSUER", "APPLE_API_KEY_PATH"]) delete env[name];
  const result = spawnSync("pnpm", ["tauri", "build", "--bundles", "app", "--config", "src-tauri/tauri.release.macos.conf.json"],
    { cwd: root, env, stdio: "inherit" });
  if (result.error || result.status !== 0) throw new Error("Local macOS build failed. Check that the selected code-signing identity is available in your keychain.");
  const target = env.CARGO_TARGET_DIR ? resolve(root, "src-tauri", env.CARGO_TARGET_DIR) : resolve(root, "src-tauri/target");
  const app = resolve(target, env.CARGO_BUILD_TARGET ?? "", "release/bundle/macos/Lenz.app");
  verifyApp(app, signing);
  console.log(`Local certificate signature verified: ${app}`);
  console.log("Keep using this certificate and the same bundle identifier for subsequent local updates.");
} catch (error) {
  console.error(error instanceof Error ? error.message : "Local macOS build failed.");
  process.exitCode = 1;
}
