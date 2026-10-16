import { readFile, copyFile, mkdir, readdir } from "node:fs/promises";
import { resolve, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { inspectPe } from "./pe.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");

export async function prepareExe({ binary, output, architecture = "x64", loader }) {
  const config = JSON.parse(await readFile(join(root, "src-tauri/tauri.conf.json"), "utf8"));
  if (!/^\d+\.\d+\.\d+$/.test(config.version) || config.version.split(".").some((part) => Number(part) > 65535)) {
    throw new Error("EXE setup requires a stable three-part version bounded to 65535.");
  }
  if (!["x64", "arm64"].includes(architecture)) throw new Error("Supported architectures: x64, arm64.");
  const image = inspectPe(await readFile(binary));
  if (image.architecture !== architecture) throw new Error("The executable architecture does not match the EXE setup.");
  if (image.imports.some((name) => /^(?:vcruntime\d.*|msvcp\d.*|concrt\d.*|libgcc_s.*|libstdc\+\+.*)\.dll$/i.test(name))) {
    throw new Error("Build with a static C/C++ runtime before packaging the EXE setup.");
  }
  const needsLoader = image.imports.some((name) => name.toLowerCase() === "webview2loader.dll");
  if (needsLoader && !loader) throw new Error("Supply the matching WebView2Loader.dll with --loader.");
  if (loader && inspectPe(await readFile(loader)).architecture !== architecture) throw new Error("WebView2Loader.dll architecture does not match the executable.");
  await mkdir(output, { recursive: true });
  if ((await readdir(output)).length) throw new Error("EXE staging directory must be empty.");
  // Explicit payload only: no Appx manifest, MSIX, signing certificate or private key.
  await copyFile(binary, join(output, "lenz.exe"));
  if (needsLoader) await copyFile(loader, join(output, "WebView2Loader.dll"));
  await copyFile(join(root, "LICENSE"), join(output, "LICENSE"));
  for (const helper of ["Install-WebView2.ps1", "MsixTools.ps1"]) {
    await copyFile(join(root, "scripts/windows", helper), join(output, helper));
  }
  return { version: config.version, productVersion: `${config.version}.0`, architecture, needsLoader, stagingDirectory: resolve(output) };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const { values } = parseArgs({ options: {
    binary: { type: "string" }, output: { type: "string" }, architecture: { type: "string" }, loader: { type: "string" },
  } });
  if (!values.binary || !values.output) throw new Error("Specify --binary and --output.");
  console.log(JSON.stringify(await prepareExe(values)));
}
