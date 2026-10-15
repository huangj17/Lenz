# Spec 05 · 划词翻译 (F1)

> 覆盖 PRD §2.2 F1 与 §3.4(划词监听)。定义选中文本监听、去抖、弹框触发与结果展示。

> ⚠️ **设计变更(2026-06-18,以实现为准)**:自动模式由「选中即译」改为 **划词指示图标**(Bob/Easydict 式)。本文 §1(选中松手即译)、§2.1(`changeCount` 直接取文本)、§4(去抖直接 emit triggered)、§5(`selection://triggered`)、§10(无选中→toast)为旧设计,**实际实现见 `docs/TODO.md` M5 设计调整 + Increment 1/2 说明**:
> - macOS CGEventTap / Windows WH_MOUSE_LL 监听左键 down/up,判「拖拽位移≥4pt 或双击 clickState≥2」为选中手势 → emit `selection://detected {x,y}`(**不读剪贴板**);
> - 前端在光标处浮出 Lens 图标,用户 **hover/点击** → `trigger_selection_translate` → AX/UIA 预读选区（失败才合成 `Cmd/Ctrl+C`）→ 翻译 → 复用 popup 在光标处弹框;
> - 自动监听总开关 = `selection.enabled`(托盘「划词监听」,默认 false / opt-in);Alt+D 快捷键划词始终可用,不受开关影响。本文 §2/§3 已同步现行平台实现。

---

## 1. 触发方式总览

| 模式 | 触发条件 | 配置项 |
|---|---|---|
| 快捷键触发 | 先选中文本,再按 `Alt+D` | `shortcuts.selection` |
| 图标触发 | 选中文字后浮出图标,hover/点击后翻译 | `selection.enabled = true` |

两种模式可同时启用。快捷键触发始终可用,自动触发受总开关 `selection.enabled` 控制。

> **总开关入口**:`selection.enabled` 同时暴露于「托盘 · 划词监听」勾选(spec 10 §1.2)与「设置 · 通用 · 划词」的「划词显示翻译图标」开关(spec 09 §2.2);两处共用该字段,经 `set_config` → `config://changed` 联动启停监听器并同步各自 UI 状态。

## 2. 平台监听实现

### 2.1 macOS

`core/selection.rs` 的 CFRunLoop + CGEventTap 独立线程监听左键 down/up,判拖拽或多击选中,只监听不吞事件。需要 Accessibility 授权;缺失发 `selection://permission-required`,设置页显示系统设置入口和重新检查。AXSelectedText 直读无需 Apple Events entitlement。

### 2.2 Windows

`core/selection_windows.rs` 用独立消息线程的 `WH_MOUSE_LL` 全局鼠标 hook,拖拽/双击发相同手势事件,停止时 WM_QUIT + Unhook。回调不读文本、不读剪贴板、不吞事件。

`core/selection_text.rs` 在工作线程初始化 COM,通过 UI Automation 焦点元素、TextPattern/GetSelection/GetText 读取选区,跳过密码字段。失败复用 Ctrl+C 和剪贴板快照恢复。图标不夺焦,记录/恢复源前台窗口;按来源屏 DPI 使用物理位置显示和命中测试。普通桌面应用无需额外授权;高权限进程和不暴露 TextPattern 的应用可能只能走快捷键/剪贴板兜底。

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

Alt+D(及点击划词图标)按下后,`handle_selection_shortcut` 按优先级取选中文本,取到即翻译:

```
全局快捷键 Alt+D 按下 / 点击划词图标
  └─ Rust handle_selection_shortcut
       ├─ ① AX 直读:read_selected_text()(AXSelectedText,不碰剪贴板)
       │      └─ 命中 → 翻译 + 光标处弹框
       ├─ ② 兜底(AX 取不到,如部分 Chrome/Electron):
       │      等修饰键松开 → 原生格式快照 + 合成 Cmd/Ctrl+C → 版本号判定变化 → 条件恢复
       │      └─ 命中 → 翻译 + 光标处弹框
       └─ ③ 都无(无选区 / 剪贴板空 / 无权限):
              ├─ 有「上次结果」→ 光标处弹框回显上次译文(不重新翻译)
              └─ 无「上次结果」→ 忽略(仅记日志)
```

- 剪贴板兜底串行执行,重叠探测直接放弃;不写占位哨兵。复制前保存所有可还原的原生格式(含富文本),超过 16 MiB 或存在无法备份的格式时跳过兜底。
- macOS 用 `NSPasteboard.changeCount` 和输入事件计数,Windows 用剪贴板序号、锁和合成输入时间戳检查干扰;检测到新的复制或用户输入时放弃恢复,保留较新的剪贴板内容。系统接口无法为所有跨进程写入提供事务保证,仍需实机测试常用源应用。
- 图标外的普通点击也会取消尚未显示的去抖任务;停用监听时立即隐藏图标并清理预读缓存。
- ①②③ 详见 `core/screenshot.rs`(`handle_selection_shortcut`)、`core/clipboard_selection.rs` 与 `core/selection_text.rs`。
- 划词图标点击走同一编排,但**优先复用图标浮出瞬间 AX 预读的缓存**(源应用仍在前台时读),规避点击图标夺焦后读到错内容;缓存缺失才回退上述 ①②③。见 [§6](#6-popup-窗口行为) 与 `commands/overlay.rs::trigger_selection_translate`。

### 3.1 「上次结果」回显与持久化

- 每次原生主服务成功翻译(F1/F2 通用)都缓存为「上次结果」(`LastResult`):**内存一份 + 磁盘一份**,磁盘文件 `last-result.json` 与 `config.json` 同目录(`app_config_dir()`)。弹窗额外服务结果只保留在当前会话,回显时按当前启用列表重新查询。
- 启动时从磁盘 `hydrate` 回内存,故**应用重启后**按 Alt+D(无选区/剪贴板空)仍能回显上次译文,而非无反应。
- 写盘为 best-effort:失败仅记日志,不阻断主流程;文件缺失(首次运行)或解析失败均静默留空。
- ⚠️ 隐私:上次译文以明文落盘(与 config 明文存凭据的 MVP 取舍一致,见 [spec 01 §6](./01-config-storage.md))。若后续需要更强隐私,可退化为「仅会话内回显」或加开关。

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
2. Rust 按折叠策略选择原生主服务并推送 translating/result/error;弹窗按当前已启用服务顺序创建结果行,在同一 `sessionId` 下按每行记忆的折叠/跳过策略并行请求其余服务。各项独立返回,失败不影响其他结果,原生主服务不重复调用;折叠跳过的服务展开后再请求(见 [spec 04 §2.1](./04-translator.md#21-弹窗多服务对比))。
3. 点击 popup 外部 → 隐藏;不销毁窗口(复用)。
4. 再次触发相同文本/目标/主引擎 → 回显主服务缓存,其余当前启用服务重新查询;缓存主服务已禁用时隐藏该项并请求当前启用服务。

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
- [ ] 快捷键模式:无新选区/剪贴板空时按 Alt+D,回显上次结果(有则弹框,无历史则无反应),不弹空框。
- [ ] 应用重启后按 Alt+D(无选区/剪贴板空),仍能回显上次翻译结果(`last-result.json` 持久化)。
- [ ] 弹框不超出任何一块屏幕边界。
- [ ] 再次选中新文本,弹框内容更新而非新增第二个弹框。
- [ ] `toggle_selection_listener(false)` 后,选中文本不触发任何弹框。

## 11. 测试点

- 快速连续选中三段文本(应仅展示最后一段的翻译)。
- 在输入框内选中文本(应正常触发)。
- 选中纯数字 / 链接(应触发但翻译结果合理)。
- 超过 5000 字符(应静默忽略)。
- macOS 未授权辅助功能时(应有引导弹窗)。
- 翻译一段文本后**重启应用**,不选任何文本按 Alt+D(应回显重启前的上次结果)。
- 删除 / 损坏 `last-result.json` 后启动(应静默留空,不报错)。
