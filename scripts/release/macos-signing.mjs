import { appendFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";

/** Preserve test releases; use Developer ID whenever credentials are configured. */
export function macosSigning(env) {
  const identity = env.APPLE_SIGNING_IDENTITY?.trim();
  const certificate = env.APPLE_CERTIFICATE?.trim();
  const password = env.APPLE_CERTIFICATE_PASSWORD;
  const hasCertificate = !!certificate;
  const hasPassword = typeof password === "string" && password.length > 0;
  if (!hasCertificate && !hasPassword && (!identity || identity === "-")) {
    return { mode: "adhoc", identity: "-" };
  }
  if (!hasCertificate || !hasPassword || !identity) {
    throw new Error("Incomplete macOS signing configuration: set all three APPLE_CERTIFICATE, APPLE_CERTIFICATE_PASSWORD and APPLE_SIGNING_IDENTITY secrets. Refusing to fall back to ad-hoc signing.");
  }
  if (!/^Developer ID Application: [^\r\n]+ \([A-Z0-9]{10}\)$/.test(identity)) {
    throw new Error("APPLE_SIGNING_IDENTITY must be the full Developer ID Application certificate name, including its team ID.");
  }
  return { mode: "developer-id", identity };
}

export function localSigning(identity) {
  const name = identity?.trim();
  if (!name || name === "-" || /[\r\n]/.test(name)) {
    throw new Error("Specify a persistent local code-signing certificate name or SHA-1 identity. Ad-hoc signing cannot preserve the update identity.");
  }
  return { mode: "local", identity: name };
}

export function selectLocalSigning(explicitIdentity, storedIdentity, environmentIdentity) {
  if (storedIdentity !== undefined && (typeof storedIdentity !== "string" || !/^[a-f\d]{40}$/i.test(storedIdentity))) {
    throw new Error("Stored local signing identity is invalid. Restore its original certificate fingerprint before building.");
  }
  return localSigning(explicitIdentity ?? storedIdentity ?? environmentIdentity);
}

export function verifyApp(appPath, signing, run = spawnSync) {
  const verification = run("codesign", ["--verify", "--deep", "--strict", appPath], { encoding: "utf8" });
  if (verification.error || verification.status !== 0) throw new Error("macOS application signature verification failed.");
  // codesign writes display information to stderr, even on success.
  const details = run("codesign", ["-d", "--verbose=4", "-r-", appPath], { encoding: "utf8" });
  if (details.error || details.status !== 0) throw new Error("Could not inspect macOS signature.");
  const output = `${details.stdout}\n${details.stderr}`;
  if (signing.mode === "developer-id" && (
    !output.split("\n").includes(`Authority=${signing.identity}`)
    || !/^TeamIdentifier=[A-Z0-9]{10}$/m.test(output)
    || /designated =>.*cdhash/.test(output)
  )) {
    throw new Error("Application signature does not have the configured stable Developer ID identity.");
  }
  if (signing.mode === "adhoc" && !/^Signature=adhoc$/m.test(output)) {
    throw new Error("Expected an explicitly requested ad-hoc test signature.");
  }
  if (signing.mode === "local" && (
    /^Signature=adhoc$/m.test(output)
    || !/designated =>.*certificate/.test(output)
    || /designated =>.*cdhash/.test(output)
    || (!/^[a-f\d]{40}$/i.test(signing.identity)
      && !output.split("\n").includes(`Authority=${signing.identity}`))
    || (/^[a-f\d]{40}$/i.test(signing.identity)
      && !output.toLowerCase().includes(`certificate leaf = h"${signing.identity.toLowerCase()}"`))
  )) {
    throw new Error("Local application must have a certificate-backed designated requirement, not a build-specific identity.");
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const signing = macosSigning(process.env);
    if (process.argv[2] === "--verify") {
      if (!process.argv[3]) throw new Error("Missing .app path for signature verification.");
      verifyApp(process.argv[3], signing);
    } else if (process.env.GITHUB_ENV) {
      await appendFile(process.env.GITHUB_ENV, `APPLE_SIGNING_IDENTITY=${signing.identity}\n`);
    }
    if (signing.mode === "adhoc") {
      console.log("::warning::Test DMG uses ad-hoc signing. Replacing it may require granting Accessibility and Screen Recording permissions again.");
    } else {
      console.log("macOS Developer ID signing configuration validated.");
    }
  } catch (error) {
    // Never print the environment, certificate, password, or subprocess buffers.
    console.error(error instanceof Error ? error.message : "macOS signing validation failed.");
    process.exitCode = 1;
  }
}
