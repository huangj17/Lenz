# Spec 06 · 截图翻译 (F2)

> 覆盖 PRD §2.2 F2。定义从快捷键触发到弹框显示的完整流程、各阶段状态与错误处理。

---

## 1. 流程总览

```
Alt+S
  │
  ▼
start_selection(trigger="screenshot")       [Rust]
  │  创建全屏选区蒙层窗口
  ▼
用户框选区域                                 [前端 selection 窗口]
  │  emit capture://region-selected { x,y,w,h }
  ▼
capture_screen_region(rect)                  [Rust core/capture]
  │  返回 CaptureResult { png, pixel_rect, scale_factor }
  ▼
run_ocr(input, opts)                         [Rust core/ocr]
  │  返回 OcrResult { blocks, detected_language }
  ▼
translate_text(texts=joined, ...)            [Rust core/translator]
  │  返回 TranslateResponse
  ▼
emit screenshot://result { ... }             [Rust → frontend popup]
  │
  ▼
popup 窗口展示原文 + 译文                    [前端 popup 窗口]
```

所有步骤在 Rust 主线程的 async task 中串行执行(非命令链路,由 Rust 内部协调)。

## 2. 详细步骤

### 2.1 快捷键响应

- Tauri 全局快捷键注册:`Alt+S`(macOS:`Option+S`)。
- handler 中直接调 `start_selection(TriggerKind::Screenshot)`,此为异步,不阻塞 UI。

### 2.2 选区完成

- 监听 `capture://region-selected` event,payload 含 `trigger = "screenshot"`。
- 若 `w < 4 || h < 4` → 视为点击取消,关闭选区窗口,流程结束。
- 同时关闭选区窗口(销毁)。

### 2.3 截图

- 调 `capture_screen_region(rect)`,逻辑见 [Spec 02](./02-capture.md)。
- 超时:5s。失败 → emit `screenshot://error { stage: "capture", ... }`。

### 2.4 OCR

- 调 `run_ocr`:
  - `preferred_languages`:取 `config.ocr.preferredLanguages`,若为空则由 OCR 引擎自动。
  - `min_confidence`:取 `config.ocr.minConfidence`(默认 0.5)。
- 结果合并:把所有 `block.text` 按阅读顺序拼接(竖排语言按列序,横排按行序)。
  - 拼接分隔符:同行 block 用空格,换行 block 用 `\n`。
  - 拼接文本作为 `translate_text` 的输入;同时保留原始 `blocks` 数组供弹框显示原文。
- OCR 结果为空 → emit `screenshot://error { stage: "ocr", code: "no_text" }`。

### 2.5 翻译

- 调 `translate_text`:
  - `source_lang = detected_language`(来自 OCR),若为 null 则让引擎自动检测。
  - `target_lang = config.translator.targetLang`。
- 翻译失败 → emit `screenshot://error { stage: "translate", ... }`。

### 2.6 结果推送

```ts
// screenshot://result 事件载荷
interface ScreenshotResult {
  original: string;          // OCR 拼接文本
  translation: string;       // 译文
  sourceLang: string | null;
  targetLang: string;
  engineId: string;
  region: { x: number; y: number; w: number; h: number };  // 逻辑坐标
  costMs: { capture: number; ocr: number; translate: number };
}
```

## 3. popup 窗口展示

popup 窗口监听 `screenshot://result` 与 `screenshot://error`。

布局(见 [Spec 09](./09-windows-ui.md)):
- 顶部语言标识行:`{sourceLang} → {targetLang}`。
- 原文区(可折叠,默认展开):滚动容器,字体略小。
- 分割线。
- 译文区:主字体,选中可复制。
- 底部工具栏:复制按钮、引擎名标识。

弹框定位:出现在选区 `region` 的正下方(`y + h + 12`),水平居中对齐选区,溢出则调整。

## 4. 错误 UI

| 阶段 | 用户可见提示 |
|---|---|
| capture 权限不足 | popup 展示"需要屏幕录制权限",含跳转系统设置按钮 |
| ocr no_text | popup 展示"未识别到文字,请重新框选" |
| translate 失败 | popup 展示 OCR 原文 + "翻译失败:{error}",含重试按钮 |
| 超时(总体 > 10s) | popup 展示"响应超时" |

## 5. 进度反馈

从选区完成到结果显示期间,popup 需立即出现(占位)并显示进度:

```
[截图中…] → [识别中…] → [翻译中…] → 结果
```

Rust 侧在关键节点 emit `screenshot://progress { stage: "capture"|"ocr"|"translate" }`。

## 6. 状态机

```
IDLE
  │ Alt+S
  ▼
SELECTING
  │ 框选完成
  ▼
CAPTURING → RECOGNIZING → TRANSLATING
  │(任一失败)           │(完成)
  ▼                     ▼
ERROR                SHOWING
                        │ 点击外部/ESC
                        ▼
                       IDLE
```

任何阶段按 ESC → 回 IDLE,取消正在进行的异步操作(通过 `CancellationToken`)。

## 7. 取消机制

- Rust 侧维护 `Option<CancellationToken>` per-session。
- 新的 F2 触发 → 取消旧 token,创建新 token。
- OCR 和翻译调用均传 token,在 `spawn_blocking` 完成后检查;HTTP 请求使用 `reqwest` timeout + `select!`。

## 8. 验收标准

- [ ] Alt+S → 进入截图模式 ≤ 300ms。
- [ ] 框选完成 → 弹框出现(含 "截图中" 进度) ≤ 500ms。
- [ ] 框选完成 → 完整结果显示 ≤ 3s(本地 OCR + 本地 LLM)。
- [ ] ESC 在任何阶段都能取消,不残留窗口。
- [ ] 弹框位置在选区正下方,不超出屏幕。
- [ ] 翻译失败时展示原文,不空白。

## 9. 测试点

- 框选单行 / 多行 / 混排文本。
- 框选无文字区域(纯图像)。
- 翻译引擎超时(设极短 timeout 模拟)。
- 连续两次 Alt+S,第一次未完成时触发第二次。
- 弹框出现后,再次框选 → 弹框内容刷新。
