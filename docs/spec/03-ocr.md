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
    pub fast_mode: bool,                   // Vision / WinOCR 都有精度/速度开关
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
  1. `InMemoryRandomAccessStream` 写入 PNG。
  2. `BitmapDecoder::CreateAsync` → `SoftwareBitmap`。
  3. `OcrEngine::TryCreateFromLanguage(Language)`:
     - 按 `preferred_languages` 依序尝试。若全部不可用 → fallback `OcrEngine::TryCreateFromUserProfileLanguages()`。
     - 若仍为 null → 返回 `LenzError::OcrUnavailable("no installed language pack")`。
  4. `engine.RecognizeAsync(bitmap)` → `OcrResult`。
  5. 遍历 `Lines`:
     - `Line.Text`:整行文本。
     - bbox:取 Line 内所有 `Word.BoundingRect` 的并集矩形(像素坐标,已经是绝对像素)。
     - 置信度:Win OCR **不返回置信度**,统一填 `0.9`(常量),并标记 `engine = "win-media-ocr"`。

### 4.1 语言包提示

若 `TryCreateFromLanguage` 返回 null,事件 emit:`ocr://language-missing` `{ lang: "ja" }`。
前端在设置页 OCR tab 显示:"系统未安装日语识别包,[点此前往系统设置]"。

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
> 设置页「测试 OCR」按钮直接调;F2/F3 主链路复用同一路径。`OcrInput`/`OcrOptions`
> 仍是引擎层(`OcrEngine::recognize`)的入参。

## 7. 性能

- **目标**(对齐 PRD §5.1):1920×1080 区域 OCR ≤ 1s。
- 实测基线(开发机,Apple M1):Vision `.accurate` 约 200~500ms,Win OCR 约 300~700ms。
- 优化:
  - OCR 调用用 `tokio::task::spawn_blocking`,避免阻塞异步 runtime。
  - 识别前不做无谓 PNG re-encode(capture 已经是 PNG 则直接传)。
  - `min_confidence` 过滤在 Rust 侧执行,减少前端/翻译侧负担。

## 8. 验收标准

- 使用仓库内 fixture 图(中英混排截图)识别,文本块数量与人工标注一致 ±1。
- bbox 回映射后,在屏幕上叠加彩色方框,能与原文字对齐(误差 ≤ 2 逻辑像素)。
- 无任何已安装语言包的 Windows,调用应 graceful 返回 `OcrUnavailable`,不崩溃。
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
