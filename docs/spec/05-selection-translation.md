# Spec 05 · 划词翻译 (F1)

> 覆盖 PRD §2.2 F1 与 §3.4(划词监听)。定义选中文本监听、去抖、弹框触发与结果展示。

> ⚠️ **设计变更(2026-06-18,以实现为准)**:自动模式由「选中即译」改为 **划词指示图标**(Bob/Easydict 式)。本文 §1(选中松手即译)、§2.1(`changeCount` 直接取文本)、§4(去抖直接 emit triggered)、§5(`selection://triggered`)、§10(无选中→toast)为旧设计,**实际实现见 `docs/TODO.md` M5 设计调整 + Increment 1/2 说明**:
> - CGEventTap 监听左键 down/up,判「拖拽位移≥4pt 或双击 clickState≥2」为选中手势 → emit `selection://detected {x,y}`(**不读剪贴板**);
> - 前端在光标处浮出 Lens 图标,用户 **hover/点击** → `trigger_selection_translate` → 合成 `Cmd+C` → 轮询剪贴板 → 翻译 → 复用 popup 在光标处弹框;
> - 自动监听总开关 = `selection.enabled`(托盘「划词监听」,默认 false / opt-in);Alt+D 快捷键划词始终可用,不受开关影响。本文待整体回写。

---

## 1. 触发方式总览

| 模式 | 触发条件 | 配置项 |
|---|---|---|
| 快捷键触发 | 先选中文本,再按 `Alt+D` | `shortcuts.selection` |
| 自动触发 | 选中文本后 `popupDelayMs` 内松开鼠标,自动弹框 | `selection.autoDetect = true` |

两种模式可同时启用。快捷键触发始终可用,自动触发受总开关 `selection.enabled` 控制。

## 2. 平台监听实现

### 2.1 macOS

文件:`core/selection.rs`(macOS 分支)

- **CGEventTap**:以 `kCGSessionEventTap` 在全局监听 `kCGEventLeftMouseUp` 事件。
- 收到 mouseUp → 延迟 `popupDelayMs`(默认 200ms)→ 读取 `NSPasteboard.generalPasteboard` 的 `changeCount`。
  - 若 changeCount 有变化,认为用户做了新的复制/选中操作;取 `stringForType: NSPasteboardTypeString`。
  - 若无变化,忽略。
- 权限:需要"辅助功能"权限(`AXIsProcessTrusted()`)。
  - 首次触发时若无权限 → emit `selection://permission-required`,前端弹引导。
  - 不阻塞主进程;监听线程等权限授予后自动重试。

> 注:macOS 14+ 对 CGEventTap 限制更严,需在 entitlements 中声明 `com.apple.security.automation.apple-events`。

### 2.2 Windows

文件:`core/selection.rs`(Windows 分支)

- **SetWinEventHook**:`EVENT_OBJECT_TEXTSELECTIONCHANGED` + `EVENT_SYSTEM_FOREGROUND`。
- 收到事件 → 延迟 `popupDelayMs` → `OpenClipboard` + `GetClipboardData(CF_UNICODETEXT)` 读取当前选中文本。
- 对比上次读取的文本;若不同则触发。
- 此方式无需额外权限。

### 2.3 监听线程管理

```rust
pub struct SelectionListener {
    handle: Option<JoinHandle<()>>,
    cancel_tx: Option<oneshot::Sender<()>>,
}

impl SelectionListener {
    pub fn start(&mut self, config: SelectionConfig, app: AppHandle);
    pub fn stop(&mut self);
    pub fn restart(&mut self, config: SelectionConfig, app: AppHandle);
}
```

- 启动时在独立线程运行(非 tokio async,避免 event loop 冲突)。
- `config.enabled = false` 或用户从托盘点"暂停" → 调 `stop()`。

## 3. 快捷键触发流程

```
全局快捷键 Alt+D 按下
  └─ Rust shortcut handler
       ├─ 读剪贴板当前文本(同平台方式)
       ├─ 若为空 → toast "未检测到选中文本"
       └─ emit selection://triggered { text, source: "shortcut", cursorPos }
```

前端 popup 窗口监听 `selection://triggered`。

## 4. 自动触发去抖

```
mouseUp / textSelectionChanged
  └─ 清除旧 timer
  └─ 设置 timer(popupDelayMs = 200ms 默认)
      └─ timer 到期
           ├─ 读文本
           ├─ 若长度 < 1 或 > 5000 字符 → 忽略
           ├─ 若与上次相同 → 忽略
           └─ emit selection://triggered { text, source: "auto", cursorPos }
```

`cursorPos: { x, y }`(鼠标屏幕逻辑坐标),用于弹框定位。

## 5. Rust → 前端事件

```ts
// selection://triggered 载荷
interface SelectionTriggered {
  text: string;
  source: "shortcut" | "auto";
  cursorPos: { x: number; y: number };
}

// selection://permission-required 载荷
interface PermissionRequired {
  platform: "macos" | "windows";
  detail: string;
}
```

## 6. popup 窗口行为

详见 [Spec 09](./09-windows-ui.md)。此处描述与 F1 强相关的逻辑:

1. 收到 `selection://triggered`:
   - 若 popup 已可见且在翻译中 → 覆盖(取消旧请求)。
   - 计算弹框位置(见 §7)。
   - 立即 show popup,显示"翻译中…"骨架屏。
2. 调 `translate_text`,得到结果后更新内容。
3. 点击 popup 外部 → 隐藏;不销毁窗口(复用)。
4. 再次触发同一文本 → 不重复翻译,直接 show 已有结果(可选,M5 评估)。

## 7. 弹框定位

```
raw_x = cursorPos.x + 16     // 向右偏移
raw_y = cursorPos.y + 16     // 向下偏移

// 防溢出检查(以 popup 宽 360 × 高 200 估算)
popup_x = min(raw_x, screen_w - 360 - 8)
popup_y = min(raw_y, screen_h - 200 - 8)
```

屏幕宽高由 Rust 侧 `Monitor` 信息提供。多显示器:用 `cursorPos` 所在 monitor 的边界检查。

## 8. Tauri Command

```rust
#[tauri::command]
fn toggle_selection_listener(
    listener: State<'_, Mutex<SelectionListener>>,
    enabled: bool,
    config: SelectionConfig,
    app: AppHandle,
) -> Result<(), LenzError>;
```

前端设置页"暂停划词监听"按钮调此 command。

## 9. 状态机

```
IDLE
  │ 用户选中文本
  ▼
WAITING(timer)
  │ timer 到期 & 文本有效
  ▼
TRANSLATING
  │ 翻译完成
  ▼
SHOWING
  │ 点击外部 / ESC
  ▼
IDLE

WAITING ──ESC / 文本清空──► IDLE
TRANSLATING ──新触发──► TRANSLATING(取消旧请求)
```

## 10. 验收标准

- [ ] 选中文本 → 弹框出现(UI 响应,不含翻译)≤ 500ms。
- [ ] 自动模式:鼠标不动选同一段文本,不重复触发翻译。
- [ ] 快捷键模式:无选中文本时按快捷键,toast 提示而非空弹框。
- [ ] 弹框不超出任何一块屏幕边界。
- [ ] 再次选中新文本,弹框内容更新而非新增第二个弹框。
- [ ] `toggle_selection_listener(false)` 后,选中文本不触发任何弹框。

## 11. 测试点

- 快速连续选中三段文本(应仅展示最后一段的翻译)。
- 在输入框内选中文本(应正常触发)。
- 选中纯数字 / 链接(应触发但翻译结果合理)。
- 超过 5000 字符(应静默忽略)。
- macOS 未授权辅助功能时(应有引导弹窗)。
