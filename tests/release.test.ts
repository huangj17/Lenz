import assert from "node:assert/strict";
import test from "node:test";
import { mkdtemp, mkdir, writeFile, readFile, readdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createHash } from "node:crypto";
// @ts-expect-error Shared Node CLI has no TypeScript declaration.
import { collectAssets } from "../scripts/release/collect-assets.mjs";
// @ts-expect-error Shared Node CLI has no TypeScript declaration.
import { macosSigning, localSigning, selectLocalSigning, verifyApp } from "../scripts/release/macos-signing.mjs";

test("macOS test releases remain available but incomplete certificate credentials never fall back silently", () => {
  assert.deepEqual(macosSigning({}), { mode: "adhoc", identity: "-" });
  assert.deepEqual(macosSigning({ APPLE_SIGNING_IDENTITY: "-" }), { mode: "adhoc", identity: "-" });
  const signed = {
    APPLE_SIGNING_IDENTITY: "Developer ID Application: Lenz Fixture (AB12345678)",
    APPLE_CERTIFICATE: "controlled-test-certificate",
    APPLE_CERTIFICATE_PASSWORD: "controlled-test-password",
  };
  assert.deepEqual(macosSigning(signed), { mode: "developer-id", identity: signed.APPLE_SIGNING_IDENTITY });
  for (const key of Object.keys(signed)) {
    assert.throws(() => macosSigning({ ...signed, [key]: "" }), /Incomplete/);
  }
  assert.throws(() => macosSigning({ ...signed, APPLE_SIGNING_IDENTITY: "-" }), /full Developer ID/);
  assert.throws(() => macosSigning({ ...signed, APPLE_SIGNING_IDENTITY: "Developer ID Application: Fixture\nINJECTED=value (AB12345678)" }), /full Developer ID/);
});

test("local signing accepts a free persistent identity and rejects ephemeral identities", () => {
  assert.deepEqual(localSigning("Lenz Local Development"), { mode: "local", identity: "Lenz Local Development" });
  assert.deepEqual(localSigning("a".repeat(40)), { mode: "local", identity: "a".repeat(40) });
  for (const identity of [undefined, "", "-", "Lenz\nINJECTED=value"]) {
    assert.throws(() => localSigning(identity), /persistent local/);
  }
});

test("future local updates keep the stored fingerprint despite another signing identity in the environment", () => {
  const stored = "ab".repeat(20);
  assert.equal(selectLocalSigning(undefined, stored, "A different certificate").identity, stored);
  assert.equal(selectLocalSigning("Explicit certificate", stored, "A different certificate").identity, "Explicit certificate");
  assert.throws(() => selectLocalSigning(undefined, "-", "Fallback certificate"), /Stored local signing identity is invalid/);
  assert.throws(() => selectLocalSigning(undefined, null, "Fallback certificate"), /Stored local signing identity is invalid/);
});

test("signature verification reads requirements from stdout and rejects build-specific local signatures", () => {
  const signing = localSigning("Lenz Local Development");
  const codesign = (stdout: string, stderr: string, status = 0) => (_: string, args: string[]) =>
    args.includes("--verify") ? { status, stdout: "", stderr: "" } : { status: 0, stdout, stderr };
  assert.doesNotThrow(() => verifyApp("fixture.app", signing,
    codesign('designated => identifier "app.lenz.desktop" and certificate leaf = H"aabb"\n', "Authority=Lenz Local Development\nTeamIdentifier=not set\n")));
  assert.throws(() => verifyApp("fixture.app", signing,
    codesign('designated => cdhash H"aabb"\n', "Signature=adhoc\n")), /certificate-backed/);
  assert.throws(() => verifyApp("fixture.app", signing,
    codesign('designated => identifier "app.lenz.desktop"\n', "")), /certificate-backed/);
  assert.throws(() => verifyApp("fixture.app", signing,
    codesign('designated => identifier "app.lenz.desktop" and certificate leaf = H"aabb"\n', "Authority=Different Local Certificate\n")), /certificate-backed/);
  assert.throws(() => verifyApp("fixture.app", signing, codesign("", "", 1)), /verification failed/);
  const pinned = localSigning("ab".repeat(20));
  assert.doesNotThrow(() => verifyApp("fixture.app", pinned,
    codesign(`designated => identifier "app.lenz.desktop" and certificate leaf = H"${"ab".repeat(20)}"\n`, "Authority=Lenz Local Development\n")));
  assert.throws(() => verifyApp("fixture.app", pinned,
    codesign(`designated => identifier "app.lenz.desktop" and certificate leaf = H"${"cd".repeat(20)}"\n`, "Authority=Lenz Local Development\n")), /certificate-backed/);
});

test("release collection flattens nested Windows artifacts and hashes the actual payload", async () => {
  const root = await mkdtemp(join(tmpdir(), "lenz-release-"));
  try {
    const input = join(root, "input");
    const nested = join(input, "msix", "release");
    await mkdir(nested, { recursive: true });
    await writeFile(join(nested, "Lenz.msix"), "package payload");
    await writeFile(join(input, "Lenz.dmg"), "disk image");
    const setupName = "Lenz_0.1.2_windows_x64_setup.exe";
    await writeFile(join(nested, setupName), "EXE installation wizard");
    await writeFile(join(nested, "lenz.exe"), "unpackaged application, not an installer");
    await writeFile(join(nested, "private.pfx"), "not a release asset");
    const output = join(root, "output");
    assert.deepEqual(await collectAssets(input, output), ["Lenz.dmg", "Lenz.msix", setupName]);
    assert.deepEqual((await readdir(output)).sort(), ["Lenz.dmg", "Lenz.msix", setupName, "SHA256SUMS.txt"]);
    assert.equal(await readFile(join(output, "Lenz.msix"), "utf8"), "package payload");
    assert.match(await readFile(join(output, "SHA256SUMS.txt"), "utf8"), new RegExp(createHash("sha256").update("package payload").digest("hex") + "  Lenz.msix"));
    assert.match(await readFile(join(output, "SHA256SUMS.txt"), "utf8"), new RegExp(createHash("sha256").update("EXE installation wizard").digest("hex") + "  " + setupName));
    await assert.rejects(collectAssets(input, output), /must be empty/);
    await mkdir(join(input, "duplicate"));
    await writeFile(join(input, "duplicate", "Lenz.msix"), "conflicting payload");
    await assert.rejects(collectAssets(input, join(root, "ambiguous")), /Duplicate/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
