# Windows MSIX 分发与 OCR 验收

Windows 系统 OCR 使用完整 MSIX 的包身份。安装后从开始菜单启动 Lenz;登录启动使用同一包内的 `LenzStartup`。解包或复制 `lenz.exe` 到包外运行不属于支持的 OCR 分发方式。

当前没有正式代码签名证书。v0.1.0 按发布安排先分发自签测试包,Windows 安装/OCR 实测推迟;正式可信签名仍待完成,见 [TODO](./TODO.md)。

`tauri.windows.conf.json` 禁用 Windows 默认 MSI/NSIS bundler;普通 `pnpm tauri build` 只构建 exe。Windows 分发通过下方 MSIX 入口,macOS 默认打包配置不受此覆盖文件影响。

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

保持安装助手文件在同一目录,运行:

```powershell
powershell -NoProfile -File .\Install-Lenz.ps1 -PackagePath .\Lenz-0.1.0.0-x64.msix -Launch
```

助手通过标准 `Add-AppxPackage` 让 Windows 验证包签名/内容/Publisher/证书信任并安装,再检查 WebView2,缺少时下载官方 bootstrapper 并验证 Microsoft 签名后安装。默认从已安装的 AUMID 启动 OCR 诊断,成功后可启动应用。助手没有允许 unsigned 或注册散装 manifest 的旁路,不代替测试证书的手动信任步骤。

诊断要求实际 `packageIdentity` 等于安装记录的 `PackageFullName`,并识别合成图片中的 `LENZ`、`12345` 和至少一个带 bbox 的文本块。报告写到临时目录,路径会打印出来;也可用 `-OcrReportPath <新路径>` 指定。诊断不初始化 GUI、用户配置或翻译接口,不截图、不读取 API Key。

OCR 失败会让安装助手返回错误,已安装的包保留。可在「设置 → 时间和语言 → 语言」补齐 OCR 语言能力,从开始菜单打开 Lenz 的 OCR 页刷新语言列表,然后重新运行:

```powershell
powershell -NoProfile -File .\Test-InstalledOcr.ps1
```

诊断优先选择英语并允许系统语言回退;CI 会准备英语 OCR 能力。若已有语言识别器无法读出合成英语图,安装英语 OCR 能力后重试。`-SkipOcrCheck` 只能用于排查安装步骤,不能作为识别验收通过的依据。

双击 MSIX 也可安装;较新的 Microsoft App Installer 会处理清单中的 WebView2 ExternalDependency。旧版 App Installer 或直接 `Add-AppxPackage` 不保证处理此依赖,因此推荐上述助手。双击安装后仍需单独运行识别验收。

## 3. 正式签名

正式分发需公开可信的代码签名证书或 Microsoft Store 签名。首次正式发布前确定并固定 `src-tauri/msix/identity.json` 的 Name/Publisher;Publisher 必须与签名证书 Subject 完全一致。保持后续升级的包身份,不要用测试 Publisher 代替正式身份。

证书已在当前用户的个人证书库且包含私钥时:

```powershell
powershell -NoProfile -File scripts/windows/Build-Msix.ps1 -CertificateThumbprint <证书指纹> -TimestampUrl <证书提供商的RFC3161地址>
```

构建会使用该证书的 Subject,MSVC 静态 CRT和 WebView2 Loader,拒绝缺失 DLL、外部 VC++ runtime 或架构不符的输入。支持 `-Architecture arm64`,原生 ARM64 包尚待 Windows 验收。已有 exe 可用 `-BinaryPath`;动态 Loader 必须提供匹配的 `-LoaderPath`。`-PackageVersion` 可显式指定四段整数版本,预发布 semver 不能直接当作 MSIX 版本。`-Unsigned` 只供内容检查,不会产出可正常安装的正式包。

## 4. CI 与发布前验收

[Windows MSIX 工作流](../.github/workflows/windows-msix.yml)在 Windows Server runner 原生 MSVC 构建测试包,再交给 Windows 11 ARM 客户端安装 x64 包并运行真实系统 OCR。临时信任仅限 GitHub-hosted 一次性机器;专用包名为 `app.lenz.desktop.ci`,测试结束清理包和证书。产物只包含公开证书、安装包、脚本与实际识别报告,没有 PFX/私钥。

工作流仅手动触发;选中 `verify_installed_ocr` 才会运行客户端安装 job,默认只构建。v0.1.0 推迟此运行验收。客户端 job 需要仓库可用的 `windows-11-arm` runner;此测试路径覆盖 x64 模拟,Windows 10/x64 原生、ARM64 包、语言包缺失和登录启动后的身份仍单独验收。交叉编译或安装包构建成功不能替代这份 Windows 报告。

发布前还要验证:全新安装和升级、缺 WebView2 时安装、F2/F3 实际截图识别、中英 OCR 语言切换、无语言包的提示、设置自启后重新登录以及关闭自启。尚无正式证书时只提供测试包,不发布为正式可信安装版本。

官方依据:[OCR 包身份要求](https://learn.microsoft.com/en-us/uwp/api/windows.media.ocr)、[MSIX 证书](https://learn.microsoft.com/en-us/windows/msix/package/create-certificate-package-signing)、[WebView2 分发](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution)、[GitHub Windows 客户端 runner](https://github.com/actions/runner-images/blob/main/images/windows/Windows11-Arm64-Readme.md)。
