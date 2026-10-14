import { readFile, writeFile, copyFile, mkdir, readdir } from "node:fs/promises";
import { resolve, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { inspectPe } from "./pe.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const escapeXml = (value) => String(value).replaceAll("&", "&amp;").replaceAll('"', "&quot;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");

export function packageVersion(value) {
  if (!/^\d+\.\d+\.\d+(?:\.\d+)?$/.test(value)) throw new Error("MSIX requires a stable numeric version or an explicit four-part package version.");
  const parts = value.split(".").map(Number);
  if (parts.length === 3) parts.push(0);
  if (parts.some((part) => part > 65535) || parts.every((part) => part === 0)) throw new Error("Invalid MSIX version.");
  return parts.join(".");
}

export function manifest({ version, architecture, identity, productName, description }) {
  if (!["x64", "arm64"].includes(architecture)) throw new Error("Supported architectures: x64, arm64.");
  if (!/^[A-Za-z0-9.-]{3,50}$/.test(identity.name)) throw new Error("Invalid MSIX identity name.");
  if (!/^CN=.+/i.test(identity.publisher)) throw new Error("Publisher must match the signing certificate subject.");
  const x = escapeXml;
  return `<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
  xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
  xmlns:uap5="http://schemas.microsoft.com/appx/manifest/uap/windows10/5"
  xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
  xmlns:win32dependencies="http://schemas.microsoft.com/appx/manifest/externaldependencies"
  IgnorableNamespaces="uap uap5 rescap win32dependencies">
  <Identity Name="${x(identity.name)}" Publisher="${x(identity.publisher)}" Version="${packageVersion(version)}" ProcessorArchitecture="${architecture}" />
  <Properties><DisplayName>${x(productName)}</DisplayName><PublisherDisplayName>${x(identity.publisherDisplayName)}</PublisherDisplayName><Description>${x(description)}</Description><Logo>Assets/StoreLogo.png</Logo></Properties>
  <Resources><Resource Language="en-US" /><Resource Language="zh-CN" /></Resources>
  <Dependencies>
    <TargetDeviceFamily Name="Windows.Desktop" MinVersion="${x(identity.minWindowsVersion)}" MaxVersionTested="10.0.26100.0" />
    <win32dependencies:ExternalDependency Name="Microsoft.WebView2" Publisher="CN=Microsoft Windows, O=Microsoft Corporation, L=Redmond, S=Washington, C=US" MinVersion="1.1.1.1" Optional="false" />
  </Dependencies>
  <Applications>
    <Application Id="${x(identity.applicationId)}" Executable="lenz.exe" EntryPoint="Windows.FullTrustApplication">
      <uap:VisualElements DisplayName="${x(productName)}" Description="${x(description)}" Square150x150Logo="Assets/Square150x150Logo.png" Square44x44Logo="Assets/Square44x44Logo.png" BackgroundColor="transparent" />
      <Extensions>
        <uap5:Extension Category="windows.startupTask" Executable="lenz.exe" EntryPoint="Windows.FullTrustApplication">
          <uap5:StartupTask TaskId="${x(identity.startupTaskId)}" Enabled="false" DisplayName="${x(productName)}" />
        </uap5:Extension>
      </Extensions>
    </Application>
  </Applications>
  <Capabilities><Capability Name="internetClient" /><rescap:Capability Name="runFullTrust" /></Capabilities>
</Package>
`;
}

export async function prepare({ binary, output, architecture = "x64", publisher, name, version, loader }) {
  const config = JSON.parse(await readFile(join(root, "src-tauri/tauri.conf.json"), "utf8"));
  const identity = JSON.parse(await readFile(join(root, "src-tauri/msix/identity.json"), "utf8"));
  if (publisher) identity.publisher = publisher;
  if (name) identity.name = name;
  const packageVersionValue = packageVersion(version ?? config.version);
  const image = inspectPe(await readFile(binary));
  if (image.architecture !== architecture) throw new Error("The executable architecture does not match the MSIX manifest.");
  if (image.imports.some((name) => /^(?:vcruntime\d.*|msvcp\d.*|concrt\d.*|libgcc_s.*|libstdc\+\+.*)\.dll$/i.test(name))) {
    throw new Error("Build with a static runtime before packaging; the executable requires a separately installed C/C++ runtime.");
  }
  const needsLoader = image.imports.some((name) => name.toLowerCase() === "webview2loader.dll");
  if (needsLoader && !loader) throw new Error("This executable imports WebView2Loader.dll. Supply the matching DLL with --loader.");
  if (loader && inspectPe(await readFile(loader)).architecture !== architecture) throw new Error("WebView2Loader.dll architecture does not match the executable.");
  // Require an empty staging directory so arbitrary existing data is never erased.
  await mkdir(output, { recursive: true });
  if ((await readdir(output)).length) throw new Error("MSIX staging directory must be empty.");
  await copyFile(binary, join(output, "lenz.exe"));
  if (loader) await copyFile(loader, join(output, "WebView2Loader.dll"));
  await mkdir(join(output, "Assets"));
  for (const icon of ["StoreLogo.png", "Square44x44Logo.png", "Square150x150Logo.png"]) {
    await copyFile(join(root, "src-tauri/icons", icon), join(output, "Assets", icon));
  }
  await copyFile(join(root, "LICENSE"), join(output, "LICENSE"));
  await writeFile(join(output, "AppxManifest.xml"), manifest({
    version: packageVersionValue, architecture, identity, productName: config.productName,
    description: "Local OCR and screen translation / 本地 OCR 与屏幕翻译",
  }));
  return { ...identity, version: packageVersionValue, architecture, runtimeImports: image.imports, stagingDirectory: resolve(output) };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const { values } = parseArgs({ options: {
    binary: { type: "string" }, output: { type: "string" }, architecture: { type: "string" },
    publisher: { type: "string" }, name: { type: "string" }, version: { type: "string" }, loader: { type: "string" },
  } });
  if (!values.binary || !values.output) throw new Error("Specify --binary and --output.");
  console.log(JSON.stringify(await prepare(values)));
}
