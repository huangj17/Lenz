import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
// Packaging runs under Node too, so the test exercises the actual manifest generator.
// @ts-expect-error The CLI is intentionally plain JavaScript for Windows PowerShell callers.
import { manifest, packageVersion } from "../scripts/windows/msix-package.mjs";
// @ts-expect-error See the CLI note above.
import { inspectPe } from "../scripts/windows/pe.mjs";

const identity = JSON.parse(readFileSync(new URL("../src-tauri/msix/identity.json", import.meta.url), "utf8"));
test("MSIX versions are bounded four-part integers and reject prerelease ambiguity", () => {
  assert.equal(packageVersion("0.1.0"), "0.1.0.0");
  assert.equal(packageVersion("1.2.3.4"), "1.2.3.4");
  for (const version of ["0.0.0", "1.2.65536", "1.2.3-beta.1", "1.2", "1.2.3.4.5"]) assert.throws(() => packageVersion(version));
});
test("package contract gives the real executable a full-trust identity and stable startup entry", () => {
  const xml = manifest({ version: "0.1.0", architecture: "x64", identity, productName: "Lenz", description: "OCR & translation" });
  assert.match(xml, /Name="app.lenz.desktop"/);
  assert.match(xml, /Executable="lenz.exe" EntryPoint="Windows.FullTrustApplication"/);
  assert.match(xml, /TaskId="LenzStartup" Enabled="false"/);
  assert.match(xml, /MinVersion="10.0.17763.0"/);
  assert.match(xml, /Name="Microsoft.WebView2"/);
  assert.match(xml, /OCR &amp; translation/);
  // Startup task ID must match the backend, otherwise the installed toggle would silently fail.
  const rust = readFileSync(new URL("../src-tauri/src/core/windows_package.rs", import.meta.url), "utf8");
  assert.ok(rust.includes('STARTUP_TASK_ID: &str = "' + identity.startupTaskId + '"'));
});
test("publisher subjects are safely encoded and unsupported architectures fail", () => {
  const input = { version: "1.0.0", architecture: "arm64", identity: { ...identity, publisher: 'CN=Test & "Team"' }, productName: "Lenz", description: "OCR" };
  assert.match(manifest(input), /CN=Test &amp; &quot;Team&quot;/);
  assert.throws(() => manifest({ ...input, architecture: "neutral" }));
});

test("packaging inspects PE architecture and DLL imports rather than trusting the filename", () => {
  const image = Buffer.alloc(1024);
  image.writeUInt16LE(0x5a4d, 0);
  image.writeUInt32LE(64, 0x3c);
  image.writeUInt32LE(0x4550, 64);
  image.writeUInt16LE(0x8664, 68);
  image.writeUInt16LE(1, 70);
  image.writeUInt16LE(240, 84);
  image.writeUInt16LE(0x20b, 88);
  image.writeUInt32LE(0x1000, 208);
  image.writeUInt32LE(0x1000, 340);
  image.writeUInt32LE(512, 344);
  image.writeUInt32LE(512, 348);
  image.writeUInt32LE(0x1040, 524);
  image.write("WebView2Loader.dll\0", 576);
  assert.deepEqual(inspectPe(image), { architecture: "x64", imports: ["WebView2Loader.dll"] });
  image.writeUInt16LE(0xaa64, 68);
  assert.equal(inspectPe(image).architecture, "arm64");
  image.writeUInt32LE(0x9999, 524);
  assert.throws(() => inspectPe(image));
  assert.throws(() => inspectPe(Buffer.from("renamed Linux binary")));
});
