// 由 SVG 源生成位图图标(需 devDependency @resvg/resvg-js)。
//
//   node scripts/gen-icons.mjs
//
// 当前生成:macOS 菜单栏托盘模板图标(单色,alpha 决定显隐)。
//   icons/tray.svg → icons/tray.png(64px)
// tray-icon crate 会把图标固定缩放为 18pt 高,这里用 64px(>2x/3x)保证 Retina 锐利。
//
// 应用图标(dock / 安装包)另由 Tauri CLI 从彩色 PNG 生成:
//   pnpm tauri icon src/assets/lenz-logo.png
import { Resvg } from "@resvg/resvg-js";
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");

function rasterize(svgPath, outPath, width) {
  const svg = readFileSync(resolve(root, svgPath));
  const resvg = new Resvg(svg, {
    fitTo: { mode: "width", value: width },
    font: { loadSystemFonts: true },
    background: "rgba(0,0,0,0)",
  });
  writeFileSync(resolve(root, outPath), resvg.render().asPng());
  console.log(`✓ ${outPath} (${width}px)`);
}

rasterize("src-tauri/icons/tray.svg", "src-tauri/icons/tray.png", 64);
