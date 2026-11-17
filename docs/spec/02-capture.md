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

### 2.3 截图定格

F2/F3 触发时**先定格、后框选**:选区蒙层会抢焦点并遮住鼠标,tooltip、菜单、下拉框、popover 等易失内容在蒙层出现后就会消失,框选完再截屏已经截不到。

1. 快捷键触发:隐藏 Lenz 自己的浮层(popup / 划词图标 / 覆盖层 / 旧选区),仅当它们原本可见时等待 150ms 离屏;随后立即对**全部显示器**整屏截图。不创建窗口、不抢焦点。托盘菜单入口先等 300ms,让菜单本身收起。
2. 定格结果 `FrozenFrame { bounds /* 逻辑 */, image /* RGBA 物理像素 */ }` 存于 managed state `FreezeState`,带递增 token;新触发作废旧 token,迟到的截图结果直接丢弃。
3. 选区窗口按逻辑边界匹配对应的定格图(重叠超过显示器面积一半),以其为背景;匹配不到(定格失败、定格后新接入的屏)的窗口退回透明实时蒙层。
4. 框选完成时取走定格图并裁剪,裁剪换算复用 §2.2 的计划函数,物理/逻辑比例按位图实际宽度计算;选区与所有定格屏都不相交时回退实时截屏(先等 150ms 让蒙层离屏)。
5. 取消、选区完成、新触发都会释放定格图。定格图只在内存中,不落盘、不进日志。

前端经自定义协议 `lenz-frame://localhost/{token}-{index}`(Windows 为 `http://lenz-frame.localhost/...`)读取定格图。响应为 32 位 BMP:BITMAPV4HEADER + BI_BITFIELDS 声明 RGBA 字节序、负高度表示自上而下,像素区原样拷贝,免去整屏 PNG 编码;不声明 alpha 通道,整图按不透明解码。会话结束或 token 不匹配返回 404。CSP `img-src` 放行该协议。

## 3. 选区蒙层

### 3.1 窗口规格(见 Spec 09)

- 运行时为**每块显示器**各创建一个透明全屏窗口 `selection-{i}`,用 `set_size` + `set_position` 精确铺满该屏。macOS 须先定尺再定位:AppKit 改尺寸时固定左下角,先定位会让顶边上移。
- ⚠️ 不用单个跨虚拟屏的窗口:macOS「显示器各自拥有独立 Space」(默认开启)会把跨屏窗口夹到单块屏,导致只有一块屏能框选。按屏拆窗规避此限制。
- 属性:`transparent: true`, `decorations: false`, `alwaysOnTop: true`, `skipTaskbar: true`, `fullscreen: false`。macOS 额外把窗口层级提到屏保级(`NSScreenSaverWindowLevel`),盖住菜单栏、Dock 与仍展开的菜单,定格图之上不再叠实时画面,也不会被系统夹到菜单栏下方而错位。
- 有定格图的窗口先隐藏构建,前端解码完背景后调用 `selection_ready` 再显示;1.5s 内未就绪仍显示,保证可取消。无定格图的窗口立即显示。首屏窗口 `selection-0` 显示时取焦,承载 ESC。
- 鼠标事件:不穿透(需要响应拖拽)。

### 3.2 前端交互

- 页面背景:定格图(`?frame=` 指向的 `lenz-frame` 图片,铺满窗口)+ `rgba(0,0,0,0.35)` 半透明遮罩;无定格图时只有遮罩。
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

// 选区窗口画好定格背景后调用,显示调用方窗口。
#[tauri::command]
fn selection_ready(window: WebviewWindow);

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

1. 隐藏旧 selection 窗口与 Lenz 浮层,按 §2.3 定格全部显示器。
2. 若已有 selection 窗口 → 全部销毁,再开新的(避免残留)。
3. 枚举显示器,每块各创建一个铺满该屏的全屏窗口(规避 macOS 独立 Space 对跨屏窗口的夹取)。
4. 向各窗口注入 `trigger` 与匹配到的定格图标识 `frame`(URL query)。
5. 收到前端 `capture://region-selected` 后销毁全部 selection 窗口,取走定格图转交给对应功能模块;`capture://region-cancelled` / `cancel_selection` 销毁窗口并丢弃定格图。

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
| 定格失败或超过 10s | 记日志,选区退回透明实时蒙层,框选后实时截屏(沿用上述权限/失败提示) |
| 选区窗口创建超过 500ms | 视为异常,取消流程 |
| 选区矩形宽或高 < 4px | 视为点击取消,emit `capture://region-cancelled` |

F2/F3 优先从定格图裁剪;无定格图时与 `capture_screen_region`、`run_ocr` 共用 `capture_region_with_timeout`。日志记录定格/截图开始、完成/失败、尺寸、缩放和耗时,不记录图像内容。

## 7. 验收标准

- 按快捷键到选区蒙层显示 ≤ 300ms(对齐 PRD F2 指标;含整屏定格,以 release 构建计)。
- 按快捷键时可见的 tooltip、菜单、下拉框、popover 出现在选区背景中,框选后的截图与 OCR 输入与背景一致。
- 选区背景与屏幕逐点对齐,覆盖菜单栏与 Dock;多显示器 + 不同 DPR 组合下,选区矩形位置与实际截图像素**误差 ≤ 1 点**。
- ESC 能在任何子状态下退出,不残留窗口;取消或完成后定格图被释放。
- 选区完成后 selection 窗口立即销毁,不出现闪烁或截到自身遮罩的 bug。

## 8. 测试点

- 单屏 1× / 单屏 2× / 双屏 1×+2× 混排 / 竖屏副屏。
- 跨屏选择(起点 A 屏,终点 B 屏)。
- macOS 首次触发时的权限弹窗路径;未授权时定格失败回退实时蒙层。
- 易失内容:浏览器 `title` tooltip、右键菜单、菜单栏菜单、下拉框、popover 打开时触发,选区背景与识别结果应包含它们。
- 托盘入口触发:定格图不含托盘菜单。
- Lenz popup 可见时触发:定格图不含 popup。
- 拖拽过程中按 ESC(应取消)。
- 拖拽过程中意外触发其他快捷键(应被选区窗口吞掉)。

## 9. 开放问题

- [ ] Windows 下 `alwaysOnTop + transparent + fullscreen` 组合在部分 GPU 驱动会黑屏,需要 W2 做兼容测试。
- [ ] Wayland(Linux)暂不支持;本 MVP 不覆盖 Linux,但代码层保持平台抽象,避免硬编码。
