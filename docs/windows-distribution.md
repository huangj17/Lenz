# Windows EXE / MSIX 分发与 OCR 验收

Windows 提供两种安装方式:

| 方式 | 用途 | 手动导入证书 |
|---|---|---|
| 普通 EXE（推荐） | 安装后直接配置腾讯云、百度等 OCR | 不需要 |
| MSIX + 安装助手 ZIP（可选） | 以后需要 Windows 系统 OCR 时切换 | 当前测试签名包需要 |

普通 EXE 直接安装应用文件,不部署 MSIX。系统 OCR 依赖 MSIX 包身份;仅向证书库导入证书不会给普通 EXE 增加身份,必须安装并运行 MSIX 版本。

当前没有正式代码签名证书。v0.1.0 按发布安排先分发自签测试包,Windows 安装/OCR 实测推迟;正式可信签名仍待完成,见 [TODO](./TODO.md)。

`tauri.windows.conf.json` 禁用 Windows 默认 MSI/NSIS bundler;普通 `pnpm tauri build` 只构建 exe。Windows 分发通过下方 MSIX 及 EXE 向导入口,macOS 默认打包配置不受此覆盖文件影响。

## 1. 构建测试包

在 Windows 10 1809+ / Windows 11 开发机准备 Node 22.6+、pnpm、Rust stable/MSVC、Visual Studio C++ build tools 和 Windows 10/11 SDK(含 MakeAppx、SignTool)。从仓库根目录运行:

```powershell
pnpm install --frozen-lockfile
pnpm windows:msix:test
```

输出位于 `src-tauri/target/msix/test/`:

- `Lenz-<四段版本>-x64.msix`:完整测试签名包。
- `Lenz-Test.cer`:公开测试证书,不含私钥。
- `Install-Lenz.ps1`、`MsixTools.ps1`、`Test-InstalledOcr.ps1`:安装及真实识别助手。
- `package-info.json`、`TEST-SIGNATURE.txt`:构建身份、架构、运行时依赖和测试签名说明。

本地构建脚本默认创建 14 天测试证书;标签发布构建设置 `-CertificateValidityDays 365`。签完删除构建机上的该私钥,不自动信任证书。安装前检查 `.cer` 的 Subject/指纹/有效期,再通过证书安装向导明确导入「本地计算机 → 受信任人」。测试包签名不代表公开可信的生产签名。

## 2. 安装并识别

### 方式一:普通 EXE 安装包（推荐）

下载 `Lenz_<版本>_windows_x64_setup.exe`,双击按向导安装。此包无需手动导入证书,不运行 MSIX 部署或本地 OCR 检查。默认安装到当前用户的 `%LOCALAPPDATA%/Programs/Lenz`,可选择其他目录;提供开始菜单入口与「Windows 设置 → 应用 → Lenz (EXE)」卸载项。卸载保留应用配置,仅清理安装器管理的文件和匹配安装路径的自启项。

若缺少 WebView2,向导下载微软官方 bootstrapper,校验微软签名后安装。EXE 自身当前尚无公开可信签名,Windows 可能显示 SmartScreen/未知发布者提示,与手动导入 MSIX 测试证书不同。

安装完成后打开 Lenz,进入「设置 → 服务 → OCR」添加腾讯云 SecretId/SecretKey,选择腾讯云并设为默认。无需安装系统 OCR 语言包。升级或卸载前,先从托盘菜单退出 Lenz。应用配置不因 EXE 重装或卸载而删除。

开始菜单与安装完成页使用 `--show-main` 显示主窗口,即使「启动时打开主窗口」关闭也能进入设置。登录启动仍保留后台启动偏好。

若点击后没有窗口和托盘,可通过 Win+R 打开 `%LOCALAPPDATA%\app.lenz.desktop\logs` 检查日志。新构建会在主线程致命错误时弹出原生提示,并额外写入 `startup-error.log`。反馈时提供错误文本和日志末尾即可,不要发送含服务密钥的 `config.json`。这类启动错误无需通过导入 OCR 证书解决。

完成页提供以后需要系统 OCR 时的 MSIX 下载链接,该操作为可选项,不会自动导入证书。

### 方式二:MSIX + 安装助手 ZIP（系统 OCR 可选版本）

确认需要 Windows 系统 OCR 后,下载并解压 ZIP。检查公开 `Lenz-Test.cer` 的 Subject/指纹/有效期,确认信任来源后通过证书向导导入「本地计算机 → 受信任人」。仅导入「当前用户」无效。普通 EXE 无需这一步。

保持安装助手文件在同一目录,运行:

```powershell
powershell -NoProfile -File .\Install-Lenz.ps1 -PackagePath .\Lenz-0.1.2.0-x64.msix -Launch
```

助手通过标准 `Add-AppxPackage` 让 Windows 验证包签名/内容/Publisher/证书信任并安装,再检查 WebView2,缺少时下载官方 bootstrapper 并验证 Microsoft 签名后安装。默认仅安装应用和检查 WebView2,不执行 OCR 诊断;`-Launch` 从已安装的 AUMID 启动应用。助手没有允许 unsigned 或注册散装 manifest 的旁路,不代替测试证书的手动信任步骤。

### 可选 MSIX 的证书错误

`0x800B0109` 表示 MSIX 签名证书不受信任,与 OCR 语言包无关。检查公开证书后明确导入「本地计算机 → 受信任人」再安装;安装助手不自动添加信任。普通 EXE 不进行 MSIX 部署,不要求导入此证书。[Microsoft 说明](https://learn.microsoft.com/en-us/windows/msix/app-installer/troubleshoot-appinstaller-issues#trusted-certificates)。

### 以后需要本地 OCR 时

先从普通 EXE 切换到 MSIX 版本,并从 MSIX 开始菜单入口运行。建议先退出并卸载「Lenz (EXE)」,避免双实例及混淆启动入口;EXE 卸载保留配置,但跨分发形式的配置共享与迁移仍待 Windows 实机验证。系统 OCR 使用 Windows 自带能力,无需安装独立 OCR 引擎。进入 Lenz 的系统 OCR 设置,通过「安装语言包」打开 Windows 语言设置,添加需要的识别语言后刷新并测试。使用腾讯云或百度 OCR 无需此步骤。

如需诊断整个安装包身份与真实识别,在 ZIP 解压目录运行:

```powershell
powershell -NoProfile -File .\Test-InstalledOcr.ps1
```

诊断要求实际 `packageIdentity` 等于安装记录的 `PackageFullName`,并识别合成图片中的 `LENZ`、`12345` 和至少一个带 bbox 的文本块。报告写到临时目录,路径会打印出来;也可用 `-ReportPath <新路径>` 指定。诊断不初始化 GUI、用户配置或翻译接口,不截图、不读取 API Key。

独立诊断失败会返回错误,已安装的应用保留。安装助手的可选 `-CheckOcr` 在自检失败时仅警告,不会把应用安装判为失败;`-OcrReportPath` 与此参数一同使用。可在「设置 → 时间和语言 → 语言」补齐 OCR 语言能力,从开始菜单打开 Lenz 的 OCR 页刷新语言列表,再重新运行上面的独立诊断。

诊断优先选择英语并允许系统语言回退;CI 会准备英语 OCR 能力。若已有语言识别器无法读出合成英语图,安装英语 OCR 能力后重试。旧参数 `-SkipOcrCheck` 兼容保留,现在默认即跳过自检;不能与 `-CheckOcr` 同时使用。跳过不代表识别验收通过。

双击 MSIX 也可安装;较新的 Microsoft App Installer 会处理清单中的 WebView2 ExternalDependency。旧版 App Installer 或直接 `Add-AppxPackage` 不保证处理此依赖,因此推荐上述助手。双击安装后可直接配置腾讯云等服务;只有需要验收系统 OCR 时再独立运行诊断。

### 生成普通 EXE

在 Windows 开发机安装 NSIS 3 (`choco install nsis`),从仓库根目录运行:

```powershell
pnpm windows:exe
```

脚本自行使用 MSVC/静态 CRT 构建应用,不需要签名证书。若已有相同版本的 MSVC 构建,可复用:

```powershell
pnpm windows:exe -BinaryPath .\src-tauri\target\x86_64-pc-windows-msvc\release\lenz.exe
```

输出为 `src-tauri/target/exe/Lenz_0.1.2_windows_x64_setup.exe`。支持 `-OutputDirectory`、`-MakeNsisPath`、`-Architecture arm64`;动态 Loader 需显式传入匹配架构的 `-LoaderPath`。生成前核对 PE 架构、静态运行时及 Loader,仅暂存明确列出的文件,不读取或嵌入 MSIX/证书。NSIS 创建当前用户安装器、开始菜单和卸载器;安装阶段仅调用 WebView2 助手。

## 3. 正式签名

正式分发需公开可信的代码签名证书或 Microsoft Store 签名。首次正式发布前确定并固定 `src-tauri/msix/identity.json` 的 Name/Publisher;Publisher 必须与签名证书 Subject 完全一致。保持后续升级的包身份,不要用测试 Publisher 代替正式身份。

证书已在当前用户的个人证书库且包含私钥时:

```powershell
powershell -NoProfile -File scripts/windows/Build-Msix.ps1 -CertificateThumbprint <证书指纹> -TimestampUrl <证书提供商的RFC3161地址>
```

构建会使用该证书的 Subject,MSVC 静态 CRT和 WebView2 Loader,拒绝缺失 DLL、外部 VC++ runtime 或架构不符的输入。支持 `-Architecture arm64`,原生 ARM64 包尚待 Windows 验收。已有 exe 可用 `-BinaryPath`;动态 Loader 必须提供匹配的 `-LoaderPath`。`-PackageVersion` 可显式指定四段整数版本,预发布 semver 不能直接当作 MSIX 版本。`-Unsigned` 只供内容检查,不会产出可正常安装的正式包。

## 4. CI 与发布前验收

[Windows EXE 启动工作流](../.github/workflows/windows-exe.yml)可检查当前发布包或重新构建当前源码。在一次性 Windows Server runner 上静默安装普通 EXE,确认进程完成初始化,再连续三次验证 `--show-main` 后存在可见的 Lenz 主窗口并在前端首轮 IPC 后保持运行。保存 stdout/stderr 与状态报告;该测试不依赖 MSIX、证书或本地 OCR,不代替 Windows 10/11 用户机器、交互流程和腾讯云真实识别验收。

[Windows MSIX 工作流](../.github/workflows/windows-msix.yml)在 Windows Server runner 原生 MSVC 构建测试包,再交给 Windows 11 ARM 客户端安装 x64 包并运行真实系统 OCR。临时信任仅限 GitHub-hosted 一次性机器;专用包名为 `app.lenz.desktop.ci`,测试结束清理包和证书。产物只包含公开证书、安装包、脚本与实际识别报告,没有 PFX/私钥。

构建阶段复用同一 MSVC 可执行文件生成普通 EXE,与 MSIX 分别保存为 artifact,避免改变 MSIX 安装测试的下载目录结构。工作流仅手动触发;选中 `verify_installed_ocr` 才会运行客户端安装 job,默认只构建。v0.1.0 推迟此运行验收。客户端 job 需要仓库可用的 `windows-11-arm` runner;此测试路径覆盖 x64 模拟,Windows 10/x64 原生、ARM64 包、语言包缺失和登录启动后的身份仍单独验收。交叉编译或安装包构建成功不能替代这份 Windows 报告。

发布前还要验证:全新安装和升级、缺 WebView2 时安装、F2/F3 实际截图识别、中英 OCR 语言切换、无语言包的提示、设置自启后重新登录以及关闭自启。尚无正式证书时只提供测试包,不发布为正式可信安装版本。

官方依据:[OCR 包身份要求](https://learn.microsoft.com/en-us/uwp/api/windows.media.ocr)、[MSIX 证书](https://learn.microsoft.com/en-us/windows/msix/package/create-certificate-package-signing)、[WebView2 分发](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution)、[GitHub Windows 客户端 runner](https://github.com/actions/runner-images/blob/main/images/windows/Windows11-Arm64-Readme.md)。
