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
emit screenshot://translating { ... }        [Rust → frontend popup]
  │  原文已知、译文未出 → popup 显示原文 + 引擎行转圈
  ▼
translate_text(texts=joined, ...)            [Rust core/translator]
  │  返回 TranslateResponse
  ▼
emit screenshot://result { ... }             [Rust → frontend popup]
  │
  ▼
popup 窗口展示原文 + 译文                    [前端 popup 窗口]
```

截图/OCR/符合折叠策略的主服务翻译由 Rust 异步任务协调。弹窗收到 translating 后,在相同后端会话中按折叠策略并行请求其余已启用服务;各结果独立更新,不等待最慢服务。共享会话的取消覆盖所有请求,见 [spec 04 §2.1](./04-translator.md#21-弹窗多服务对比)。

OCR 使用配置中的默认服务(系统/百度/视觉模型),见 [Spec 03 §2.2–2.5](./03-ocr.md#22-服务选择与配置)。视觉模型返回的 `text` 直接作为原文,保留段落;系统/百度的 `blocks` 仍通过 `join_blocks` 合并。OCR 第三方服务超时按该服务的 `timeoutMs`,不受系统热状态 10s 预算限制。

## 2. 详细步骤

### 2.1 快捷键响应

- Tauri 全局快捷键注册:`Alt+S`(macOS:`Option+S`)。
- handler 中直接调 `start_selection(TriggerKind::Screenshot)`,此为异步,不阻塞 UI。

### 2.2 选区完成

- 监听 `capture://region-selected` event,payload 含 `trigger = "screenshot"`。
- 若 `w < 4 || h < 4` → 视为点击取消,关闭选区窗口,流程结束。
- 关闭选区窗口(销毁),保持 popup 和划词图标隐藏;抓屏完成前不显示翻译浮层。

### 2.3 截图

- 调 `capture_screen_region(rect)`,逻辑见 [Spec 02](./02-capture.md)。
- 截图独立限时 10s;OCR 的独立预算见 [Spec 03 §7](./03-ocr.md#7-性能),不扣减截图或 popup 握手耗时。翻译另按引擎 `timeoutMs` 执行。截图失败 → emit `screenshot://error { stage: "capture", ... }`。
- 等选区蒙层离屏后抓取;拿到图片后才显示 popup,避免大选区截到自己的翻译框。

### 2.4 OCR

- 调 `run_ocr`:
  - `preferred_languages`:取 `config.ocr.preferredLanguages`,若为空则由 OCR 引擎自动。
  - `min_confidence`:取 `config.ocr.minConfidence`(默认 0.5)。
- 结果合并:把所有 `block.text` 按阅读顺序拼接(竖排语言按列序,横排按行序)。
  - 拼接分隔符:同行 block 用空格,相邻行用 `\n`;行间垂直空隙超过一整行高时判为段落分隔,保留一个空行 `\n\n`(尽量保留原文分段)。
  - 拼接文本作为 `translate_text` 的输入;同时保留原始 `blocks` 数组供弹框显示原文。
- OCR 结果为空 → emit `screenshot://error { stage: "ocr", code: "no_text" }`。

### 2.5 翻译

- 进入翻译阶段前(原文已知、译文未出)先 emit `screenshot://translating`(载荷见下),
  让 popup 立即渲染「原文 + 语言条 + 引擎行」完整布局、仅引擎行右侧转圈,而非整块骨架。
- 调 `translate_text`:
  - `source_lang = detected_language`(来自 OCR),若为 null 则让引擎自动检测。
  - `target_lang = config.translator.targetLang`。
- 原生主服务翻译失败 → emit `screenshot://error { stage: "translate", ... }`,只更新该服务错误行;其他服务继续返回。每项失败均可单独重试,成功项保持可复制。

```ts
// screenshot://translating 事件载荷(原文已知、译文未出)
interface ScreenshotTranslating {
  sessionId: number;
  original: string;
  sourceLang: string | null;   // F2 为 OCR 检测值;F1 划词为 null
  targetLang: string;
  engineId: string;            // 实际主引擎;全部折叠跳过时为空
  requestId: string | null;    // 独立取消主服务;无自动请求时为 null
}
```

> F1 划词翻译(spec 05)无截图/OCR 阶段,直接在翻译前 emit 本事件;两条路径共用
> popup 的同一套「原文 + 引擎行转圈」加载布局。

### 2.6 结果推送

```ts
// screenshot://result 事件载荷
interface ScreenshotResult {
  sessionId: number;
  original: string;          // OCR 拼接文本
  translation: string;       // 译文
  sourceLang: string | null;
  targetLang: string;
  engineId: string;
  region: { x: number; y: number; w: number; h: number };  // 逻辑坐标
  cost: { capture: number; ocr: number; translate: number };
}
```

## 3. popup 窗口展示

popup 窗口监听 `screenshot://progress`、`screenshot://translating`、`screenshot://result` 与 `screenshot://error`。

布局(见 [Spec 09](./09-windows-ui.md)):
- 顶部语言标识行:`{sourceLang} → {targetLang}`。
- 原文区(可折叠,默认展开):滚动容器,字体略小。
- 分割线。
- 译文区:按已启用服务的设置顺序逐项显示,每行独立加载/错误/耗时/复制/重试;整块列表滚动。
- 底部工具栏:单服务显示引擎名,多服务显示服务数量;固定保留 Esc 提示。

弹框定位:出现在选区 `region` 的正下方(`y + h + 12`),水平居中对齐选区,溢出则调整。

## 4. 错误 UI

| 阶段 | 用户可见提示 |
|---|---|
| capture 权限不足 | popup 展示"需要屏幕录制权限",含跳转系统设置按钮 |
| ocr no_text | popup 展示"未识别到文字,请重新框选" |
| translate 失败 | 保留原文及其他服务结果,仅对应服务行展示 "翻译失败:{error}" 和重试按钮 |
| 截图/OCR 超过各自预算或引擎超过 `timeoutMs` | popup 区分"截图超时"、"文字识别超时"和翻译"响应超时";翻译超时保留原文并允许重试 |

## 5. 进度反馈

抓屏完成后 popup 出现(占位)并显示进度;抓屏期间保持隐藏:

```
[抓屏,浮层隐藏] → [识别中…] → [翻译中(原文 + 引擎行转圈)] → 结果
```

- `ocr` 阶段(尚无文本):emit `screenshot://progress { sessionId, stage }`,popup 显示整块骨架占位。
- `translate` 阶段(原文已知):改 emit `screenshot://translating`(见 §2.5),popup 显示
  「原文 + 语言条 + 引擎行右侧转圈」的完整布局,译文区留骨架占位;译文到达
  (`screenshot://result`)后转圈变 `✓ 耗时`、译文就地填入。故 translate 阶段**不再**走
  整块骨架的 `progress`。

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

popup 或选区窗口聚焦时按 ESC → 回 IDLE,取消会话;抓屏期间浮层隐藏,可用新请求取代旧请求。

## 7. 取消机制

- Rust `ScreenshotSession` 用 `watch` 维护递增会话号;F1、F2、手动重译共享取消域。
- 新触发 / 关闭调用 `begin` 或 `cancel`,通过 `select!` 释放旧 HTTP future;已开始的原生阻塞截图/OCR可继续结束,结果不投递。
- 所有 `screenshot://*` 事件带 `sessionId`。手动重译先调用 `begin_popup_session`,再调用 `translate_popup(req, sessionId, requestId)`;前端屏蔽旧会话事件,保留预约期间已到达的新会话结果。
- 剪贴板事务在阻塞任务内完成恢复步骤,不随翻译取消中断。
- 语言互换仅在成功且源语言已知时可用,用当前译文作为新原文。

## 8. 验收标准

- [ ] Alt+S → 进入截图模式 ≤ 300ms。
- [ ] 框选完成 → 抓屏结束后弹框出现(识别进度),通常 ≤ 500ms。
- [ ] 框选完成 → 完整结果显示 ≤ 3s(本地 OCR + 本地 LLM)。
- [ ] popup / 选区的 ESC 能取消;连续触发会取消旧流程,不残留窗口。
- [ ] 弹框位置在选区正下方,不超出屏幕。
- [ ] 翻译失败时展示原文,不空白。

## 9. 测试点

- 框选单行 / 多行 / 混排文本。
- 框选无文字区域(纯图像)。
- 翻译引擎超时(设极短 timeout 模拟)。
- 连续两次 Alt+S,第一次未完成时触发第二次。
- 弹框出现后,再次框选 → 弹框内容刷新。
