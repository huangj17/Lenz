// 由 SVG 源生成位图图标(需 devDependency @resvg/resvg-js)。
//
//   node scripts/gen-icons.mjs
//
// 应用图标:复用透明品牌标记,加上冰蓝渐变圆角底板。
//   src/assets/lenz-logo.svg → src/assets/lenz-logo.png(1024px)
// 菜单栏托盘模板图标(单色,alpha 决定显隐):
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

function rasterize(svg, outPath, width) {
  const resvg = new Resvg(svg, {
    fitTo: { mode: "width", value: width },
    font: { loadSystemFonts: true },
    background: "rgba(0,0,0,0)",
  });
  writeFileSync(resolve(root, outPath), resvg.render().asPng());
  console.log(`✓ ${outPath} (${width}px)`);
}

const logo = readFileSync(resolve(root, "src/assets/lenz-logo.svg"), "utf8");
const appIcon = `
<svg width="1024" height="1024" viewBox="0 0 1024 1024" fill="none" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <linearGradient id="app-background" x1="160" y1="0" x2="864" y2="1024" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#E6F9FF"/>
      <stop offset="0.5" stop-color="#C9E3FF"/>
      <stop offset="1" stop-color="#C8D0FF"/>
    </linearGradient>
  </defs>
  <rect width="1024" height="1024" rx="224" fill="url(#app-background)"/>
  <rect x="2" y="2" width="1020" height="1020" rx="222" stroke="#fff" stroke-opacity="0.65" stroke-width="4"/>
  <svg x="160" y="160" width="704" height="704" viewBox="0 0 96 96">
    ${logo.replace(/<svg\b[^>]*>/, "").replace(/<\/svg>\s*$/, "")}
  </svg>
</svg>`;

rasterize(appIcon, "src/assets/lenz-logo.png", 1024);
rasterize(readFileSync(resolve(root, "src-tauri/icons/tray.svg")), "src-tauri/icons/tray.png", 64);
