# Spec 03 · OCR 引擎

> 覆盖 PRD §2.2 F5 与 §3.4(OCR 桥接)。定义 OCR 抽象、macOS Vision 与 Windows.Media.Ocr 实现、坐标回映射。

---

## 1. 目标

1. 把一张位图(或屏幕区域截图)识别为**文本块 + bbox + 置信度**列表。
2. 抽象为 trait,方便未来切换 Tesseract / PaddleOCR / 云端 OCR。
3. 在跨平台差异下输出**统一结构**。

## 2. Trait 定义

```rust
// src-tauri/src/core/ocr/mod.rs

#[async_trait::async_trait]
pub trait OcrEngine: Send + Sync {
    fn name(&self) -> &'static str;
    fn supported_languages(&self) -> Vec<String>;  // BCP-47
    async fn recognize(&self, input: OcrInput, opts: OcrOptions) -> Result<OcrResult, LenzError>;
}

pub struct OcrInput {
    pub png: Vec<u8>,
    pub pixel_rect: Rect,     // 截图在屏幕上的物理像素位置(用于回映射)
    pub scale_factor: f32,    // 来自 CaptureResult
}

pub struct OcrOptions {
    pub preferred_languages: Vec<String>,  // 按优先级排序
    pub fast_mode: bool,                   // 仅 Vision 支持
    pub auto_detect_language: bool,       // macOS 13+ 自动检测;Windows 系统语言回退
    pub min_confidence: f32,               // 过滤低置信度文本
}

pub struct OcrResult {
    pub blocks: Vec<OcrBlock>,
    pub detected_language: Option<String>,
    pub cost_ms: u32,
}

pub struct OcrBlock {
    pub text: String,
    pub bbox: Rect,          // 逻辑坐标(相对虚拟屏),已做完回映射
    pub confidence: f32,     // 0..1
}
```

### 2.1 工厂

```rust
pub fn default_engine() -> Arc<dyn OcrEngine> {
    #[cfg(target_os = "macos")] { return Arc::new(macos::VisionOcr::new()); }
    #[cfg(target_os = "windows")] { return Arc::new(windows::WinMediaOcr::new()); }
    #[allow(unreachable_code)] { panic!("Unsupported OS"); }
}
```

`default_engine()` 在启动阶段构造一次,放 `State<Arc<dyn OcrEngine>>`。

## 3. macOS 实现

文件:`core/ocr/macos.rs`

- 依赖:`objc2`、`objc2-foundation`、`objc2-vision`(或手写 FFI)。
- 流程:
  1. `NSData::dataWithBytes` 包裹 PNG。
  2. `CIImage::imageWithData`。
  3. 构造 `VNImageRequestHandler`。
  4. `VNRecognizeTextRequest`:
     - `setRecognitionLevel: .accurate`(默认);`fast_mode` → `.fast`。
     - `setUsesLanguageCorrection: true`。
     - `setRecognitionLanguages: preferredLanguages`。
     - macOS 13+ 设置 `automaticallyDetectsLanguage`;macOS 12 不调用该 selector,设置页说明系统版本限制。
     - 显式使用 CPU:macOS 14+ 从每个阶段的支持设备列表选择 `MLCPUComputeDevice`;macOS 12/13 使用 `usesCPUOnly`。本机 GUI 进程自动选择的加速路径曾长时间卡在 TextRecognition 同步等待,切换 CPU 后真实截图 OCR 在 57–176ms 内返回。
  5. 同步执行 `perform:error:`(在 tokio `spawn_blocking` 里)。
  6. 遍历 `VNRecognizedTextObservation`:
     - `topCandidates: 1` 取最高候选。
     - bbox 在 Vision 里是**归一化坐标**(0..1,原点在左下)。需要:
       - 翻转 Y:`y = 1 - y - h`
       - 乘以图像像素尺寸 → 物理像素 bbox
       - 加上 `pixel_rect.origin` → 屏幕物理像素 bbox
       - 除以 `scale_factor` → 屏幕逻辑坐标 bbox

### 3.1 语言检测

若 `preferredLanguages` 为空:调用 `VNRecognizeTextRequest.supportedRecognitionLanguagesForTextRecognitionLevel` 获取系统支持列表,取前两位(通常包含系统语言与英语)。

## 4. Windows 实现

文件:`core/ocr/windows.rs`

- 依赖:`windows` crate(Graphics.Imaging / Media.Ocr / Storage.Streams)。
- 流程:
  1. `GetCurrentPackageFullName` 检查包身份,无身份返回 `OcrIdentityRequired`。
  2. `InMemoryRandomAccessStream` 写入 PNG;`BitmapDecoder::CreateAsync` → BGRA8 / Ignore alpha 的 `SoftwareBitmap`。
  3. `OcrEngine::TryCreateFromLanguage(Language)`:
     - 按 `preferred_languages` 依序尝试。若全部不可用且 `auto_detect_language = true` → fallback `OcrEngine::TryCreateFromUserProfileLanguages()`;关闭回退则返回 OcrUnavailable。
     - 若仍为 null → 返回 `LenzError::OcrUnavailable("no installed language pack")`。
  4. `engine.RecognizeAsync(bitmap)` → `OcrResult`。
  5. 遍历 `Lines`:
     - `Line.Text`:整行文本。
     - bbox:取 Line 内所有 `Word.BoundingRect` 的并集矩形(像素坐标,已经是绝对像素)。
     - 置信度:Win OCR **不返回置信度**,统一填 `0.9`(常量),并标记 `engine = "win-media-ocr"`。
     - 不使用合成置信度过滤文本。Windows 页说明系统不支持精度模式与置信度控制;macOS 对应控件真实生效。

### 4.1 语言包提示

优先语言和系统回退均不可用时返回 OcrUnavailable;F2/F3 发 `ocr://language-missing` 无载荷事件。主窗口跳到 OCR 页,显示实际安装语言、缺少语言包提示及 `ms-settings:regionlanguage` 入口;刷新重新枚举语言包。

识别语言支持添加、删除和优先顺序调整,全部持久化。Windows 工作线程初始化 COM apartment,异步 WinRT 操作用 windows-future 的 `join()` 完成。

### 4.2 桌面应用包身份与验证边界

[Microsoft 文档](https://learn.microsoft.com/en-us/uwp/api/windows.media.ocr)限定本 API 在桌面应用中的官方支持范围为具有 package identity 的应用。Lenz 使用完整 MSIX 包,清单及构建入口见 [spec 11 §4.2](./11-nfr.md#42-windows)。开始菜单通过包入口启动 `lenz.exe`;登录启动使用包内 `LenzStartup`,保持相同身份。普通 exe/MSI 开发路径不承诺系统 OCR 可用。

识别前检查实际进程身份。缺身份与缺语言包分别返回 `OcrIdentityRequired`、`OcrUnavailable`;F2/F3 分别发 `ocr://package-required`、`ocr://language-missing`,设置页跳到 OCR 并展示对应安装或语言入口。`get_system_info.windowsPackageIdentity` 用于显示实际进程状态;不从配置推断身份。

安装诊断入口:`lenz.exe --diagnose-windows-ocr <report.json>`。诊断在 GUI、用户配置和翻译服务初始化之前运行,使用嵌入的合成图片 `OcrSmoke.png` 调用同一 `run_win_ocr`,输出实际包身份、可用语言、文本块、识别文字及错误。优先英语并允许系统语言回退;要求结果含 `LENZ` 和 `12345`,失败退出码 1。安装脚本从已安装的 AUMID 激活诊断,核对身份等于 `PackageFullName`,不能用解包后直接执行 exe 代替。CI 准备英语 OCR 能力以稳定自检。诊断不截图、不读取用户配置或密钥。

当前已完成打包与诊断代码、交叉编译和本地静态检查;尚无 Windows 安装运行报告。只有 CI/Windows 的实际 `installed-ocr.json` 验收成功后,才能把安装后识别标为通过。

`preferredLanguages` 当前只选择首个可创建的识别器,每张图片执行一次识别。列表并非逐语言全部识别或自动检测图片语言;系统回退也只是选取用户语言列表中可用的识别器。

## 5. 坐标回映射算法

给定:
- OCR 平台返回的 bbox 物理像素(相对截图图像左上)`raw`
- 截图的 `pixel_rect`(相对虚拟屏物理像素)与 `scale_factor`

逻辑坐标 bbox:

```
phys_x = pixel_rect.x + raw.x
phys_y = pixel_rect.y + raw.y
phys_w = raw.w
phys_h = raw.h

logical_x = phys_x / scale_factor
logical_y = phys_y / scale_factor
logical_w = phys_w / scale_factor
logical_h = phys_h / scale_factor
```

多屏缩放不同的跨屏截图:在 capture 阶段已统一拼接到同一缩放坐标系(见 Spec 02 §2.2),故此处视作单一 `scale_factor`。

## 6. Tauri Command

```rust
#[tauri::command]
async fn run_ocr(
    rect: Rect,                       // 逻辑坐标:内部先截图再 OCR
    config: State<'_, ConfigStore>,   // OcrOptions 由 config.ocr 构造
    ocr: State<'_, Arc<dyn OcrEngine>>,
) -> Result<OcrResult, LenzError>;
```

> 实现说明:前端传 PNG 字节不现实,故 `run_ocr` 以**屏幕逻辑矩形**为入参,内部
> `capture_region(rect)` 截图后再调引擎 `recognize`;`OcrOptions` 取自 `config.ocr`。
> F2/F3 主链路共用相同 OCR 配置;设置页不另设测试 OCR 按钮。`OcrInput`/`OcrOptions`
> 仍是引擎层(`OcrEngine::recognize`)的入参。

## 7. 性能

- **目标**(对齐 PRD §5.1):1920×1080 区域 OCR ≤ 1s。
- 热状态基线(开发机,Apple M1):Vision `.accurate` 约 200~500ms;Windows 性能尚未实测。首次模型初始化不计入热状态目标。
- 2026-10-14 本机合成图验证:640×220 英文图片首次 Vision 识别约 25.9s,重复调用约 0.44s;原先截图/OCR 共用 10s 会误报超时。
- F2/F3 与 `run_ocr` 共用 `recognize_with_timeout`,OCR 独立限时 10s。Vision 按优先语言顺序、快速/精准模式和自动语言检测组成初始化配置;该配置首次成功前允许 60s,成功后恢复 10s。置信度只影响结果过滤,不重新计算初始化状态;识别失败不标记成功。
- Vision 同时最多执行一个原生识别。取消/超时只停止等待和后续结果投递,并发槽仍由阻塞线程持有直到原生调用结束,避免连续重试堆积原生任务。
- 日志记录引擎、尺寸、识别模式、语言数量、超时预算、块数和耗时,不记录识别文字或图像;原生结束日志在调用方取消后仍可用于诊断。
- 优化:
  - OCR 调用用 `tokio::task::spawn_blocking`,避免阻塞异步 runtime。
  - 识别前不做无谓 PNG re-encode(capture 已经是 PNG 则直接传)。
  - `min_confidence` 过滤在 Rust 侧执行,减少前端/翻译侧负担。

## 8. 验收标准

- 使用仓库内 fixture 图(中英混排截图)识别,文本块数量与人工标注一致 ±1。
- bbox 回映射后,在屏幕上叠加彩色方框,能与原文字对齐(误差 ≤ 2 逻辑像素)。
- 无任何已安装语言包的 Windows,调用应 graceful 返回 `OcrUnavailable`,不崩溃。
- Windows 从安装包入口运行合成图片自检,包身份与安装记录一致且识别文字含 `LENZ 12345`。
- 普通无身份 exe 返回 `OcrIdentityRequired`,不误导为缺少语言包。
- `cargo test --features ocr-test` 跑通(离线 fixture 测试)。

## 9. 测试点

- 中文简繁 / 日语 / 英文 / 韩语四组 fixture。
- 反色图像(深底白字)。
- 文字带轻微旋转(±5°)。
- 超长单行文本(>200 字符)。
- 多段落间距很小(粘连)场景,确认 block 切分合理。

## 10. 扩展点

- 未来新增 Tesseract 实现:`core/ocr/tesseract.rs`,实现同一 trait,通过 `config.ocr.engine = "tesseract"` 切换。
- 云端 OCR(Azure / Google Vision):同样走 trait,但需要注意隐私声明。
