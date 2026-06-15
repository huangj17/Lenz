# Spec 07 · 原图覆盖翻译 (F3)

> 覆盖 PRD §2.2 F3 与 §3.4(原图覆盖窗口)。定义透明悬浮窗的创建、bbox 定位、颜色采样与字号自适应。

---

## 1. 流程总览

```
Alt+F
  │
  ▼
start_selection(trigger="overlay")          [Rust]
  │
  ▼
用户框选区域                                 [前端 selection 窗口]
  │  emit capture://region-selected { trigger="overlay" }
  ▼
capture_screen_region(rect)                  [Rust]
  │
  ▼
run_ocr → OcrResult { blocks }              [Rust]
  │  每个 block 含 text + bbox(逻辑坐标)
  ▼
translate_batch(texts=[...])                 [Rust]
  │  返回与 blocks 等长的译文数组
  ▼
sample_colors(png, blocks)                   [Rust]
  │  为每个 block 采样背景色
  ▼
create_overlay_window(region, blocks+translations+colors) [Rust]
  │
  ▼
前端 overlay 窗口渲染                        [React overlay]
  │  按 bbox 绝对定位,渲染每个译文块
  ▼
ESC / 点击空白 → 销毁 overlay 窗口
```

## 2. OCR 结果聚合

F3 与 F2 的差异在于:F3 需要保留每个 `OcrBlock` 的 **独立 bbox**,不做行合并拼接。

- 过滤:去除 `confidence < min_confidence` 的 block。
- 不排序(保留 OCR 返回顺序,通常是阅读顺序)。
- 每个 block 独立送翻译(`translate_batch`)。

## 3. 批量翻译

- 调 `translate_batch(texts = blocks.map(|b| b.text))`,见 [Spec 04 §3.2](./04-translator.md)。
- 若引擎不支持批量(`supports_batch = false`),Manager 层自动串行调用。
- 返回结果与输入按索引对应,长度相同。
- 若某条翻译失败:对应 slot 填空字符串,不阻断其他 block 展示。

## 4. 背景颜色采样

文件:`core/capture.rs::sample_block_color(png: &RgbaImage, block_bbox: Rect) -> Rgba`

算法(中等方案,对应 PRD §2.2 F3 视觉效果):

1. 取 bbox 四条边各向内 2px 的像素行/列。
2. 计算全部采样像素的**中位数 R/G/B**(抗噪声,避免被文字像素主导)。
3. 返回 `[r, g, b, 255]`。

调用:Rust 侧在翻译完成后遍历所有 block,统一批量采样。

### 4.1 前景色决策

前端接收 `bgColor` 后自动决定文字颜色:

```ts
function foregroundColor(bg: [r, g, b]): string {
  const lum = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
  return lum > 0.5 ? "#000000" : "#ffffff";
}
```

## 5. Overlay 窗口规格

### 5.1 创建参数

```rust
tauri::WebviewWindowBuilder::new(app, "overlay", url)
    .transparent(true)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .focusable(false)
    .position(region.x as f64, region.y as f64)
    .inner_size(region.w as f64, region.h as f64)
    .build()?;

window.set_ignore_cursor_events(true)?;
```

- 窗口覆盖在框选区域(逻辑坐标位置与尺寸)。
- 鼠标事件完全穿透:下层应用可正常操作。
- 多次触发 F3:先销毁旧 overlay 窗口,再创建新窗口。

### 5.2 多屏支持

- 窗口通过 `set_position` 使用虚拟屏逻辑坐标放置,跨屏 Tauri 负责换算,无需手动处理。

## 6. 前端渲染(overlay 窗口)

### 6.1 数据注入

Rust 创建窗口后,通过 `emit_to("overlay", "overlay://init", payload)` 传递数据:

```ts
interface OverlayBlock {
  id: number;
  bbox: { x: number; y: number; w: number; h: number };  // 相对 region 左上角
  translation: string;
  bgColor: [number, number, number];
  fgColor: string;   // 由 Rust 计算好传过来
}

interface OverlayInitPayload {
  regionW: number;
  regionH: number;
  blocks: OverlayBlock[];
}
```

### 6.2 布局

- 窗口根节点:`position: relative; width: 100%; height: 100%; overflow: hidden`。
- 每个 block:

```css
.overlay-block {
  position: absolute;
  left: {bbox.x}px;
  top:  {bbox.y}px;
  width:  {bbox.w}px;
  height: {bbox.h}px;
  background-color: rgb({bgColor});
  color: {fgColor};
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  border-radius: 2px;
}
```

### 6.3 字号自适应

```ts
function fitFontSize(text: string, boxW: number, boxH: number): number {
  // 初始字号:min(boxH * 0.8, 32)
  // 逐步缩小:每次 -1px 直到 canvas measureText 宽度 ≤ boxW
  // 最小字号:8px
  const canvas = document.createElement("canvas");
  const ctx = canvas.getContext("2d")!;
  let size = Math.min(boxH * 0.8, 32);
  while (size >= 8) {
    ctx.font = `${size}px sans-serif`;
    if (ctx.measureText(text).width <= boxW) break;
    size -= 1;
  }
  return size;
}
```

长文本超出最小字号仍溢出 → `overflow: hidden` + `text-overflow: ellipsis`(横排)。

### 6.4 关闭交互

- ESC 键 → 前端 emit `overlay://close-requested` → Rust 销毁 overlay 窗口。
- 因为 `focusable: false`,ESC 需要由 **其他窗口**（如 popup 或 main）或 Rust 的全局快捷键捕获转发。
  - 方案:注册临时全局快捷键 `ESC`(仅在 overlay 存在时有效),收到后销毁窗口并解绑快捷键。
- 点击空白:因为鼠标穿透,无法在 overlay 自身捕获点击。ESC 是关闭的主路径。

## 7. Tauri Commands

```rust
#[tauri::command]
async fn show_overlay(
    region: Rect,
    blocks: Vec<OverlayBlockData>,
    app: AppHandle,
) -> Result<String /* overlay window label */, LenzError>;

#[tauri::command]
async fn close_overlay(label: String, app: AppHandle) -> Result<(), LenzError>;

pub struct OverlayBlockData {
    pub id: u32,
    pub bbox: Rect,          // 相对 region 左上角,逻辑坐标
    pub translation: String,
    pub bg_color: [u8; 3],
    pub fg_color: String,
}
```

`show_overlay` 由 Rust 内部流程调用,不由前端直接 invoke。

## 8. 性能目标

| 阶段 | 目标 |
|---|---|
| 框选完成 → overlay 显示 | ≤ 4s(本地 OCR + 本地 LLM) |
| 颜色采样(100 个 block) | ≤ 50ms |
| 前端渲染(100 个 block) | ≤ 100ms |

## 9. 验收标准

- [ ] 框选完成 → 覆盖层显示 ≤ 4s。
- [ ] 译文不超出原文 bbox(超出时缩小字号到最小值,超长则截断)。
- [ ] 覆盖层不拦截下层鼠标事件(鼠标穿透,可点击覆盖区域下方内容)。
- [ ] ESC 关闭覆盖层,下层窗口恢复正常交互。
- [ ] 多屏 + 不同 DPR 下,覆盖块位置与原文字对齐(误差 ≤ 2 逻辑像素)。
- [ ] 深底白字与浅底黑字场景均可识别译文。

## 10. 测试点

- 纯英文漫画气泡(不规则排布)。
- 日文纵排文字(bbox 高 > 宽)。
- 单个超大 block(整段落文字)。
- 密集小字(bbox < 20×10)。
- 翻译后部分 block 失败(其余 block 正常展示)。
- Retina + 标准屏双显示器,框选跨屏区域。

## 11. 开放问题

- [ ] ESC 全局快捷键与系统/其他应用的 ESC 是否冲突——考虑改为自定义快捷键或托盘菜单"关闭覆盖"作为后备。
- [ ] 竖排文字(日文 tategumi)的 block 顺序是否正确——取决于 OCR 引擎,MVP 先观察 Vision 行为。
- [ ] 背景采样中位数算法在渐变背景下效果一般——P2 考虑 blur + average 方案。
