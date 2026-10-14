import { readdir, mkdir, readFile, copyFile, writeFile } from "node:fs/promises";
import { basename, join, resolve } from "node:path";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

export async function collectAssets(input, output) {
  const paths = [];
  async function walk(directory) {
    for (const entry of await readdir(directory, { withFileTypes: true })) {
      const path = join(directory, entry.name);
      if (entry.isDirectory()) await walk(path);
      else if (entry.isFile() && /\.(dmg|msix|zip)$/i.test(entry.name)) paths.push(path);
    }
  }
  await walk(input);
  if (!paths.length) throw new Error("No installer assets found.");
  const names = paths.map((path) => basename(path));
  if (new Set(names).size !== names.length) throw new Error("Duplicate installer filenames; refusing to overwrite assets.");
  await mkdir(output, { recursive: true });
  if ((await readdir(output)).length) throw new Error("Release asset directory must be empty.");
  const checksums = [];
  for (const path of paths.sort((a, b) => basename(a).localeCompare(basename(b)))) {
    const name = basename(path);
    await copyFile(path, join(output, name));
    checksums.push(`${createHash("sha256").update(await readFile(path)).digest("hex")}  ${name}`);
  }
  await writeFile(join(output, "SHA256SUMS.txt"), checksums.join("\n") + "\n");
  return names.sort();
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const { values } = parseArgs({ options: { input: { type: "string" }, output: { type: "string" } } });
  if (!values.input || !values.output) throw new Error("Specify --input and --output.");
  console.log(JSON.stringify(await collectAssets(values.input, values.output)));
}
