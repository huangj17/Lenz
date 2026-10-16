import assert from "node:assert/strict";
import test from "node:test";
import { mkdtemp, mkdir, writeFile, readFile, readdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createHash } from "node:crypto";
// @ts-expect-error Shared Node CLI has no TypeScript declaration.
import { collectAssets } from "../scripts/release/collect-assets.mjs";

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
