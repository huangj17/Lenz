# Spec 02 · 截图与选区

> 覆盖 PRD §2.2 F2/F3 前半段与 §3.4(截图与选区)。F2/F3 共享同一套采集层,差别仅在后续处理分叉。

---

## 1. 目标

1. 按需获取**多显示器任一区域**的位图,输出给 OCR。
2. 提供一个低延迟、零残留的**全屏选区蒙层**,用户以鼠标拖拽框选区域。
3. 同时兼容标准屏幕与 HiDPI(Retina / Windows scale ≥ 125%)。

## 2. 采集方式

### 2.1 完整屏幕位图

- Rust 侧使用 `xcap` crate 获取所有 `Monitor`,按需抓取单屏或跨屏。
- 数据结构:`image::RgbaImage`,后续转 PNG 供 OCR(如需)或直接传 raw buffer。
- 多屏处理:以 `Monitor.id()` 为 key 缓存,并保存每屏的缩放比例 `scale_factor` 与物理 origin。

### 2.2 区域截取

- 输入:`Rect { x, y, w, h }`(屏幕**逻辑坐标**,跨屏以最左上显示器原点为 (0,0))。
- 步骤:
  1. 根据 rect 找出覆盖到的所有 Monitor。
  2. 逐屏抓取,按 `scale_factor` 把逻辑坐标换算为物理像素。
  3. 裁剪 + 如需跨屏则在物理像素坐标系里拼接为单张图。
- 输出:`CaptureResult { image: Vec<u8> /* PNG */, pixel_rect: Rect, scale_factor: f32 }`。

> 约定:上层模块不关心物理像素,仅接收 `pixel_rect` 用于 OCR 坐标回映射(见 Spec 03 §4)。

## 3. 选区蒙层

### 3.1 窗口规格(见 Spec 09)

- 运行时为**每块显示器**各创建一个透明全屏窗口 `selection-{i}`,用 `set_position` + `set_size`(物理像素)精确铺满该屏。
- ⚠️ 不用单个跨虚拟屏的窗口:macOS「显示器各自拥有独立 Space」(默认开启)会把跨屏窗口夹到单块屏,导致只有一块屏能框选。按屏拆窗规避此限制。
- 属性:`transparent: true`, `decorations: false`, `alwaysOnTop: true`, `skipTaskbar: true`, `fullscreen: false`。
- 鼠标事件:不穿透(需要响应拖拽)。

### 3.2 前端交互

- 页面背景:`rgba(0,0,0,0.35)`(半透明遮罩)。
- 十字光标 `cursor: crosshair`。
- 按下鼠标开始绘制:框内**镂空透明**(使用 4 个绝对定位 div 遮住四周,或 canvas 绘制 `destination-out`)。
- 选区边框:1px 白色实线,角 4 个 8×8 white handle。
- 顶部或旁侧浮显尺寸提示:`widthpx × heightpx`。
- 支持键盘:
  - `ESC` → 取消,关闭选区窗口。
  - `Enter` → 确认当前选区(如已存在有效矩形)。

### 3.3 坐标反馈

完成框选后由前端 emit:

```ts
emit("capture://region-selected", {
  x, y, w, h,            // 逻辑坐标,相对虚拟屏左上
  displayId,             // 主要落在的显示器 id(选区跨屏时取起点所在)
  trigger: "screenshot" | "overlay",
});
```

Rust 侧监听并继续走 F2/F3 分支。

## 4. Tauri Commands

```rust
#[tauri::command]
async fn start_selection(trigger: TriggerKind, app: AppHandle) -> Result<(), LenzError>;

#[tauri::command]
async fn cancel_selection(app: AppHandle) -> Result<(), LenzError>;

#[tauri::command]
async fn capture_screen_region(
    rect: Rect,             // 逻辑坐标
) -> Result<CaptureResult, LenzError>;

pub struct CaptureResult {
    pub png: Vec<u8>,
    pub pixel_rect: Rect,   // 物理像素
    pub scale_factor: f32,
}
```

`start_selection` 的职责:

1. 若已有 selection 窗口 → 全部销毁,再开新的(避免残留)。
2. 枚举显示器,每块各创建一个铺满该屏的全屏窗口(规避 macOS 独立 Space 对跨屏窗口的夹取)。
3. 向各窗口注入 `trigger`(URL query 或 initial state)。
4. 收到前端 `capture://region-selected` 后销毁全部 selection 窗口,转交给对应功能模块。

## 5. 坐标系与 HiDPI

- 所有对外接口用**逻辑坐标**(points / CSS 像素)。
- 跨屏:以 macOS 的 "NSScreen.frame 并集" 或 Windows 的 `GetSystemMetrics(SM_XVIRTUALSCREEN...)` 为基准,左上角 (0,0)。
- 缩放换算在 `core/capture.rs` 内部完成:`pixel = round(logical * scale_factor)`。
- 前端 overlay(Spec 07)渲染时同样使用逻辑坐标即可,无需关心 DPR。

## 6. 错误与超时

| 场景 | 行为 |
|---|---|
| 无权限(macOS 未授予 "屏幕录制") | 返回 `LenzError::Permission`,前端弹授权引导 |
| xcap 抓取失败(驱动异常) | 返回 `LenzError::Internal`,toast 报错 |
| 截图超过 10s | 返回阶段明确的 `LenzError::Timeout`;截图预算独立于 OCR 和窗口握手 |
| 选区窗口创建超过 500ms | 视为异常,取消流程 |
| 选区矩形宽或高 < 4px | 视为点击取消,emit `capture://region-cancelled` |

F2/F3、`capture_screen_region` 和 `run_ocr` 共用 `capture_region_with_timeout`。日志记录截图开始、完成/失败、尺寸、缩放和耗时,不记录图像内容。

## 7. 验收标准

- 按快捷键到选区蒙层显示 ≤ 300ms(对齐 PRD F2 指标)。
- 多显示器 + 不同 DPR 组合下,选区矩形位置与实际截图像素**误差 ≤ 1 点**。
- ESC 能在任何子状态下退出,不残留窗口。
- 选区完成后 selection 窗口立即销毁,不出现闪烁或截到自身遮罩的 bug。

## 8. 测试点

- 单屏 1× / 单屏 2× / 双屏 1×+2× 混排 / 竖屏副屏。
- 跨屏选择(起点 A 屏,终点 B 屏)。
- macOS 首次触发时的权限弹窗路径。
- 拖拽过程中按 ESC(应取消)。
- 拖拽过程中意外触发其他快捷键(应被选区窗口吞掉)。

## 9. 开放问题

- [ ] Windows 下 `alwaysOnTop + transparent + fullscreen` 组合在部分 GPU 驱动会黑屏,需要 W2 做兼容测试。
- [ ] Wayland(Linux)暂不支持;本 MVP 不覆盖 Linux,但代码层保持平台抽象,避免硬编码。
