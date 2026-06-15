# Spec 09 · 窗口与 UI

> 覆盖 PRD §2.3(主窗口/状态栏)、§4(交互与 UI)。定义四类窗口的规格、布局与通用 UI 约定。

---

## 1. 窗口清单

| label | 类型 | 生命周期 | 入口 |
|---|---|---|---|
| `main` | 静态(tauri.conf) | 常驻可隐藏 | 托盘 / 快捷键 `Alt+Cmd+T` |
| `popup` | 静态(tauri.conf) | 常驻可隐藏 | F1/F2 结果推送时 show |
| `selection` | 运行时创建 | 一次性 | F2/F3 触发时创建 |
| `overlay` | 运行时创建 | 一次性 | F3 结果推送时创建 |

## 2. main 窗口

### 2.1 窗口属性

```json
{
  "label": "main",
  "width": 800, "height": 560,
  "minWidth": 680, "minHeight": 460,
  "resizable": true,
  "decorations": true,
  "center": true,
  "visible": false
}
```

macOS:`titleBarStyle: "overlay"`(隐藏标题栏,前端用 `data-tauri-drag-region` 实现拖拽)。

### 2.2 Tab 结构

| Tab | 路由 | 内容 |
|---|---|---|
| 通用 | `/settings/general` | 语言、主题、自启、更新、代理 |
| 快捷键 | `/settings/shortcuts` | 各功能快捷键录制/清空 |
| 翻译引擎 | `/settings/engines` | 引擎列表 + 增删改 + 测试 |
| OCR | `/settings/ocr` | 引擎选择、语言包提示 |
| 关于 | `/settings/about` | 版本、开源协议、GitHub 链接 |

### 2.3 翻译引擎 Tab

- 左侧:引擎列表(拖拽排序、启用/禁用 toggle、删除)。
- 右侧:选中引擎的详情表单:
  - 名称、类型(下拉)、API URL、API Key(password 输入)、模型名(LLM 类)。
  - System Prompt(LLM 类,多行文本框,含变量说明)。
  - 超时时间(数字 + "ms" 单位)。
  - 使用代理(toggle)。
  - "测试连接"按钮 → 调 `test_engine` command,显示延迟 / 错误。
- 底部"添加引擎"按钮 → 新建空表单。

### 2.4 快捷键 Tab

每行:功能名 + 当前快捷键显示 + "录制"按钮 + "清空"按钮。

录制流程:
1. 点击"录制" → 按钮变为"按下快捷键…"(监听 keydown)。
2. 捕获组合键后显示预览(如 `⌥ S`)。
3. 检测冲突:与已有快捷键或系统常用键冲突 → 显示警告,仍允许保存。
4. 点击"保存"→ 调 `set_config` 更新 `shortcuts.*`。

## 3. popup 窗口

### 3.1 窗口属性

```json
{
  "label": "popup",
  "width": 360, "minHeight": 120, "maxHeight": 480,
  "resizable": false,
  "decorations": false,
  "transparent": true,
  "alwaysOnTop": true,
  "skipTaskbar": true,
  "visible": false,
  "focusable": true
}
```

高度动态调整:内容渲染后通过 `window.setSize` 更新。

### 3.2 布局

```
┌─────────────────────────────────┐
│ [英语] → [中文]      [×]        │  ← 语言行 + 关闭按钮
├─────────────────────────────────┤
│ 原文(可折叠)                    │
│  selected text here...          │
├─────────────────────────────────┤
│ 译文                            │
│  翻译后的文字...                 │
├─────────────────────────────────┤
│ [复制译文]    via DeepSeek-R2   │  ← 底部工具栏
└─────────────────────────────────┘
```

- 背景:macOS `vibrancy: "hud"`,Windows `acrylic`(Tauri 支持的程度为准)。
- 圆角:8px。
- 字号:14px(译文),12px(原文、语言标识)。
- padding:12px。

### 3.3 状态

| 状态 | 展示 |
|---|---|
| loading | 骨架屏:语言行灰色占位 + 两行占位块 |
| success | 完整布局 |
| error | 错误图标 + 错误信息 + 重试按钮 |

### 3.4 交互

- 点击窗口外部 → 隐藏(通过 `on_window_event BlurEvent` 或前端 blur 监听)。
- ESC → 隐藏。
- 复制按钮 → 将译文写入剪贴板,按钮短暂变"已复制"。
- 原文区点击折叠/展开 → 调整窗口高度。

### 3.5 定位

由各功能模块(F1/F2)在调用 `show_popup` 前计算目标坐标,通过 `window.setPosition` 设置。定位逻辑见 [Spec 05 §7](./05-selection-translation.md) 和 [Spec 06 §2.6](./06-screenshot-translation.md)。

## 4. selection 窗口

### 4.1 窗口属性

```json
{
  "label": "selection",
  "transparent": true,
  "decorations": false,
  "alwaysOnTop": true,
  "skipTaskbar": true,
  "resizable": false,
  "focusable": true
}
```

位置/尺寸:运行时设置为虚拟屏全部显示器的并集矩形。

### 4.2 UI 实现要点

- 全屏 canvas 或 SVG 实现选区绘制(推荐 canvas:性能更好)。
- 遮罩颜色:`rgba(0,0,0,0.35)`。
- 选区内镂空:用 `globalCompositeOperation = "destination-out"` 清除矩形区域。
- 边框:在 canvas 上绘制 1px 白色矩形。
- 尺寸提示:`{w}×{h}`:用绝对定位 div,跟随选区右上角。
- 鼠标事件:
  - `mousedown` → 开始选区,记录起点。
  - `mousemove` → 重绘选区。
  - `mouseup` → 结束选区,emit `capture://region-selected`。
  - `keydown ESC` → emit `capture://region-cancelled`,关闭窗口。

### 4.3 坐标转换

前端获取的鼠标坐标是 CSS 坐标(相对窗口,已考虑 DPR)。
窗口本身以逻辑坐标放置,因此前端的 `clientX/Y` + 窗口原点 = 虚拟屏逻辑坐标。
通过 `window.__TAURI__.window.getCurrent().innerPosition()` 获取窗口原点偏移。

## 5. overlay 窗口

规格详见 [Spec 07 §5](./07-overlay-translation.md)。此处补充 UI 相关:

- 不显示任何边框、背景或 UI chrome。
- 每个译文块独立渲染(无全局背景色)。
- 字体:`system-ui, sans-serif`(与目标应用接近)。
- 不可交互(focusable: false,鼠标穿透)。

## 6. 通用 UI 约定

### 6.1 主题

```css
/* 浅色 */
:root {
  --bg: #ffffff;
  --bg-secondary: #f5f5f5;
  --text: #111111;
  --text-muted: #888888;
  --accent: #0066ff;
  --border: #e0e0e0;
}

/* 深色 */
:root[data-theme="dark"] {
  --bg: #1a1a1a;
  --bg-secondary: #2a2a2a;
  --text: #f0f0f0;
  --text-muted: #888888;
  --accent: #4da6ff;
  --border: #3a3a3a;
}
```

主题应用:`config.app.theme`:
- `"system"`:跟随 `prefers-color-scheme`。
- `"light"` / `"dark"`:强制。

Rust 侧检测系统主题变化后 emit `config://theme-changed`。

### 6.2 字体

- UI 文字:`system-ui`(系统默认字体)。
- 代码/快捷键展示:`ui-monospace, monospace`。

### 6.3 组件库

使用 shadcn/ui:按需引入,不全量导入。覆写变量使其对接 `--bg`/`--text` 等自定义 token。

### 6.4 国际化

- `src/lib/i18n.ts`:使用 `i18next`。
- 语言包文件:`src/locales/zh-CN.json`,`src/locales/en-US.json`。
- `config.app.locale = "system"` 时,取 `navigator.language` 前缀匹配。
- 所有 UI 文字通过 `t("key")` 引用,禁止硬编码中文/英文。

## 7. 验收标准

- [ ] main 窗口所有 Tab 可正常导航,内容加载无白屏。
- [ ] popup 在 F1/F2 触发后 ≤ 500ms 出现。
- [ ] popup 高度随内容动态调整,不出现纵向滚动条或截断。
- [ ] 深色 / 浅色 / 跟随系统三种主题均正常渲染,无硬编码颜色穿帮。
- [ ] 多语言切换(中/英)后所有 UI 文字即时更新,无需重启。
- [ ] selection 窗口在多显示器下覆盖所有屏幕区域。
