import { Resvg } from "@resvg/resvg-js";
import { writeFileSync } from "node:fs";

// Synthetic test data; no user screenshot or font file is shipped.
const svg = '<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="220"><rect width="1024" height="220" fill="white"/><text x="60" y="130" font-family="Arial" font-size="64" fill="black">LENZ OCR TEST 12345</text></svg>';
const image = new Resvg(svg, { font: { loadSystemFonts: true } }).render().asPng();
writeFileSync(new URL("../../src-tauri/resources/OcrSmoke.png", import.meta.url), image);
