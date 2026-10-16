import assert from "node:assert/strict";
import test from "node:test";
import { mkdtemp, writeFile, readdir, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
// @ts-expect-error The packaging CLI is plain JavaScript for PowerShell callers.
import { prepareExe } from "../scripts/windows/exe-package.mjs";

function image(architecture: "x64" | "arm64", dependency?: string) {
  const data = Buffer.alloc(1024);
  data.writeUInt16LE(0x5a4d, 0);
  data.writeUInt32LE(64, 0x3c);
  data.writeUInt32LE(0x4550, 64);
  data.writeUInt16LE(architecture === "x64" ? 0x8664 : 0xaa64, 68);
  data.writeUInt16LE(1, 70);
  data.writeUInt16LE(240, 84);
  data.writeUInt16LE(0x20b, 88);
  data.writeUInt32LE(0x1000, 340);
  data.writeUInt32LE(512, 344);
  data.writeUInt32LE(512, 348);
  if (dependency) {
    data.writeUInt32LE(0x1000, 208);
    data.writeUInt32LE(0x1040, 524);
    data.write(dependency + "\0", 576);
  }
  return data;
}

test("standalone staging includes only the app, license and WebView2 helpers, without MSIX or certificates", async () => {
  const root = await mkdtemp(join(tmpdir(), "lenz-exe-"));
  try {
    const binary = join(root, "lenz.exe");
    await writeFile(binary, image("x64"));
    await writeFile(join(root, "Lenz.msix"), "not part of standalone setup");
    await writeFile(join(root, "private.pfx"), "must not be bundled");
    await writeFile(join(root, "Lenz-Test.cer"), "not needed");
    const output = join(root, "output");
    const result = await prepareExe({ binary, output });
    assert.equal(result.architecture, "x64");
    assert.equal(result.needsLoader, false);
    assert.deepEqual((await readdir(output)).sort(), ["Install-WebView2.ps1", "LICENSE", "MsixTools.ps1", "lenz.exe"]);
    assert.deepEqual(await readFile(join(output, "lenz.exe")), image("x64"));
    await assert.rejects(prepareExe({ binary, output }), /must be empty/);
  } finally { await rm(root, { recursive: true, force: true }); }
});

test("standalone staging rejects incompatible architecture and missing runtime dependencies", async () => {
  const root = await mkdtemp(join(tmpdir(), "lenz-exe-runtime-"));
  try {
    const binary = join(root, "lenz.exe");
    const output = join(root, "output");
    await writeFile(binary, image("arm64"));
    await assert.rejects(prepareExe({ binary, output, architecture: "x64" }), /architecture/);
    await writeFile(binary, image("x64", "vcruntime140.dll"));
    await assert.rejects(prepareExe({ binary, output }), /static C\/C\+\+ runtime/);
    await writeFile(binary, image("x64", "WebView2Loader.dll"));
    await assert.rejects(prepareExe({ binary, output }), /matching WebView2Loader/);
    const loader = join(root, "WebView2Loader.dll");
    await writeFile(loader, image("arm64"));
    await assert.rejects(prepareExe({ binary, output, loader }), /architecture/);
    await writeFile(loader, image("x64"));
    const result = await prepareExe({ binary, output, loader });
    assert.equal(result.needsLoader, true);
    assert.deepEqual(await readFile(join(output, "WebView2Loader.dll")), image("x64"));
  } finally { await rm(root, { recursive: true, force: true }); }
});
