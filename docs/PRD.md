# PRD:Lenz

> 一款基于 Tauri 的跨平台(macOS + Windows)截图翻译桌面工具,支持划词翻译、截图翻译、原图覆盖翻译,主打本地 LLM + 隐私友好。
>
> **Lenz** — 像镜头一样,一眼看懂世界。

---

## 1. 项目概述

### 1.1 背景与动机

Bob 是 macOS 上广受好评的翻译工具,但:
- **仅支持 macOS**,Windows 用户无同类体验
- **订阅制收费**,OCR 与离线翻译已进入付费墙
- **对本地 LLM 接入不够友好**,主要面向在线 API

本项目目标是做一个**开源、跨平台、本地优先**的截图翻译工具,聚焦 Bob 最核心的几个翻译场景,砍掉冗余功能。

### 1.2 产品定位

- **目标用户**:开发者、研究者、跨境工作者,尤其是注重隐私/想用本地 LLM 的用户
- **核心价值**:
  1. 跨平台:Mac + Windows 统一体验
  2. 开源免费:MIT License,GitHub 发布
  3. 本地优先:原生支持 LM Studio / Ollama / 任意 OpenAI 兼容接口
  4. 轻量高效:Tauri 打包,安装包 < 15MB

### 1.3 非目标(明确不做)

- ❌ 语音翻译 / TTS 朗读
- ❌ 文档整篇翻译(PDF / Word)
- ❌ 浏览器插件 / 沉浸式网页翻译
- ❌ 移动端(iOS / Android)
- ❌ 商业化 / 付费版本
- ❌ 协作 / 云同步 / 账号体系

---

## 2. 核心功能

### 2.1 功能清单(MVP)

| 模块 | 优先级 | 说明 |
|---|---|---|
| 划词翻译 | P0 | 选中文本 → 快捷键或自动弹框 |
| 截图翻译 | P0 | 框选区域 → OCR → 弹框显示译文 |
| 原图覆盖翻译 | P0 | 框选区域 → 译文按原位置覆盖显示 |
| 翻译引擎配置 | P0 | 支持 LM Studio / OpenAI 兼容 API |
| 状态栏图标 | P0 | 常驻菜单栏,快捷入口 |
| 主窗口(设置页) | P0 | 配置引擎/快捷键/主题等 |
| 多语言界面 | P1 | 中/英 |
| 主题 | P1 | 深/浅色 |
| 自动启动 | P1 | 系统启动时自动运行 |
| 检查更新 | P1 | GitHub Release 轮询 |

---

### 2.2 功能详述

#### F1. 划词翻译

**触发方式**(用户可配置,二选一或都启用):
- **图标触发**:鼠标选中文字后浮出图标,hover/点击后翻译;默认关闭监听
- **快捷键触发**:选中文本后按快捷键(默认 `⌥ D` / `Alt+D`)弹出译文

**交互流程**:
```
用户选中文本 → 触发 → 获取剪贴板/选区文本 → 调翻译引擎 → 弹框显示译文
```

**弹框位置**:默认在当前选中文字下方水平居中,留 12pt 间距;下方空间不足时移到上方,并限制在选区所在屏幕工作区内。无法取得选区边界时使用触发瞬间的鼠标位置,不随后续鼠标移动。

**弹框内容**:
- 原文(可折叠)
- 译文
- 源语言 / 目标语言标识(自动检测显示)
- 关闭按钮(或点击外部关闭)

**验收标准**:
- 选中文本 → 弹框出现 ≤ 1s(不含翻译耗时)
- 弹框优先贴近选区下方,内容增减沿用同一选区锚点;手动拖动后尊重用户位置,不超出屏幕边界。
- 取词失败明确提示重新选中或辅助功能授权,不展示无关的上次结果。
- 再次选中新文本,弹框内容更新而非新增

---

#### F2. 截图翻译

**触发方式**:全局快捷键(默认 `⌥ S` / `Alt+S`)

**交互流程**:
```
快捷键触发 → 进入截图模式(全屏遮罩 + 十字光标)
→ 用户框选区域 → 截取图片
→ 所选 OCR 服务识别 → 得到文字(系统/百度/腾讯云另含坐标)
→ 调翻译引擎 → 得到译文
→ 弹框显示(原文 + 译文)
```

**弹框位置**:跟随截图区域附近弹出

**流程约束**:截图前隐藏翻译弹窗与划词图标,抓屏完成后再显示进度。F2/F3 的截图独立限时 10s,系统 OCR 常规限时 10s;macOS Vision 首次成功识别某组语言/模式前允许 60s 初始化,第三方 OCR 使用服务自身 `timeoutMs`,不扣减截图或窗口握手耗时。翻译使用引擎配置的 `timeoutMs`(允许本地模型较长响应)。新请求或关闭弹窗取消旧翻译;手动重译和快捷键结果按会话隔离。翻译超时保留原文供重试。

**验收标准**:
- 按快捷键到进入截图模式 ≤ 300ms
- 框选完成到弹框出现 ≤ 3s(本地 OCR + 本地 LLM)
- 支持 ESC 取消截图

---

#### F3. 原图覆盖翻译(核心差异化)

**触发方式**:全局快捷键(默认 `⌥ F` / `Alt+F`)

**交互流程**:
```
快捷键触发 → 进入截图模式 → 用户框选区域
→ 截图完成即创建可交互蒙层,保留选区截图,周围背景变暗
→ 工具条显示“识别中”与 loading,后台 OCR(得到文本块及坐标)
→ 工具条切换“翻译中”,批量翻译所有文本块
→ 按原文本 bbox 渲染译文:
   - 背景:取原区域颜色模糊/覆盖
   - 文字:自适应字号,保证译文填充
→ 点击工具条 × 或按 Esc 关闭并恢复原样
```

**视觉效果(中等方案)**:
- 保留用户选区截图,每个文本块按原位置覆盖译文
- 原区域用"取色填充"(采样原区域边缘平均色)覆盖原文
- 译文叠加在上层,字号根据 bbox 尺寸自适应
- 译文颜色根据背景亮度自动选黑/白

**验收标准**:
- 截图完成后立即显示蒙层与关闭入口,不等待 OCR/翻译;热状态首个交互反馈目标 ≤ 1s
- 识别与翻译期间持续显示相应 loading,完成后在同一蒙层覆盖译文;失败在蒙层内提示
- 本地热状态框选完成到译文显示目标 ≤ 4s,远端翻译耗时受服务影响
- 译文不超出原文 bbox 区域(超出自动缩小字号)
- 蒙层获得焦点并接收鼠标/键盘;工具条提供明显的 × 和 Esc 关闭提示
- 点击 × 或按 Esc 关闭、恢复下层交互;托盘「关闭覆盖层」保留为后备入口

---

#### F4. 翻译引擎

**支持类型**:
1. **OpenAI 兼容接口**(LM Studio / Ollama / DeepSeek / OpenAI 官方等)
2. **传统翻译 API**:Google Translate / DeepL / 有道翻译
3. **Apple 系统翻译**:macOS 15+ 系统 Translation;先在系统设置下载源语言和目标语言包,缺包时明确提示并提供设置入口;Windows 使用其他配置引擎
4. **插件化接口**:预留扩展点,未来可通过插件机制扩展

**配置项**(每个引擎实例):
- 名称(自定义)
- 类型(下拉选择)
- API URL
- API Key
- 模型名(LLM 类引擎)
- System Prompt(LLM 类引擎,支持变量 `{sourceLang}` `{targetLang}`)
- 超时时间
- 启用/禁用开关

**多引擎管理**:
- 用户可配置多个引擎
- 设置默认引擎
- 划词/截图翻译弹窗按折叠策略并行请求已启用服务,按设置顺序逐项展示结果、耗时和独立复制按钮;默认服务标记为“默认”,先返回的结果先显示。单项失败/超时不影响其他服务,支持单项重试。
- 每个服务标题可折叠/展开,行内菜单独立设置“记住折叠状态”和“折叠时不自动翻译”。未记住时下次查询默认展开;记住后跨查询和重启恢复。折叠跳过的服务展开时再翻译,折叠时取消该服务未完成的请求;未开启跳过则折叠后继续翻译,展开复用已有结果。
- 编辑原文、切换/互换语言后按各服务折叠策略重新请求;关闭弹窗或新请求取消整个旧会话,迟到结果不能写回。原图覆盖使用默认服务。

**翻译方向**:
- 源语言:自动检测(基于 OCR 结果或文本内容)
- 目标语言:默认中文,用户可改

---

#### F5. OCR 引擎

**支持服务**:系统 OCR、百度 OCR、腾讯云 OCR、通用视觉模型接口,默认系统 OCR。设置页支持添加、编辑、删除第三方服务、切换默认服务和测试识别。

**系统原生 OCR**:
- **macOS**:Apple Vision 框架(`VNRecognizeTextRequest`)
- **Windows**:Windows.Media.Ocr API

**系统识别语言**:跟随系统 OCR 能力
- macOS Vision:中英日韩等数十种
- Windows OCR:需安装具有包身份的 MSIX 版本并通过开始菜单或包内登录启动入口运行;系统需安装对应 OCR 语言包。安装应用不要求本地 OCR 或语言包,不默认执行 OCR 自检;可直接在安装后配置腾讯云等服务。需要系统 OCR 时再从设置页添加 Windows OCR 语言包并测试;独立合成图片诊断区分缺包身份与缺语言包。

**百度 OCR**:使用文字识别应用的 API Key + Secret Key,支持标准/高精度含位置版、识别语言与服务超时;支持截图翻译及原图覆盖翻译。

**腾讯云 OCR**:使用腾讯云 SecretId + SecretKey 调用通用文字识别高精度版,提供通用中英文/多语种场景、置信度过滤和服务超时,支持截图翻译及原图覆盖翻译。截图在内存中编码并上传腾讯云,Base64 后限制 10M,超限提示缩小选区。凭据沿用当前本机明文配置存储。

**通用视觉模型**:使用支持图片输入的 OpenAI Chat Completions 兼容接口,配置接口地址、可选 API Key、模型名和超时。当前只用于截图翻译;选为默认时,原图覆盖继续使用系统 OCR,不向视觉模型发送图片。

**测试与隐私**:测试识别内置“LENZ OCR TEST 12345”样例并显示识别文字及耗时,不截取用户屏幕。第三方 OCR 在识别时上传图片给用户配置的服务,设置页明确提示;系统 OCR 在本机运行。系统服务不可删除,删除当前默认第三方服务后恢复系统服务。

**OCR 返回数据结构**:
```typescript
interface OCRResult {
  blocks: Array<{
    text: string;
    bbox: { x: number; y: number; w: number; h: number }; // 屏幕逻辑坐标
    confidence: number;
  }>;
  text: string | null; // 视觉模型的完整原文;其 blocks 为空,不伪造坐标
  detectedLanguage?: string;
}
```

**架构设计**:OCR 引擎抽象为 trait/接口,按每次请求的服务配置快照选择实现,切换无需重启;后续可扩展 Tesseract / PaddleOCR。

---

### 2.3 系统功能

#### 状态栏图标
- 常驻菜单栏
- 菜单项:
  - 打开主窗口
  - 划词翻译 / 截图翻译 / 原图翻译(同快捷键，使用原生快捷键提示统一靠右显示)
  - 暂停/启用划词监听
  - 设置(应用内快捷键:macOS `Command+,`,Windows `Ctrl+,`)
  - 退出

#### 主窗口(设置页)
- Tab 结构:
  - **通用**:启动/更新/主题/语言、划词显示翻译图标开关(与托盘「划词监听」同一开关)
  - **快捷键**:各功能快捷键自定义
  - **服务**:文本翻译（引擎增删改、超时和测试）/ OCR（服务增删改、默认切换、样例识别测试;按服务能力展示精度、置信度和语言设置）
  - **关于**:版本信息、开源链接、许可证

头部提供中英文设置搜索及跳转;连接状态依据最近测试结果;关于页显示实际版本、构建日期、Bundle ID、发布入口和离线许可证。macOS 提供辅助功能/屏幕录制引导,Windows 提供 OCR 语言包入口。

#### 自动启动
- 用户可在通用设置里开关
- Windows 普通 EXE 的开始菜单与安装完成页明确显示主窗口;后台启动仍遵循「启动时打开主窗口」设置。启动发生致命错误时显示提示并保留本地日志。
- 实现:
  - macOS:`LSUIElement` + Launch Agent
  - Windows MSIX:包内 `StartupTask`;普通 EXE 版和无包开发版使用注册表 `Run` 键

#### 检查更新
- 开启时启动 3s 后检查,每 24h 检查;重新开启立即检查,支持手动检查
- 拉取正式 GitHub Releases API,按语义版本比较
- 新版本显示提示和发布页入口;无发布/网络错误分别反馈;不自动安装

> 应用内代理设置已移除:不再内置代理配置。翻译请求沿用 reqwest 默认行为(尊重系统环境变量代理);如需代理访问云端引擎,由用户在系统/环境层配置。

---

## 3. 技术方案

### 3.1 技术栈

| 层 | 技术 |
|---|---|
| 框架 | Tauri 2.0 |
| 前端 | React 18 + TypeScript + Vite |
| UI 库 | shadcn/ui + Tailwind CSS |
| 状态管理 | Zustand |
| 后端语言 | Rust |
| 存储 | JSON 文件(serde_json) |

### 3.2 关键依赖(Rust / Tauri)

```toml
# 截图
xcap = "0.0"                              # 跨平台屏幕截图

# 窗口 / 系统
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-global-shortcut = "2"
tauri-plugin-autostart = "2"
tauri-plugin-updater = "2"
tauri-plugin-clipboard-manager = "2"

# 网络
reqwest = { version = "0.12", features = ["json"] }

# OCR(原生桥接,通过 FFI)
# macOS: objc2 + Vision framework
# Windows: windows crate + Windows.Media.Ocr

# 工具
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

### 3.3 项目结构

```
lenz/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs                # 入口
│   │   ├── commands/              # Tauri commands(前端调用接口)
│   │   │   ├── translate.rs
│   │   │   ├── ocr.rs
│   │   │   ├── capture.rs
│   │   │   └── config.rs
│   │   ├── core/
│   │   │   ├── capture.rs         # 屏幕截图
│   │   │   ├── ocr/
│   │   │   │   ├── mod.rs         # OCR trait
│   │   │   │   ├── macos.rs       # Vision 实现
│   │   │   │   └── windows.rs     # Win OCR 实现
│   │   │   ├── translator/
│   │   │   │   ├── mod.rs         # Translator trait
│   │   │   │   ├── openai.rs      # OpenAI 兼容
│   │   │   │   ├── google.rs      # Google Translate
│   │   │   │   ├── deepl.rs
│   │   │   │   └── youdao.rs
│   │   │   ├── overlay.rs         # 原图覆盖窗口管理
│   │   │   └── selection.rs       # 划词监听
│   │   └── config/
│   │       ├── mod.rs
│   │       └── schema.rs          # 配置 schema
│   └── Cargo.toml
├── src/                           # 前端
│   ├── main.tsx
│   ├── App.tsx
│   ├── windows/
│   │   ├── main/                  # 主设置窗口
│   │   ├── popup/                 # 翻译结果弹框
│   │   ├── selection/             # 截图选区蒙层
│   │   └── overlay/               # 原图覆盖层
│   ├── components/
│   ├── hooks/
│   ├── stores/
│   └── lib/
├── package.json
└── README.md
```

### 3.4 关键技术点

#### 截图与选区
- 用 `xcap` 获取完整屏幕位图
- 创建全屏透明窗口 + Canvas 实现选区框
- 截取选区后传给 OCR

#### OCR 桥接
- **macOS**:Rust → `objc2` crate → 调 Vision `VNRecognizeTextRequest`
- **Windows**:Rust → `windows` crate → 调 `OcrEngine.RecognizeAsync`
- 统一返回 `OCRResult` 结构

#### 原图覆盖窗口
- 创建 Tauri 窗口:
  ```json
  {
    "transparent": true,
    "decorations": false,
    "alwaysOnTop": true,
    "skipTaskbar": true,
    "focus": true
  }
  ```
- 蒙层覆盖来源显示器并接收交互,不注册临时全局 Escape
- 选区截图按原位置显示,前端按 bbox 定位译文块;工具条 × 与本地 Esc 统一关闭

#### 划词监听
- **macOS**:`NSPasteboard` 轮询 + `CGEventTap` 检测选中事件
- **Windows**:`SetWinEventHook` 监听 `EVENT_SYSTEM_FOREGROUND` + 剪贴板
- 权限:macOS 需要"辅助功能"权限

#### 翻译引擎抽象
```rust
#[async_trait]
pub trait Translator {
    async fn translate(&self, req: TranslateRequest) -> Result<TranslateResponse>;
    fn name(&self) -> &str;
    fn supports_batch(&self) -> bool;
}

pub struct TranslateRequest {
    pub text: String,           // 单条文本
    pub texts: Vec<String>,     // 批量(原图翻译用)
    pub source_lang: Option<String>,
    pub target_lang: String,
}
```

---

## 4. 交互与 UI

### 4.1 默认快捷键

| 功能 | macOS | Windows |
|---|---|---|
| 划词翻译 | `⌥ D` | `Alt+D` |
| 截图翻译 | `⌥ S` | `Alt+S` |
| 原图覆盖翻译 | `⌥ F` | `Alt+F` |
| 打开主窗口 | `⌥ ⌘ T` | `Ctrl+Alt+T` |

均可用户自定义。

另有固定的应用内「设置」快捷键:macOS `Command+,`,Windows `Ctrl+,`;仅在 Lenz 窗口获得焦点时触发,不注册为全局快捷键。

### 4.2 弹框设计原则
- 最小化视觉干扰
- 圆角 + 毛玻璃背景(macOS vibrancy / Windows acrylic)
- 字号 14px,padding 12px
- 支持键盘操作(ESC 关闭,Enter 复制译文,预留 MVP+1)

### 4.3 主题
- 跟随系统 / 强制深色 / 强制浅色
- 使用 CSS 变量统一管理

### 4.4 应用图标
- 桌面与安装包图标使用冰蓝至浅蓝紫的渐变圆角底板,保留蓝紫色 L 与「文 / A」双气泡。
- 窗口内品牌标记保持透明背景,托盘沿用独立模板图标(→ spec 09 §2.1)。

---

## 5. 非功能需求

### 5.1 性能

| 指标 | 目标 |
|---|---|
| 应用启动到常驻托盘 | ≤ 2s |
| 空闲内存占用 | ≤ 150MB |
| 划词到弹框(不含翻译) | ≤ 500ms |
| 截图到 OCR 完成 | ≤ 1s(1920×1080 区域) |
| 安装包大小 | ≤ 15MB |

### 5.2 隐私
- 所有截图数据仅保留在内存中,不落盘
- 不做任何数据上报 / 埋点
- 开源可审计

### 5.3 兼容性
- macOS 12+(Vision 框架要求)
- Windows 10 1809+(Windows.Media.Ocr 要求)
- Windows 默认提供普通 EXE 安装包,无需手动导入证书,安装后可直接配置腾讯云等 OCR。另保留 MSIX + 安装助手 ZIP,供以后需要 Windows 系统 OCR 时可选安装;仅导入证书不会给普通 EXE 赋予包身份。MSIX 测试签名阶段需用户明确检查并信任证书。
- 架构:x64 + ARM64

---

## 6. 开发计划

### 6.1 里程碑(12 周)

| 阶段 | 周数 | 目标 |
|---|---|---|
| M1 项目搭建 | W1 | Tauri 项目初始化、基础窗口、托盘图标 |
| M2 OCR 桥接 | W2-3 | macOS Vision + Windows OCR 完整打通 |
| M3 翻译引擎 | W4 | OpenAI 兼容接口 + 配置体系 |
| M4 截图翻译 | W5-6 | 完整的 F2 功能 |
| M5 划词翻译 | W7 | F1 功能(Mac 优先,Win 后补) |
| M6 原图覆盖 | W8-9 | F3 核心差异化功能 |
| M7 设置页 | W10 | 主窗口 UI + 所有配置项 |
| M8 完善 | W11 | 主题、i18n、自启、更新 |
| M9 打包发布 | W12 | 双平台构建、GitHub Release、文档 |

### 6.2 风险与应对

| 风险 | 影响 | 应对 |
|---|---|---|
| Vision / Win OCR Rust 桥接复杂度 | 高 | W2 优先验证可行性,不行则上 Tesseract |
| 划词监听权限 / 稳定性问题 | 中 | Mac 要用户授权辅助功能,Win 用 UIA |
| 原图覆盖层在多屏 / HiDPI 下对位偏移 | 中 | 早期做多屏测试,统一用屏幕逻辑坐标 |
| 打包签名(尤其 Mac 公证) | 中 | 早期走完整 CI 流程验证 |
| LLM 输出格式不稳定(批量翻译 JSON 结构) | 低 | 用 structured output 或加解析兜底 |

---

## 7. 后续规划(P2 / 非 MVP)

- 弹框操作:复制、收藏、TTS 朗读
- 翻译缓存(相同文本不重复调用)
- 流式输出(长文本逐字显示)
- 分段 / 合并翻译策略
- OCR 引擎可切换(Tesseract / PaddleOCR)
- 插件系统(用户自定义翻译引擎)
- 导入导出配置
- 词典模式(单词显示音标/释义/例句)

---

## 8. 开源与社区

- License:MIT
- 仓库:`github.com/<user>/lenz`
- 主页域名(建议抢注):`lenz.app` / `lenz.tools` / `getlenz.com`
- Bundle ID:
  - macOS:`com.lenz.app` 或 `app.lenz.desktop`
  - Windows:`Lenz`
- 欢迎 PR,提供详细的开发文档
- Issue 模板、PR 模板
- 提供双语 README(中/英)

---

**文档版本**:v0.3  
**最后更新**:2026-10-14
**变更记录**:
- v0.3(2026-06-15):移除历史记录功能
- v0.2(2026-04-24):项目定名 Lenz,补充 Bundle ID 与域名建议
- v0.1(2026-04-24):初版
