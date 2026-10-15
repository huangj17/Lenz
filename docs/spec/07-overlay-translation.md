# Spec 07 · 原图覆盖翻译 (F3)

> 覆盖 PRD §2.2 F3 与 §3.4(原图覆盖窗口)。定义可交互翻译蒙层、选区截图、bbox 定位、颜色采样与关闭交互。

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
  │  先保证选区蒙层离屏;抓屏前不显示新覆盖层
  ▼
create_overlay_window(image, progress="ocr") [Rust → React]
  │  立即显示截图、loading 和 × / Esc;不等待识别与翻译
  │
  ▼
run_ocr → OcrResult { blocks }              [Rust]
  │  每个 block 含 text + bbox(逻辑坐标)
  ▼
translate_batch(texts=[...])                 [Rust]
  │  更新进度为 translate;返回与 blocks 等长的译文数组
  ▼
sample_colors(png, blocks)                   [Rust]
  │  为每个 block 采样背景色
  ▼
update_overlay(progress="complete", blocks+translations+colors) [Rust]
  │
  ▼
前端 overlay 窗口渲染                        [React overlay]
  │  按 bbox 绝对定位,渲染每个译文块
  ▼
× / 窗口内 Esc / 托盘关闭覆盖层 → 取消会话并销毁 overlay 窗口
```

## 2. OCR 结果聚合

F3 与 F2 的差异在于:F3 需要保留每个 `OcrBlock` 的 **独立 bbox**,不做行合并拼接。

默认 OCR 为系统/百度时使用该服务的含坐标结果;默认为视觉模型时继续使用系统 OCR,不向视觉模型发送截图。路由见 [Spec 03 §2.2](./03-ocr.md#22-服务选择与配置)。百度按服务 `timeoutMs` 执行,行坐标复用物理像素→屏幕逻辑坐标转换,使用真实置信度过滤。

截图独立限时 10s;OCR 共用 [Spec 03 §7](./03-ocr.md#7-性能) 的独立预算,Vision 首次使用一组语言/模式允许 60s 初始化,成功后为 10s。失败提示区分截图和文字识别阶段,日志包含各阶段耗时。首次初始化不受 ≤4s 的热状态性能目标约束。

- 过滤:macOS 去除 `confidence < min_confidence` 的 block;Windows 不用合成置信度过滤。
- 不排序(保留 OCR 返回顺序,通常是阅读顺序)。
- 每个 block 独立送翻译(`translate_batch`)。

## 3. 批量翻译

- 调 `translate_batch(texts = blocks.map(|b| b.text))`,见 [Spec 04 §3.2](./04-translator.md)。
- 若引擎不支持批量(`supports_batch = false`),overlay 编排逐块调用;批量失败也逐块回退。
- 返回结果与输入按索引对应,长度相同。
- 若某条翻译失败:对应 slot 填空字符串,不阻断其他 block 展示。

## 4. 背景颜色采样

文件:`core/overlay.rs::sample_color`

算法(中等方案,对应 PRD §2.2 F3 视觉效果):

1. 取 bbox 四条边各向内 2px 的像素行/列。
2. 计算全部采样像素的**中位数 R/G/B**(抗噪声,避免被文字像素主导)。
3. 返回 `[r, g, b, 255]`。

调用:Rust 侧在翻译完成后遍历所有 block,统一批量采样。

### 4.1 前景色决策

Rust 根据采样亮度计算 fgColor,前端直接渲染。计算规则:

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
    .focusable(true)
    .accept_first_mouse(true)
    .resizable(false)
    .position(viewport.x as f64, viewport.y as f64)
    .inner_size(viewport.w as f64, viewport.h as f64)
    .build()?;

// 前端 DOM 就绪后 show() + set_focus()。
```

- 窗口覆盖来源显示器,背景半透明变暗;选区截图与译文保留原逻辑坐标。
- 蒙层接收鼠标和键盘,避免误操作下层应用;关闭后恢复下层交互。
- 工具条包含标题、Esc 提示和 ×,优先放在截图外并夹紧到屏幕内。
- 多次触发 F3:先销毁旧 overlay 窗口,再创建新窗口。

### 5.2 多屏支持

- 复用 capture 裁剪区域及来源屏 scaleFactor。macOS 使用逻辑位置/尺寸,先设置尺寸再设置位置,避免 AppKit 调整尺寸时移动窗口原点;Windows 用同一来源屏比例还原物理位置/尺寸,避免已有窗口 DPI 造成错位。block 相对捕获区域定位,保留负坐标屏幕。

## 6. 前端渲染(overlay 窗口)

### 6.1 数据注入

截图完成后 Rust 将包含截图和 `ocr` 进度的 OverlayPayload 保存于 managed state,立即创建隐藏窗口,与 OCR/翻译并行。前端先订阅当前 WebView 的 `overlay://updated`,再调用 `get_overlay_payload`;快照期间到达的更新按 generation/revision 合并,旧快照不能回退新进度。事件仅传进度/译文,不重复传截图。

首次载荷对应 DOM 提交后的 effect 直接调用一次 `overlay_ready(generation)` 显示并聚焦;后续进度只更新同一个窗口,不重复抢焦点。隐藏原生 WebView 可能暂停 `requestAnimationFrame`,不能等待动画帧才通知显示。日志记录窗口创建与显示,不记录译文。

```ts
interface OverlayPayload {
  generation: number;
  scaleFactor: number;
  region: { x: number; y: number; w: number; h: number };
  viewport: { x: number; y: number; w: number; h: number };
  imageDataUrl: string; // 仅用户选区 PNG,不落盘,关闭释放
  progress: {
    revision: number;
    phase: "ocr" | "translate" | "complete" | "failed";
    blocks: { bbox: { x: number; y: number; w: number; h: number }; translation: string; bgColor: [number, number, number]; fgColor: string }[];
    error: LenzError | null;
  };
}
```

独立 ScreenshotSession 负责取消和有效性检查。OCR/翻译期间保留截图,工具条显示相应 loading,关闭入口一直可用;失败停止 loading 并在当前蒙层提示,抓屏失败才回主窗口。关闭立即取消等待中的任务并清空载荷;状态更新同时校验 generation 与有效会话,旧结果不能重新显示已关闭的覆盖层。卸载时解绑事件和键盘监听。

### 6.2 布局

- 窗口根节点:`position: relative; width: 100%; height: 100%; overflow: hidden`。
- 截图位于 `(region.x - viewport.x, region.y - viewport.y)`,每个 block 在截图容器内定位。
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

- × 与当前窗口的 Escape 都调用异步 `close_overlay`,取消会话、释放截图/译文并销毁窗口。
- 显示后聚焦蒙层,由 WebView 的 `keydown` 捕获 Escape;不注册或注销全局 Escape。
- 关闭请求去重,失败显示中英错误并允许重试;组件卸载时清理键盘监听。
- 原生窗口关闭请求也转到异步关闭入口,托盘「关闭覆盖层」保留。
- 菜单栏图标或“打开设置”先关闭蒙层再聚焦主窗口,避免主窗口在蒙层后夺走键盘焦点。
- 2026-10-14 旧版卡死采样:全局快捷键插件持有 shortcuts mutex 调用 Esc 回调,回调内 `unregister` 再次获取同一锁,锁住 macOS 主线程。删除该路径以消除重入死锁。

## 7. Tauri Commands

- `show_overlay(app, payload: OverlayPayload) -> Result<(), LenzError>`:创建覆盖层。
- `close_overlay(app) -> Result<(), LenzError>`:取消流程、销毁窗口并清空载荷,发 `overlay://closed { id: "overlay" }`。
- `get_overlay_payload(app) -> Option<OverlayPayload>`、`overlay_ready(app, generation)`:挂载握手。
- `overlay://updated { generation, progress }`:定向发送给 overlay WebView 的阶段/结果更新,持久于 managed state 供挂载读取。

主入口为工具条 × 和窗口内 Esc;不占用系统全局 Escape。截图前关闭旧覆盖层并取消旧会话。

## 8. 性能目标

| 阶段 | 目标 |
|---|---|
| 框选完成 → 首次可交互蒙层 | 热状态目标 ≤ 1s,截图完成即显示,不等待 OCR/翻译 |
| 框选完成 → 译文显示 | ≤ 4s(本地 OCR + 本地 LLM 热状态);远端服务耗时另计 |
| 颜色采样(100 个 block) | ≤ 50ms |
| 前端渲染(100 个 block) | ≤ 100ms |

## 9. 验收标准

- [ ] 框选后截图完成即显示可交互蒙层,本地热状态首次反馈目标 ≤ 1s。
- [ ] 慢 OCR/翻译期间有阶段 loading,完成后同窗原位更新;失败不会永久停在 loading。
- [ ] 识别中/翻译中均可 × / Esc 取消,迟到结果不能重新显示蒙层。
- [ ] 译文不超出原文 bbox(超出时缩小字号到最小值,超长则截断)。
- [ ] 蒙层工具条 × 可关闭,没有鼠标穿透,关闭后下层窗口正常交互。
- [ ] ESC 关闭覆盖层,下层窗口恢复正常交互。
- [ ] 多屏 + 不同 DPR 下,覆盖块位置与原文字对齐(误差 ≤ 2 逻辑像素)。
- [ ] 深底白字与浅底黑字场景均可识别译文。

## 10. 测试点

- 纯英文漫画气泡(不规则排布)。
- 日文纵排文字(bbox 高 > 宽)。
- 单个超大 block(整段落文字)。
- 密集小字(bbox < 20×10)。
- 翻译后部分 block 失败(其余 block 正常展示)。
- 快照加载期间先到进度事件、旧事件迟到、处理中关闭/连续框选。
- Retina + 标准屏双显示器,框选跨屏区域。

## 11. 开放问题

- [ ] 双平台多屏切换后的焦点恢复和本地 Esc 验收。
- [ ] 竖排文字(日文 tategumi)的 block 顺序是否正确——取决于 OCR 引擎,MVP 先观察 Vision 行为。
- [ ] 背景采样中位数算法在渐变背景下效果一般——P2 考虑 blur + average 方案。
