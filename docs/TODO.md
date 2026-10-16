# Lenz 开发 TODO List

> 基于 [PRD v0.3](./PRD.md) §6.1 里程碑 + [spec](./spec/) 各模块详细规格拆分的任务清单。
>
> 约定:
> - 完成标记为 `[x]`,跳过用 `[-]` 并注明原因。
> - 每项任务右侧 `(→ spec XX §Y)` 指向对应 spec 节。
> - P0/P1/P2 对应 PRD 优先级;风险项以 ⚠️ 标记。
> - 跨里程碑的通用约束集中写在本文档末尾的"持续执行"小节。
> - `[x]` 的范围以任务注释为准:代码实现、构建成功与实机验收分别记录;按日期保留的测试数量为当次结果。

---

## 2026-10-16 · Windows 安装后启动失败

- [x] 增加 Windows 原生 EXE 启动工作流;发布包在冷启动后的一次再次启动中复现 `state() called before manage() for SelectionListener` 并退出（运行 38870342117）。原因是自动创建的 WebView 在 setup 注入状态之前发送 IPC。
- [x] 通过启动插件在配置窗口创建前准备全部状态与事件,再使用 Tauri 原有的自动建窗流程。EXE 开始菜单/完成页增加 `--show-main`;托盘失败时显示主窗口。Windows 主线程致命错误写入独立启动日志并显示原生错误提示。
- [x] 修复 Windows 原生 NSIS 构建的源文件分隔符,增加失败时缓存及编译产物保留。macOS 本地 cargo check/fmt/Clippy、96 项 Rust 测试（10 项原生验收忽略）、41 项前端/打包测试、NSIS 3.13 警告视为错误的编译及 PowerShell 语法检查通过;启动修复提交的 Windows/macOS CI（38870808696）均通过。Rust 的 localhost mock 测试需在允许回环监听的环境运行。
- [x] [Windows Server 原生验收](https://github.com/huangj17/Lenz/actions/runs/38874364350)通过:普通 EXE 静默安装、默认后台启动完成初始化、连续三次 `--show-main` 显示主窗口且进程保持运行。最终应用构建来自 93e3474;检查脚本已修正运行中日志的共享读取,并支持复用已有安装包。安装器解包后与同次 MSVC 应用逐字节一致,无 MSIX/证书载荷。
- [x] 已替换 Release EXE 并更新校验值:3,996,423 字节,SHA256 `363f78ae1883432de58ef0b471eb3e7f8e16b4a20888a4199ce098250c5ceb69`。[云端发布与公开下载回验](https://github.com/huangj17/Lenz/actions/runs/38875389254)通过;MSIX/ZIP 和 macOS 附件保持原构建,五项安装附件校验值均已核对。
- [ ] Windows 10/11 用户机器启动及腾讯云真实识别仍需验收。

## 2026-10-16 · 普通 EXE 分发与系统 OCR 可选安装

- [x] 根据用户反馈把 EXE 从 MSIX 外壳改为普通 NSIS 当前用户安装包,无需导入 MSIX 测试证书;默认支持配置腾讯云等 OCR。仅调用 WebView2 安装助手,不执行 MSIX 部署/证书导入/OCR 自检(→ PRD §5.3、spec 03 §4.2、spec 11 §3.1/4.2)。
- [x] 增加普通 EXE 的显式载荷暂存及 PE 架构/静态运行时/动态 Loader 校验;保留独立 MSIX 分发。构建入口改为 `pnpm windows:exe [-BinaryPath <lenz.exe>]`,发布工作流复用同一 MSVC 可执行文件。
- [x] 增加开始菜单入口、当前用户卸载项和卸载器;退出运行中的应用后升级/卸载,清理自身文件与匹配路径的自启项,保留配置。完成页提供以后需要系统 OCR 时的 MSIX 下载入口;明确仅导入证书不能给普通 EXE 补身份。
- [x] NSIS 3.13 实际生成普通安装器（警告视为错误）,解包确认含原发布 MSVC 应用、卸载器和 WebView2 助手,无 MSIX/证书/身份清单/安装 OCR 助手。41 项前端/打包测试、9 项 MSIX 安装回归、typecheck/build、PowerShell 语法与 diff 检查通过。
- [x] 已替换 v0.1.2 的 EXE 附件并同步 SHA256SUMS/发布说明。普通 EXE 为 3,995,380 字节,SHA256 `84076527d4813a202b946a21890c679389a3d0cc4fa0c8bded22210373b20144`;下载回验与本地产物逐字节一致,五项安装附件校验全部匹配。MSIX/ZIP 和双架构 DMG 未变。
- [x] README 分行保留普通 EXE 与 MSIX + 安装助手 ZIP 两种下载方式,分别说明腾讯云 OCR 和可选系统 OCR 的安装条件。构建脚本、工作流、测试、中英文提示和安装说明一并纳入本轮提交;发布说明指向主分支上的最新 Windows 分发指南。
- [ ] Windows 原生安装/运行/升级/卸载、无语言包时腾讯云真实识别,以及切换 MSIX 后的配置和自启仍待实测。

## 2026-10-16 · 本地 OCR 与安装解耦

- [x] 默认仅安装应用和必需的 WebView2,不下载或要求本地 OCR 语言包、不执行 OCR 自检。EXE 显式跳过自检,安装后可配置腾讯云等服务;系统 OCR 能力和语言包以后从设置页添加(→ PRD F5、spec 03 §4.2、spec 11 §4.2)。
- [x] 安装助手新增可选 `-CheckOcr`,失败只警告,保留安装成功及启动;严格 Windows CI/独立诊断仍要求真实识别通过。兼容旧 `-SkipOcrCheck`,拒绝互斥选项和无自检的报告路径。
- [x] 根据用户截图明确 `0x800B0109` 为应用签名证书不受信任,发生在 OCR 前。助手返回独立退出码 10;EXE 使用中英文专门提示并可重新打开证书查看器后重试,不自动修改信任。
- [x] 9 项安装流程回归通过,覆盖默认/旧跳过参数不调用 OCR、显式自检、缺本地 OCR 时继续启动、证书错误退出码 10、部署/WebView2 真失败和互斥参数。Windows API 均为隔离桩,不计原生验收;CI 已接入脚本。39 项前端/打包测试、typecheck/build、PowerShell 语法及 diff 检查通过。
- [x] 用 NSIS 3.13 重新编译附证书/不附证书两种 EXE（警告视为错误）;解包核对新安装助手与源码一致、EXE/ZIP 中 MSIX/公开证书一致且原签名包 SHA256 不变。
- [x] 已替换 v0.1.2 的 EXE/ZIP 并同步 SHA256SUMS/发布说明;下载回验与本地产物逐字节一致,五项 SHA256 全部匹配。EXE 为 6,228,273 字节/SHA256 `639e3b8b82e4c153be73a48b72717b0414f5a9d7b103bf49b97b24c6aac2f27a`,ZIP 为 6,117,502 字节/SHA256 `3fe3633ab8fcb9ced4c3783174b7a8f12a13ccaeff4e6886d2792716ccfa86fa`;签名 MSIX 与 macOS 两份 DMG 未变。源码改动未提交/推送。
- [x] 中英文系统 OCR 提示在源码中说明语言包可选;本次复用已发布 MSIX,未重编译应用本体,该界面文案随后续应用版本发布。
- [ ] Windows 原生证书恢复安装、无本地语言包的安装/启动及腾讯云真实识别待实测。

## 2026-10-16 · Windows 双安装入口

- [x] 增加 NSIS EXE 安装向导,封装同一次构建的 MSIX/安装助手及可选公开测试证书;保留原 MSIX 和 ZIP。双入口共用包身份、升级、开始菜单及卸载,不自动信任证书,不绕过 OCR 自检(→ PRD §5.3、spec 11 §3.1/4.2/4.3)。
- [x] 标签发布和手动 Windows 构建接入 EXE;收集器仅发布规范命名的 setup EXE 并生成 SHA256,排除裸应用 exe。同步 Windows 分发指南和开发命令。
- [x] macOS 上用 NSIS 3.13 实际编译测试证书版/无证书页版（编译警告视为错误）;PowerShell 全部脚本语法、39 项前端/打包纯逻辑测试、typecheck/build、diff 检查通过。7-Zip 解包核对内嵌 MSIX、公开证书和三份助手与 v0.1.2 原发布载荷逐字节一致,无私钥。
- [x] 为 [v0.1.2](https://github.com/huangj17/Lenz/releases/tag/v0.1.2) 补充 `Lenz_0.1.2_windows_x64_setup.exe`（6,227,010 字节,SHA256 `ce050ca44396e2faf5bbf866458986daf7109e7e4dd39bd66c56e20140b2ce94`）,更新 SHA256SUMS 和发布说明;原四个安装附件及校验值保留;重新下载 EXE/清单后逐字节比对通过,清单五项 SHA256 全部匹配。构建/工作流源码与文档修改保留在工作区,未提交或推送。
- [ ] Windows 原生 EXE 向导安装/证书引导/OCR/升级/卸载仍待实测;macOS 编译或脚本检查不计为 Windows 运行验收。

## 2026-10-15 · v0.1.2 发布验收

- [x] 整合腾讯云 OCR、macOS 划词取词/选区定位修复、权限提示和渐变应用图标;应用版本同步为 0.1.2,发布说明见 `docs/releases/v0.1.2.md`。
- [x] 本地前端 typecheck/build、39 项纯逻辑测试、Rust fmt/check、全量测试(96 过、10 项环境测试忽略)及全目标 Clippy(`-D warnings`)通过。腾讯云原生设置页验证与限制见服务 OCR 记录及发布说明。
- [x] 发布前 Windows CI 发现划词光标锚点分支的冗余 `return`,已修正;本地 macOS/Windows GNU 全目标 Clippy 通过。提交 `59df689` 的 [macOS/Windows CI](https://github.com/huangj17/Lenz/actions/runs/38707228945) 全部通过,Windows 测试仅编译,不计作运行验收。
- [x] v0.1.2 标签指向 `59df689`,已推送并触发[安装包发布流程](https://github.com/huangj17/Lenz/actions/runs/38707793673)。
- [x] [v0.1.2](https://github.com/huangj17/Lenz/releases/tag/v0.1.2) 已正式发布并标记 Latest;三个平台构建和发布任务全部成功。已下载两份 DMG、Windows MSIX/安装助手 ZIP 与 SHA256SUMS,四个安装附件的 SHA256 全部匹配。
- [x] 下载包核验:Windows MSIX 为 `app.lenz.desktop` / `0.1.2.0` / x64,包含签名;ZIP 内 MSIX 与独立附件逐字节校验一致,包含公开 `.cer` 与安装助手,不含常见私钥文件。此检查不等于 Windows 安装/OCR/自启运行验收。

## 2026-10-15 · macOS 权限名称说明修正

- [x] 核对本机系统资源 `SecurityPrivacyExtension.appex/Contents/Resources/Localizable.loctable`: `ACCESSIBILITY` 的中英文显示名为“设备控制和数据访问” / “Device Control and Data Access”。保留已有 `Privacy_Accessibility` 深链,设置与弹窗文案同时说明新旧名称。
- [x] 已开启仍检测不到授权时提示退出并重新添加当前安装的 Lenz;不修改系统权限或绕过 `AXIsProcessTrusted` 检查。签名/安装副本导致旧授权不匹配属于待确认原因,不凭设置截图断言。
- [x] `pnpm typecheck`、`pnpm build` 与 `git diff --check` 通过;本轮仅改提示与相关说明,未重新打包,当前已安装测试包不会随源码文案更新。

## 2026-10-15 · 应用图标底色

- [x] 桌面图标改为冰蓝至浅蓝紫渐变圆角底板,保留 L 与「文 / A」双气泡;生成脚本复用透明 SVG,窗口内品牌标记与托盘模板保持各自用途(→ PRD §4.4、spec 09 §2.1)。
- [x] 重新生成 1024px 源 PNG 与 17 份桌面 PNG/ICNS/ICO/MSIX 图标;检查 256px 预览、ICNS 多尺寸解包和 ICO 格式。`pnpm typecheck`、`pnpm build` 通过(沿用单次 `pnpm_config_verify_deps_before_run=false`,避免 pnpm 自动重装现有依赖)。
- [ ] 本轮仅更新图标资产,未重新打包或替换已安装应用;macOS Finder/Dock 与 Windows 安装后外观待验收。

## 2026-10-15 · macOS v0.1.1 快捷键取词与选区定位修复

- [x] 修正 macOS 剪贴板兜底要求版本恰好 +1 导致多格式复制被误判失败的问题,等待版本稳定后校验输入/前台来源;修饰键未松开时停止合成复制。
- [x] AX 读取绑定实际前台应用并取得选区边界;取词失败显示原因及辅助功能设置入口,不再回显无关旧结果。同原文/目标/引擎缓存仍可复用。
- [x] 弹窗默认置于选区下方,空间不足放上方;内容 resize 沿用锚点及来源屏幕工作区,手动拖动后尊重用户位置。无 AX 边界回退触发瞬间鼠标位置,图标缓存携带边界。
- [x] 前端 typecheck/build、37 项测试、Rust fmt/check、87 项测试通过(10 项环境测试默认忽略);bindings 由导出测试重新生成。新增 3 项定位回归,覆盖增长/缩回、手动拖动、负坐标/缩放和非法 AX 边界;剪贴板回归覆盖多次版本写入。
- [x] 已构建本机 Apple Silicon 修复测试包 `src-tauri/target/release/bundle/dmg/Lenz_0.1.1_selection-fix_aarch64.dmg`;应用重新做本地 ad-hoc 签名并通过严格校验,未覆盖已安装应用或发布 Release。
- [ ] 2 项 TextEdit 原生选区/剪贴板恢复测试已补充并尝试执行,但自动化过程中系统前台切换,固定文稿前置条件未满足,不能计作通过。浏览器本地验证页被工具 URL 策略拦截,未绕过;真实浏览器取词、原生弹窗、多屏缩放与安装后验收仍待完成。

## 2026-10-15 · v0.1.1 发布准备

- [x] 应用版本同步为 0.1.1(package.json / Cargo.toml / Cargo.lock / tauri.conf.json),新增 `docs/releases/v0.1.1.md`,更新 README 下载入口与功能/验收范围。
- [x] 发版前本地验证:版本契约、前端 typecheck/build/37 项测试、Rust fmt/全目标 Clippy(`-D warnings`)/84 项测试(8 项依赖环境忽略)及 Windows GNU 全目标 check 通过。本机 pnpm 的运行前检查尝试重装现有依赖且被镜像网络/无 TTY 阻止;后续使用单次 `pnpm_config_verify_deps_before_run=false` 执行已有依赖检查并通过,未修改依赖版本或全局配置。

## 2026-10-15 · 按服务折叠与延迟翻译

- [x] 弹窗每行标题支持折叠/展开,设置菜单按服务分别保存“记住折叠状态”和“折叠时不自动翻译”;未记住时新查询展开,折叠跳过时展开才请求。已完成结果保留供展开复用(→ spec 09 §3.2、spec 04 §2.1)。
- [x] 同会话新增独立 requestId 与取消命令,折叠只取消对应服务;旧结果不能覆盖展开后的新请求。原生主服务也遵守折叠策略,全部跳过时不发起自动请求,不修改覆盖翻译的全局默认服务。
- [x] 配置 v6→v7 增加按服务偏好,兼容缺失字段并保留已有配置与凭据;重新生成 bindings,同步 PRD、spec 01/04/05/06/09。
- [x] `pnpm typecheck/build/test`(37 项)、Rust fmt/check/test(84 项通过、8 项依赖环境忽略)通过。新增 6 项前端折叠/取消回归、3 项 Rust 请求取消/主服务选择回归、2 项迁移回归。完整 Rust 测试的 3 项既有 OCR localhost 用例在沙箱内因端口权限失败,移至沙箱外后全部通过。
- [x] Playwright 隔离 IPC 验证独立偏好、取消准确请求、展开补译/结果复用、新查询/重载恢复、连续保存含前次失败、失败回滚、菜单键盘导航/Escape、300px 英文深色布局;截图 `output/playwright/popup-collapse-{light,narrow-dark,save-error,final}.png`。
- [x] 独立 macOS 原生 popup 验证:默认 Index 折叠跳过时只请求 Google localhost;展开 Index 后单项调用并返回结果,折叠 Google 时 localhost 确认连接取消且 Index 结果保留。偏好实际写入 v7 配置,重启后三项均折叠且无自动请求;Apple 展开返回语言包安装指引。仅使用固定 Hello 和本地服务,不计为线上服务验收。独立进程/配置已清理、临时 QA 入口已移除、默认开发构建已恢复。
- [x] 根据实测反馈修正服务菜单:新增真正的齿轮图标、删除冗余的展开/折叠菜单项;点击服务标题切换折叠时立即关闭菜单。移除旧文案并同步键盘初始焦点。typecheck/build/37 项前端测试通过;Playwright 验证收起/展开均关闭菜单、仅保留两项偏好、Escape/键盘导航和中英深浅/300px 布局,截图 `output/playwright/popup-menu-fix-{light,dark}.png`。本次原生窗口自动化未能定位开发进程,未重复计为原生实测。
- [ ] Windows 原生折叠菜单、真实快捷键/框选 OCR 与折叠结合的整链仍待实测。

## 2026-10-15 · 弹窗多翻译服务对比

- [x] 修复“启用三个服务、弹窗只显示默认服务”:F1/F2 原生主服务与前端额外服务共用会话,按设置顺序并行请求已启用服务,独立显示加载/结果/耗时/错误/复制。运行态回退时报告实际主引擎,避免重复请求;禁用服务不展示,缺配置的启用服务保留错误行(→ spec 04 §2.1)。
- [x] 单项重试不重复请求成功项;编辑原文、切换/互换语言重译全部服务。缓存主服务回显时查询其余当前启用服务。长结果在共享列表滚动,语言条和服务数量/Esc 固定可见。关闭/新请求取消全部旧请求,迟到事件与响应不能覆盖当前会话。
- [x] 新增 8 项前端回归和 1 项 Rust 多请求取消回归;前端 typecheck/build/31 项测试、Rust fmt/check/79 项测试(8 环境验收测试默认忽略)通过,默认开发构建已恢复。PRD、spec 04/05/06/09 同步。
- [x] Playwright 生产预览(隔离 IPC fixture)验证三服务、禁用项过滤、乱序结果、局部失败/重试/复制、手动重译、旧会话隔离、Esc、缓存回显、300px 英文深色长结果滚动,以及额外服务识别语言后互换并请求全部服务。截图见 `output/playwright/popup-multi-{partial,error,complete,final,narrow-dark}.png`。
- [x] 独立 macOS `app.lenz.multiqa.20261015` 原生 popup 同时展示三行:Apple 真实翻译 `Hello → 你好`(148ms)、Index localhost 固定结果(818ms)、Google localhost 模拟 503 的独立错误行。此验收不计作 Google/Index 线上服务通过。测试构建首次隔离标识未生效即停止,后续校验唯一标识后重建;最终 QA 已退出、独立配置已清理、临时入口已移除,未授予新系统权限。
- [ ] Windows 原生、多服务模式下的真实框选/OCR 整链、Google/Index 线上并发仍待实测;原生单项重试点击被工具激活主窗口后失焦隐藏打断,该项仅浏览器验收通过。

## 2026-10-15 · 苹果翻译测试超时修复

- [x] 复现英中语言包未安装时,系统下载授权阶段一直等待直到请求超时;对照确认翻译任务已执行,不能仅靠延长超时解决。改为 `LanguageAvailability` 预检查,缺包立即返回安装指引,不弹出持续转圈的 helper 窗口(→ spec 04 §6.2)。
- [x] 苹果服务面板提供系统语言包设置入口,提示同时安装源/目标语言且测试源语言为英语;缺包和超时使用中英文提示,所有连接测试错误完整换行显示。PRD 与 spec 04 同步。
- [x] 本机 macOS 27.0.1 在系统设置下载英语(美国)/简体中文后,真实 helper `Hello → 你好` 约 0.66s,三条批量返回 `你好 / 早上好 / 谢谢` 约 0.10s;未安装法语的请求约 0.02s 返回安装错误。同语言批量与自动源语言检测通过。
- [x] 新增 3 项 Rust 原生回归:同语言批量默认运行;已安装英中翻译及未安装法语错误依赖本机语言包状态,默认忽略但本轮显式运行并通过。`pnpm typecheck`、`pnpm build`、Rust fmt/check 通过。
- [x] Playwright 生产预览(隔离 IPC fixture)验证缺包/超时提示、打开系统设置的 IPC、中英与深浅主题四种组合、720×480 错误完整换行;截图见 `output/playwright/apple-languages-*.png`。这部分为界面验收,真实翻译由上述原生 helper 与 Rust 测试验证。
- [ ] macOS 15 SwiftUI 已安装语言对宿主、Intel 原生运行仍待实测;本轮未验证 Windows(该引擎在 Windows 不可用)。

## 2026-10-15 · OCR 服务管理

- [x] 系统 OCR、百度标准/高精度含位置版和 OpenAI Chat Completions 兼容视觉接口接入;服务按请求配置快照切换,无需重启。截图翻译使用默认服务;视觉模型仅返回完整原文,原图覆盖回退系统 OCR,百度坐标复用屏幕坐标转换(→ spec 03 §2.2–2.4)。
- [x] OCR 设置页支持添加、编辑、设默认、删除、语言/超时和内置样例测试;固定保留系统服务,删除默认第三方服务后恢复系统。新增中英文文案、上传提示、真实识别结果/耗时与本地化错误;编辑/切换后忽略旧测试结果。服务搜索和系统偏好搜索均可跳转。
- [x] 配置 v5→v6 迁移保留现有语言顺序、精度、自动检测、置信度及翻译凭据;重新生成 bindings,保持原子持久化与 config 事件同步(→ spec 01 §2、§7)。
- [x] Rust 回归测试 77 通过/6 环境验收测试忽略;包含真实 localhost HTTP 协议测试、图片请求、百度鉴权/表单、负坐标与缩放、错误脱敏、配置迁移及视觉模型覆盖回退。`cargo check`、格式检查、`pnpm typecheck`、`pnpm build` 和前端 23 项测试通过。
- [x] Playwright 最终生产预览(隔离 IPC fixture)验证两类第三方服务增改、保存/重载、默认切换、测试通过/失败、编辑后旧结果失效、保存延迟期间修改草稿不启动过期测试、删除默认回退、系统不可删除、设置搜索及 720×480 无横向溢出。界面截图: `output/playwright/ocr-services-light.png`。
- [x] macOS 独立 `app.lenz.ocrqa` 原生窗口验证系统样例识别(首次 29767ms、再次 106ms),真实 Rust HTTP→本机固定视觉响应(37ms、重启后 8ms)、默认切换、重启后 URL/模型/默认服务与英文深色主题持久化。未修改生产配置或授予新系统权限。
- [ ] 百度线上凭据/额度、真实视觉模型识别质量、切换新服务后的真实框选→OCR→翻译整链及 Windows 原生窗口仍待验收;localhost 固定响应和浏览器 fixture 不计作线上识别通过。

## 2026-10-15 · 原图翻译即时反馈与 loading

- [x] F3 在截图完成后立即创建可交互蒙层,与 OCR/翻译并行;选区截图始终保留,工具条按阶段显示“识别中/翻译中”及旋转 loading,完成后同窗覆盖译文。先抓屏再显示,避免把新工具条或 loading 抓进 OCR 输入。
- [x] 增加 typed `OverlayProgress` 与定向 `overlay://updated` 事件;截图只在初始载荷传输。先订阅再读快照,按 generation/revision 合并,防止进度丢失或旧快照回退。首次 DOM 就绪显示/聚焦,更新阶段不反复抢焦点。
- [x] 处理中 × / Esc 始终可取消;旧会话和关闭后的结果无法写回。失败在当前蒙层提示并停止 loading,截图前的失败仍回主窗口。工具条/错误提示使用实色主题底,保证任意桌面背景上的可读性。
- [x] 新增 2 项前端回归与 3 项 Rust 回归:快照/事件竞态、卸载清理、慢请求保留截图/阶段、处理中取消及迟到结果、新选区隔离。前端 typecheck/build/23 项测试、Rust fmt/check/69 项测试(6 ignored)/全目标 clippy(`-D warnings`)、Windows GNU 全目标 check 通过。
- [x] macOS QA 原生固定载荷/事件验证“识别中 → 翻译中 → 原位译文”、完成后 ×、翻译中 Esc、识别中 ×、OCR 超时提示与关闭;关闭后返回主窗口并确认载荷清空。截图见 `output/native-e2e/overlay-progress-{ocr,translate,complete,error}.png`。此验收不调用屏幕录制、实际 OCR 或远端服务,不作为完整业务链路/首次反馈延迟的实测。
- [x] QA 已退出,配置 SHA256 与验收前一致;未变更系统权限或生产配置,默认开发构建已恢复。
- [x] PRD、spec 07/09 同步显示时机、状态协议、取消行为及首次反馈/译文完成两个独立性能目标。
- [ ] 新显示流程下的真实框选 → OCR → 远端翻译完整重跑与 ≤1s 首次反馈测量、Windows 原生窗口交互仍待实测。

---

## 2026-10-14 · 覆盖蒙层与 Esc 卡死修复

- [x] 采样正在卡死的实际开发进程:macOS 主线程停在全局 Esc 回调 → `overlay::close` → `unregister` → shortcuts mutex。插件在持锁期间调用回调,注销自身导致重入死锁,菜单栏因此一起失去响应。
- [x] 按用户要求改为可聚焦、可点击的翻译蒙层:保留选区截图,周围变暗,工具条提供 × 与 Esc 提示;删除临时全局 Escape 注册/注销及鼠标穿透。
- [x] × 与窗口内 Esc 共用异步关闭入口,取消会话、清空截图/译文载荷并销毁窗口;原生关闭请求同样清理。菜单栏图标/打开设置先关闭蒙层再聚焦主窗口。
- [x] 新增 `viewport` 与 `imageDataUrl` IPC 字段并生成 bindings;来源显示器选择、负坐标回映射及工具条屏内夹紧有回归覆盖,包括屏幕布局变化后选区已离屏的情况。
- [x] macOS QA 原生交互验收(固定图像和译文,未重新运行 OCR/远端服务):深色中文 ×、深色中文 Esc、浅色中文真实鼠标 ×、浅色英文点击译文后 Esc 均关闭成功;关闭后可通过原生 Cmd+, 打开主窗口,并确认 ×/Esc 后 `get_overlay_payload` 返回 null。截图见 `output/native-e2e/overlay-modal-{dark,light}.png`。
- [x] 前端 typecheck/build/21 项测试,Rust fmt/66 项测试(6 ignored)/全目标 clippy(`-D warnings`)、Windows GNU 全目标 check 通过。默认开发构建已恢复,QA 已退出,QA 配置按原始字节和 SHA256 恢复;本轮未启用屏幕录制权限。
- [x] PRD、spec 07/09 与规格入口同步新的蒙层交互;保留此前超时修复记录作为历史验收范围。
- [ ] 最后补入的“打开设置先关蒙层”联动、实际托盘点击及系统原生关闭请求未完成单独实测;末轮 QA 调试操作自动审批超时。默认 Cmd+W 没有触发窗口关闭,不将其列为已通过入口。Windows 实机及新蒙层下的完整 OCR/远端翻译重跑仍待验收。

---

## 2026-10-14 · 截图/OCR 超时修复

- [x] 根因复现:640×220 合成文字图 Vision 首次识别 25.9s,重复运行 0.44s;不依赖截屏权限或翻译服务,原 10s 总预算会误报失败。
- [x] F2/F3 与命令入口统一截图/OCR 阶段超时:截图独立 10s,Vision 按语言/模式配置首次成功前 60s、成功后 10s;popup 握手不扣减 OCR 预算。
- [x] Vision 原生调用串行化,外层取消后并发槽持续持有到原生结束;日志记录阶段预算、尺寸和耗时,不记录截图或识别文字。
- [x] 新增慢初始化、持续阻塞、语言/模式切换与取消后串行化回归测试;主窗口/弹窗提供中英阶段超时提示。
- [x] 本地自动化:前端 typecheck/build/20 项测试,Rust fmt/check/全目标 clippy(`-D warnings`)、完整测试 65 通过、6 项原生服务/本地模型测试默认忽略;bindings 导出无契约变化。Windows GNU 全目标 check 通过。
- [x] 修复后独立运行真实 Vision 合成图测试:中英双语言首次 25.4s 成功返回;系统缓存就绪后新引擎约 0.56s、同引擎再次约 29ms,并断言首次成功后恢复 10s 预算。
- [x] Playwright 隔离 IPC fixture 检查 macOS/Windows 页面分支、中英深浅主题的主窗口提示及中英弹窗提示,20 项断言通过;截图无溢出。该验证不代替原生窗口验收。
- [x] GUI 原生诊断:默认 Vision 加速路径长时间卡在 TextRecognition 同步等待;改为显式 CPU 后真实截图 OCR 57–176ms 完成。CPU 合成图首次 29.5s、同引擎再次 45ms,首次预算仍保留。
- [x] 共享 HTTP client 添加 `Lenz/{版本}` User-Agent;QA 应用真实 Index-Translate 连接测试 299ms 返回中文。原无标识请求曾超时,独立请求曾返回 HTTP 412。
- [x] 修复隐藏 WebView 等待动画帧导致覆盖层永不显示,改为 DOM 提交后直接通知 ready;macOS 先设置尺寸再定位,修复本机 90pt 纵向偏移。
- [x] macOS QA 原生整链验收:调用真实 `start_selection` 后鼠标拖动框选固定测试页,700×390 截图 92ms、中英快速 OCR 79ms/4 块,真实 Index-Translate 中文译文显示。窗口可见,负坐标外接屏 1× 的位置/尺寸与载荷完全一致;下层按钮由 1→2 验证鼠标穿透。原生覆盖截图保存于 `output/native-e2e/overlay.png`。
- [x] 最终前端 typecheck/build/20 项测试、Rust fmt/65 项测试(6 ignored)/全目标 clippy 通过。验收结束已退出 QA、撤销并移除临时屏幕录制授权,QA 配置按原始字节与 SHA256 恢复。
- [ ] 真实远端连续翻译稳定性:第二次框选截图 76ms/OCR 56ms 正常,远端翻译未及时返回,已取消;不能把连接测试与首次整链成功视为连续服务稳定性通过。未完成本地固定响应的重复整链补验。
- [ ] F2 原生弹窗整链、2×/跨屏缩放与 Windows 原生运行仍未实测。自动化按键没有触发 macOS 全局快捷键,F3 快捷键及覆盖层全局 Escape 不算通过;选区入口使用真实 IPC。

---

## 2026-10-14 · v0.1.0 安装包发布

- [x] 标签发布工作流:版本一致性检查、macOS arm64/x64 DMG、Windows x64 MSIX + 安装助手 ZIP、SHA256 与 GitHub Release 上传。
- [x] Xcode 26+ SDK 选择、macOS 12 最低版本及 ad-hoc 签名;Windows 365 天自签测试证书,不导出私钥、不自动修改信任。
- [x] 发布说明写明签名状态和未验收范围;前端 typecheck/build/20 项测试(含产物嵌套目录回归)、版本错配拒绝、工作流 YAML 和 PowerShell 解析通过,改动文件凭据扫描无匹配。
- [x] v0.1.0 标签与发布提交 cc93d55 已推送,触发 [Release installers](https://github.com/huangj17/Lenz/actions/runs/38529429284)。
- [x] Apple Silicon DMG 与 Windows x64 MSIX 的 GitHub 原生构建成功;Windows MakeAppx/SignTool 链路通过。
- [x] Intel DMG 原生构建成功;三份安装包齐全。Apple Silicon DMG 完整性/ad-hoc 签名/版本/架构/最低系统检查通过,Windows ZIP 内公开证书和安装助手齐全且无私钥。
- [x] [v0.1.0 Release](https://github.com/huangj17/Lenz/releases/tag/v0.1.0) 资产上传完成;首轮上传因 Windows 产物嵌套目录失败,已修复收集路径并通过 [Publish built installers](https://github.com/huangj17/Lenz/actions/runs/38532660951) 重用原标签的三份成功构建产物。
- [x] 下载产物与公开 `SHA256SUMS.txt` 的四项校验值全部匹配;两份 DMG 签名/版本/架构/最低系统与镜像完整性检查通过。`releases/latest` 返回 v0.1.0。
- [x] 修复 Rust 1.99 CI 的 async_trait 宏 double_must_use 误报(仅限制到两个 trait)及固定大小切片 lint;本地 macOS/Windows GNU clippy 通过。
- [x] Rust 1.99 的双平台 Clippy 修复已通过实际 CI;macOS Rust 测试通过。
- [x] 发布后主分支 [CI](https://github.com/huangj17/Lenz/actions/runs/38533802391)(33459fc)双平台通过:前端 typecheck/20 项测试/build、Rust fmt/全目标 Clippy;macOS 执行 Rust 测试,Windows `cargo test --no-run` 编译测试。
- [ ] Windows Rust 测试运行:先前 Windows Server 上测试 exe 启动返回 `0xc0000139`(STATUS_ENTRYPOINT_NOT_FOUND),根因未确认,需 Windows 客户端检查。当前 CI 仅编译测试,不把先前运行失败记为通过。
- [ ] Windows 安装/OCR/自启运行验收按用户要求推迟;验收工作流改为手动,默认不运行安装识别。
- [ ] Developer ID 公证与 Windows 公开可信签名后续补齐;本次发布明确使用测试签名。

---

## 2026-10-14 · 设置功能补齐与双平台适配

- [x] 更新检查:启动延迟/24h 轮询/重新开启/手动检查,semver 比较、结果提示和官方 Release 入口;无正式发布不误报最新。
- [x] OCR:快速/精准、自动检测、置信度真实绑定,语言添加/删除/拖动/箭头排序,配置 v5 迁移;Windows 按系统能力展示,提供语言包安装与刷新。
- [x] 引擎超时输入,真实测试状态;Apple Translation 原生桥接(macOS 15+),Windows/低版本禁用此系统专属引擎。
- [x] F3:独立取消会话、截图/OCR/批量及逐块回退翻译、颜色采样、字体适应、鼠标穿透、Esc/托盘关闭及显示握手。
- [x] Windows 划词:鼠标 hook + UIA TextPattern,保留剪贴板兜底,源窗口焦点及 DPI 定位;macOS 设置授权引导和实际运行状态。
- [x] 设置搜索和聚焦跳转,真实版本/构建日期/Bundle ID,GitHub/更新日志/许可证入口;托盘跟随界面语言,平台快捷键提示。
- [x] IPC 类型重新生成;PRD、spec 01/03/04/05/07/09/10、开发依赖说明同步。
- [x] 自动化:前端 typecheck/build、15 项测试;macOS fmt/check/clippy(`-D warnings`)、61 项 Rust 测试(6 项依赖原生服务/本地模型而忽略);Windows GNU 全目标 check/clippy(`-D warnings`)。
- [x] Playwright 生产预览验证搜索、配置提交/重载、OCR 编辑、引擎超时/测试、更新/外链/许可证及 Windows Ctrl+,。使用隔离 IPC fixture,不代替原生系统验收。
- [x] Apple helper 同语言 JSON stdin/stdout 协议测试,不需要下载语言包。
- [x] Windows GNU 优化构建完整链接成功,生成 PE32+ x86-64 GUI `lenz.exe`;全目标 check/clippy 通过。
- [x] macOS/Windows 页面分支的中英深浅与 720×480 最小窗口布局检查;覆盖层文本和 ready 握手检查（隔离 IPC fixture）。
- [x] 修复 Tailwind dark 样式与手动主题类不同步,深色分段选中背景使用相同 `.dark` selector。
- [x] macOS 独立 QA 应用原生验证:实际版本/权限/OCR 语言、许可证资源、更新结果、Cmd+, 恢复隐藏窗口、搜索聚焦和超时保存;应用重启后深色主题/OCR 模式/自动检测/语言顺序保留。未授予系统权限,未修改生产配置。
- [ ] macOS 原生辅助功能/屏幕录制、Apple 跨语言翻译和语言包下载授权验收;Hello → 中文探测 20s 未返回,已取消,不能记为通过。
- [ ] Windows 实机 OCR/语言包、UIA 源应用覆盖、Ctrl+, 和托盘按键、自启重启验收。
- [x] Windows OCR 包身份与分发实现:完整 MSIX、开始菜单和包内 StartupTask;缺身份/缺语言提示独立处理,安装后运行生产 OCR 的合成图片诊断。测试签名与 Windows CI 已编写,实际安装结果见下方未完成项(→ spec 03 §4.2、spec 11 §4.2)。
- [ ] 双平台 F3 混合 DPI/负坐标屏幕、鼠标穿透、Esc 冲突和性能验收。
- [-] Windows GNU 调试完整链接超出 DLL 导出序号上限;发布使用 MSVC 原生构建,已成功生成 v0.1.0 MSIX。Windows 安装运行验收仍待完成。

---

## 2026-10-14 · Windows OCR 包身份与分发

- [x] `GetCurrentPackageFullName` 实际进程身份检测;`OcrIdentityRequired` / `ocr://package-required` 与缺语言包错误分开,设置页和截图弹窗中英文引导同步。
- [x] 完整 MSIX 清单:full-trust 入口、图标、包名/Publisher/四段版本及 `LenzStartup`;包内登录启动保持身份,不绕过用户/策略禁用。Windows 平台配置关闭默认 MSI/NSIS 分发,统一走 MSIX 构建入口。
- [x] MSVC 无 bundle + 静态 CRT 构建、MakeAppx/SignTool 入口、Publisher/证书匹配、可选 RFC3161 时间戳;PE 架构/运行时依赖检查;已有动态 exe 要求匹配 WebView2Loader。
- [x] WebView2 ExternalDependency + PowerShell runtime 检查/官方 bootstrapper/Microsoft 签名验证;安装助手验证 MSIX 信任且默认运行实际 OCR 诊断。
- [x] 测试签名:本地/手动验收默认 14 天、v0.1.0 发布使用 365 天自签证书;只导出公开 `.cer`、签后清理私钥,本地不自动信任。CI 专用身份与证书信任/包清理限于一次性 GitHub-hosted runner。
- [x] 已安装 AUMID 激活自检:生产识别实现读取合成 `OcrSmoke.png`,核对实际包身份、`LENZ`/`12345`、文本块与失败报告;不读用户配置/密钥、不截图。
- [x] 手动验收工作流:Windows MSVC 构建,选中 `verify_installed_ocr` 才在 Windows 11 ARM 客户端安装 x64 包并生成真实识别报告;默认只构建。此客户端安装/OCR job 尚未运行。
- [x] 本地验证:19 项 Node 测试、typecheck/build;61 项 Rust 测试(6 ignored);macOS/Windows GNU 全目标 clippy(`-D warnings`);Windows GNU 优化 exe 链接;6 个 PowerShell 脚本解析和真实 PE 清单暂存检查。
- [x] 隔离 IPC 浏览器验证:无身份/无语言提示分开、下载按钮打开官方 Release、中英深色与 720×480 布局;PowerShell 本地证书信任 guard、COM 激活桥接编译和工作流 YAML 检查。均不代替 Windows 运行验收。
- [x] PRD、spec 03/10/11、README、[Windows 分发指南](./windows-distribution.md)同步;bindings 由 Rust 重新生成。
- [x] v0.1.0 标签工作流已完成 Windows CI 实际 MSVC 构建、MakeAppx 打包和 SignTool 测试签名;MSIX 与安装助手 ZIP 已公开。
- [ ] Windows 客户端 MSIX 安装与 `installed-ocr.json` 识别验收;按用户要求推迟,不把原生构建成功算作识别成功。
- [ ] Windows 10/x64 原生及 Windows 11/ARM64 包安装、升级、缺 WebView2、无包身份/无语言包、F2/F3 中英识别及自启重登后身份验收。
- [ ] 正式代码签名证书/Store 签名与固定生产 Publisher;用户确认当前暂无正式证书。v0.1.0 已公开为测试签名安装包,尚非正式可信签名。

---

## 2026-09-22 · 全项目审查修复

按审查清单 P1 → P2 → P3 完成 16 项代码修复,详细变更和验证边界见 [修复记录](./reviews/2026-09-22-fixes.md)。以下为代码与自动化验证状态;原生权限、多屏与 Windows 实机检查仍单独验收。

- [x] P1:旧配置兼容;剪贴板快照、并发和条件恢复;截图前隐藏浮层。
- [x] P2:统一翻译会话;配置提交串行化;快捷键注册回滚;按引擎超时;供应商语言码转换;互换使用译文;补齐系统入口;丢弃过期引擎测试结果;取消 HTTP future;取消图标延迟任务。
- [x] P3:显示结构化错误;复用 HTTP client、目标语言变更不重载引擎;选区按动画帧合并重绘。
- [x] 附带修复 Google 返回值中的 HTML 实体重复解码。
- [ ] macOS 实机复核剪贴板富文本/图片、源应用快速复制、全屏和多屏截图、系统快捷键冲突。
- [ ] Windows 实机复核剪贴板、系统快捷键及多屏交互;2026-10-14 已补齐原生 MSVC 构建,运行验收仍待做。

---

## M0 · 准备(W0,可并行提前)

- [ ] **账号与证书**
  - [ ] 注册 Apple Developer Program(macOS 签名/公证前置)(→ spec 11 §4.1)
  - [ ] 申请 Windows 代码签名证书(评估 EV vs 标准,可延后到 M9)(→ spec 11 §4.2)
  - [x] GitHub 仓库 `huangj17/Lenz`,默认分支 `main`;v0.1.0 标签与 Release 已公开。
- [ ] **基础资产**
  - [ ] 确认产品中英文名(Lenz / Lenz)、slogan 与图标风格
  - [x] 应用 logo 与多尺寸安装包图标已生成(`src/assets/lenz-logo.*`、`src-tauri/icons/`)并用于 v0.1.0。
  - [ ] 准备托盘图标(macOS 模板黑白;Windows 彩色)(→ spec 10 §1.1)
  - [ ] 抢注域名与 Bundle ID(`app.lenz.desktop` / `Lenz`)(→ PRD §8)

---

## M1 · 项目搭建(W1)

目标:Tauri 项目初始化、基础窗口、托盘图标可用。

> **状态:✅ 已完成**(commit 5ea9cf8;设置页视觉在 7d780ae 进一步完善)。

### 1.1 脚手架
- [x] `cargo create-tauri-app` 初始化(React 19 + TypeScript + pnpm + Vite 7,版本不固定)
- [x] 前端目录按 spec 00 §2.2:`windows/{main,popup,selection,overlay}`、`components`、`hooks`、`stores`、`lib`、`locales`(→ spec 00 §2.2)
- [x] Rust 目录按 spec 00 §2.1(`commands/`、`core/`、`config/`、`types.rs`、`error.rs`)(→ spec 00 §2.1)
- [x] `tauri.conf.json` 的 `main` 和 `popup` 两个静态窗口(→ spec 09 §2.1 / §3.1)
- [x] shadcn 风格组件 + Tailwind v4;暗/亮色 CSS 变量骨架(→ spec 09 §6.1)
- [x] Zustand 基础 store:`config`、`session`
- [x] i18next 接入;`zh-CN.json` / `en-US.json`(→ spec 09 §6.4)
- [x] Rust 共享类型 `types.rs`(`Rect`、`TriggerKind`、`TranslateRequest/Response`)(→ spec 00 §5)
- [x] 统一错误枚举 `LenzError`(→ spec 00 §5, §6)
- [x] 前端 `lib/ipc.ts` 封装 + 统一 error 拦截(→ spec 00 §6)
- [x] 接入 `tauri-specta`,Rust → TS 类型导出链路跑通(生成 `bindings.ts`)(→ spec 00 §9)

### 1.2 配置与存储
- [x] `ConfigStore`:RwLock + JSON 原子写(tmp → rename)(→ spec 01 §4)
- [x] 默认 config 生成(首次启动)(→ spec 01 §3)
- [x] 损坏 config 的备份与重建逻辑(→ spec 01 §4.1)
- [x] JSON Merge Patch 的 `set_config` command(线缆用 String 承载,规避 specta)(→ spec 01 §5)
- [x] `config://changed` 事件 + keypath 标记(→ spec 01 §4.2)
- [x] 各监听方订阅 `config://changed`:前端主题/语言、托盘重建、快捷键热重绑、翻译引擎与自启同步已接通(→ spec 01 §4.3)。
- [x] schema 迁移已实现 v1→v7,含内置引擎、凭据启用修正、OCR 偏好与服务列表兼容,并有迁移回归测试(→ spec 01 §7)。
- [x] 测试(→ spec 01 §9) —— 注:merge patch / 损坏 fallback 已有单测;并发 / 含中文路径 TODO

### 1.3 托盘与主窗口
- [x] `TrayIcon` 构建 + 菜单项(打开主窗口、F1/F2/F3、划词监听 toggle、设置、检查更新、关于、退出)(→ spec 10 §1.2)
- [x] 左键点击托盘 → 显示主窗口(→ spec 10 §1.3)
- [x] macOS「设置...」增加 `Command+,` 原生菜单快捷键,接入托盘与默认应用菜单(→ spec 10 §1.2)。初次 fmt/check/test 通过(57 passed,6 ignored);2026-10-14 独立 QA 应用已验证实际按键及恢复隐藏窗口,托盘视觉与权限交互仍单独验收。
- [x] Windows「设置...」增加 `Ctrl+,` 托盘快捷键提示与应用内按键监听,复用主窗口显示、取消最小化和聚焦逻辑;补充平台、键盘布局、修饰键、输入法、长按和已消费事件的回归测试(→ spec 10 §1.2)。验证:`pnpm typecheck` / `pnpm build` / `pnpm test`(11 passed)及 macOS `cargo fmt -- --check` / `cargo check` / `cargo test`(57 passed,6 ignored)通过;当时 Windows 交叉编译缺少 windres;已在 2026-10-14 补齐工具并通过全目标 check/clippy,Windows 实机仍待验收。
- [x] 配置变更后菜单重建(`rebuild_tray_menu`)(→ spec 10 §1.3)
- [x] 三项翻译菜单改用原生快捷键提示,与「设置...」统一靠右显示;继续读取当前快捷键配置,清空时隐藏提示(→ spec 10 §1.2)。验证:`cargo fmt -- --check` / `cargo check` / `cargo test` 通过(57 passed,6 ignored);原生菜单视觉验收及 Windows 实测待完成。
- [x] 主窗口 Tab 导航(→ spec 09 §2.2) —— 注:实际为 通用/快捷键/服务/关于(服务页分段)
- [x] macOS 不在 Dock 显示(`ActivationPolicy::Accessory`)(→ spec 11 §4.1)

### 1.4 启动时序
- [x] `app.setup()` 按 spec 00 §7,各步失败不阻断并 emit `app://startup-warning`(→ spec 00 §7)
- [x] `tauri-plugin-log` 接入(→ spec 11 §5)

### 1.5 验收(M1 结束)
- [x] `pnpm tauri dev` 启动,托盘与主窗口可见 —— 注:macOS 已验证(含全局快捷键注册);Windows 待
- [x] 冷启动 → 托盘可用 ≤ 2s(粗测)(→ spec 11 §1)
- [x] 配置文件读写闭环 + 破坏 config 不崩溃

---

## M2 · OCR 桥接(W2-3)⚠️ 高风险

目标:macOS Vision + Windows.Media.Ocr 完整打通,输出统一 `OcrResult`。

> **进度(2026-06-15)**:抽象层(2.1)、xcap 截图(2.2)、macOS Vision(2.3)已完成;
> 可行性 **GO** —— headless 单元测试在内存图像上零权限识别文字(`Hello Vision 2026`,conf 1.0,~320ms),无需 Tesseract 兜底(2.6)。
> **更新(2026-10-14)**:Windows.Media.Ocr、MSIX 包身份和安装诊断已实现,MSVC 原生构建与测试签名打包成功;待客户端安装/识别验收。真实多屏/Retina bbox 校准与发布版权限回归仍待做;macOS 开发版 F2 全流程已在 M4 验证。

### 2.1 抽象层
- [x] `OcrEngine` trait 定义(→ spec 03 §2)
- [x] `OcrInput`、`OcrOptions`、`OcrResult`、`OcrBlock` 共享类型(→ spec 03 §2)
- [x] `default_engine()` 工厂 + 启动阶段注入 `State<Arc<dyn OcrEngine>>`(→ spec 03 §2.1)
- [x] `run_ocr` Tauri command(改为 `run_ocr(rect)`,内部截图+识别)(→ spec 03 §6)

### 2.2 截图采集
- [x] `xcap` 包装:完整屏幕位图 + 单屏抓取(→ spec 02 §2.1)
- [x] `capture_screen_region(rect)`:逻辑坐标 → 物理像素裁剪(→ spec 02 §2.2) —— 注:单屏已实现;跨屏拼接 TODO
- [x] `CaptureResult { png, pixel_rect, scale_factor }` 返回(→ spec 02 §4)
- [x] 无屏幕录制权限的 macOS → 返回 `LenzError::Permission`(→ spec 02 §6) —— 注:代码层映射;真实授权弹窗未运行验证
- [ ] 真实截图端到端验证(需屏幕录制授权 + 运行;依赖 M4 选区或调试入口)

### 2.3 macOS Vision
- [x] `objc2` / Vision framework FFI 接入(objc2-vision 0.3.2)(→ spec 03 §3)
- [x] `VNRecognizeTextRequest` 执行 + `.accurate` / `.fast` 模式(→ spec 03 §3)
- [x] 归一化坐标 → 屏幕逻辑坐标的回映射 helper(Y 翻转 + 像素 + pixel_rect + scale)(→ spec 03 §5) —— 注:fixture(1×)验证;真实多屏/Retina 校准 TODO
- [x] `supportedRecognitionLanguages` 读取(→ spec 03 §3.1)
- [x] 用 `tokio::task::spawn_blocking` 包裹调用(隔离非 Send 的 ObjC 对象)(→ spec 03 §7)
- [-] entitlements 声明 `NSScreenCaptureUsageDescription`(→ spec 11 §4.1) —— N/A:macOS 屏幕录制走 TCC 运行时授权,无标准 Info.plist usage key;非沙盒分发无需特殊 entitlement

### 2.4 Windows.Media.Ocr(原生 MSVC 构建与 MSIX 打包通过,待安装运行验收)
- [x] `windows` crate(0.62)引入 `Graphics.Imaging` + `Media.Ocr` + `Storage.Streams` + `Globalization`(→ spec 03 §4)
- [x] PNG → `InMemoryRandomAccessStream` → `BitmapDecoder` → `SoftwareBitmap` 流水线(→ spec 03 §4)
- [x] `TryCreateFromLanguage` 依序尝试 + `TryCreateFromUserProfileLanguages` 回退(→ spec 03 §4)
- [x] Line bbox = Word.BoundingRect 并集;Windows 不返回真实置信度,契约填 0.9 但不参与置信度过滤;`map_pixel_bbox` 回映射(→ spec 03 §4)。
- [x] 无语言包 → emit `ocr://language-missing`;无身份 → `ocr://package-required`,均接入 F2/F3 与设置页(→ spec 03 §4.1、§4.2)。
- [x] Windows GNU 全目标 check/clippy、优化 exe 链接及 GitHub 原生 MSVC 构建通过;COM 初始化和 WinRT `join()` 已编译验证。MSIX 分发、包内自启与安装 OCR 诊断已实现;真实系统运行结果仍待 Windows 客户端验收。

### 2.5 坐标验证(可视化调试工具)
- [ ] Debug 面板:输入一张 fixture 图,彩色框叠加显示 block bbox,目视对齐(→ spec 03 §8)
- [x] 多 DPR 回映射单测:1×/2×/3×(→ spec 02 §8) —— `map_normalized_bbox`(Vision 归一化+翻转)与 `map_pixel_bbox`(WinOCR 绝对像素)各档已单测;真实屏可视化校准待

### 2.6 兜底计划 ⚠️
- [x] W2 末尾做可行性评估(→ PRD §6.2) —— **结论:GO**,Vision 桥接可行,无需 Tesseract 兜底
- [x] 在 `core/ocr/` 下为 Tesseract 预留实现位 —— `tesseract.rs` + `ocr-tesseract` feature(默认不编译、不引入依赖)

### 2.7 验收(M2 结束)
- [x] fixture 图识别,block 数量与人工标注 ±1(→ spec 03 §8) —— 注:macOS Vision headless 单测通过;Windows 待
- [ ] 1920×1080 区域 OCR ≤ 1s(→ spec 11 §1) —— macOS 小图识别实测约 320ms;尚无该分辨率实测,不能据此判定性能目标通过。
- [ ] bbox 在屏幕叠加与原文字对齐 ≤ 2 逻辑像素(→ spec 03 §8) —— 真实屏端到端校准待
- [ ] Windows 无语言包 graceful 返回 `OcrUnavailable`,不崩溃(→ spec 03 §8) —— 生产实现已区分缺语言与缺包身份;Windows 未运行验证。

---

## M3 · 翻译引擎(W4)

目标:OpenAI 兼容接口可用,配置体系完整。

> **进度(2026-06-16)**:抽象层 + 管理器(3.1)、OpenAI 兼容(3.2)、传统 API
> Google/DeepL/有道 + 微软 Azure(3.3)、代理(3.4)、commands(3.5)、错误映射(3.6)均已实现,
> `cargo check` / `cargo clippy`(本模块无新增告警)/ 34 项单测(含纯函数 helper + 迁移)/
> `pnpm typecheck` 全过。`reqwest`(native-tls + socks)+ `sha2` 已入依赖。
> 另:新增 `EngineKind::{Microsoft, Apple}` 与 `region` 字段,内置 8 个默认引擎预设
> (有道/苹果/智谱/硅基流动 默认开,谷歌/微软/ollama/LM Studio 默认关;schema v1→v2 迁移注入);
> **苹果翻译更新(2026-10-14)**:已接入 macOS 15+ Translation 原生桥接,系统语言包和跨语言原生验收待做(见本文最新记录)。
> **3.7 验收已通过**(2026-06-16):对真实 Ollama(`qwen2.5:1.5b`)/ LM Studio 端点新增
> 7 项 `#[ignore]` 门控验收测试(`mod acceptance`)全绿——端到端 / 批量 10 条 /
> test_engine 57ms / 引擎热更新路由 / 代理强制 + 本地转发拦截(无 mitmproxy 时的等价证明)。
>
> **更新(2026-07-07):应用内代理功能已移除**——`ProxyConfig` / 引擎 `useProxy` 删除,
> `reload` 去掉 proxy 参数,双 client 合并为单 client(reqwest 默认,尊重环境变量代理),
> 相关验收测试(代理强制 / 本地转发)与 `reqwest` 的 `socks` 特性一并删除。见 spec 04 §7 / spec 10 §5。

### 3.1 抽象与管理
- [x] `Translator` trait(→ spec 04 §1) —— `core/translator/mod.rs`,含 id/name/kind/supports_batch/translate
- [x] `EngineKind` 枚举(→ spec 04 §1) —— 复用 `config::schema::EngineKind`(openai/google/deepl/youdao/baidu/microsoft/apple)。
- [x] `TranslatorManager`(含共享 `reqwest::Client` 与 `default_id`)(→ spec 04 §2) —— 所有引擎共享单 client,id→引擎表
- [x] `reload(configs, default_id)` 支持热更新(不重启)(→ spec 04 §2)
- [x] 订阅 `config://changed` 触发 reload(→ spec 01 §4.3) —— `setup.rs` listen,仅 `translator.*`/整体重置触发

### 3.2 OpenAI 兼容
- [x] 单条翻译(chat completions 非流式)(→ spec 04 §3.1) —— 端点容忍是否已含 `/v1`;空 apiKey 不带鉴权头(本地 LLM)
- [x] 默认 system prompt + `{sourceLang}`/`{targetLang}` 变量替换(→ spec 04 §3.1) —— BCP-47 → 英文全名
- [x] 批量翻译(JSON 数组)+ 解析兜底(失败降级为逐条串行)(→ spec 04 §3.2) —— 含去 ```json 围栏 + 长度校验
- [x] 超时字段尊重(`engine.timeoutMs`) —— 每请求 `.timeout(timeoutMs)`

### 3.3 传统 API(P0 但可分批)
- [x] Google Translate v2(批量 `q[]`)(→ spec 04 §4) —— 含 `detectedSourceLanguage` + HTML 实体还原
- [x] DeepL(Free/Pro URL 切换 + 语言代码映射 `zh-CN` → `ZH`)(→ spec 04 §5) —— key 尾 `:fx` 判 Free
- [x] 有道智云(`appKey:appSecret` 拆分)(→ spec 04 §6) —— 注:实际为 signType=v3 的 **SHA256** 摘要(非 HMAC);`supports_batch=false`,引擎内逐条
- [x] 微软 Azure Translator(批量 JSON + Key + region)(→ spec 04 §6.1) —— `TranslatorEngine` 新增 `region` 字段
- [x] 苹果系统翻译(→ spec 04 §6.2) —— 原生 helper/Translation 桥接,Windows 和 macOS 15 以下明确禁用;语言包实机验收待做
- [x] 内置默认引擎预设(全部默认关闭,用户配置后启用)(→ spec 04 §12) —— `default_engines()` + schema v1→v2 迁移注入 + 前端 `default-config.ts` 镜像

### 3.4 代理(已移除)
- [-] 应用内代理功能已移除(2026-07-07)(→ spec 04 §7) —— 原 `build_http_client(proxy)` / `useProxy` / 双 client 全部删除,改为共享单 client(reqwest 默认,尊重系统环境变量代理);如需代理由用户在系统层配置

### 3.5 Tauri commands
- [x] `translate_text` / `translate_batch`(→ spec 04 §8) —— 均路由 `TranslatorManager::translate`
- [x] `test_engine`(发送 "Hello" → 返回延迟/结果)(→ spec 04 §8) —— 一次性引擎;失败 `ok=false`
- [x] `list_engines`(→ spec 01 §5) —— M1 已有

### 3.6 错误映射
- [x] 超时 → `Network("timeout")`(→ spec 04 §9)
- [x] 4xx → `TranslateFailed("auth"|"...")`;5xx → `"server_error"`(→ spec 04 §9) —— 401/403→auth、429→rate_limit、其余 4xx→client_error
- [x] 解析失败 → `"parse_error"`(→ spec 04 §9)

### 3.7 验收(M3 结束)
> **验收方式**:把运行态手测固化为 `#[ignore]` 门控的真实端点测试(`core/translator/mod.rs` 的 `mod acceptance`,7 项)。运行:
> `cargo test --manifest-path src-tauri/Cargo.toml --lib acceptance -- --ignored --nocapture --test-threads=1`。
> **2026-06-16 全绿**(本机 Ollama `:11434` `qwen2.5:1.5b` + LM Studio `:1234`)。
- [x] LM Studio / Ollama 本地端点接入,一段英文 → 中文端到端 OK(→ spec 04 §10) —— Ollama `Hello, world…`→`你好，世界。这是一个端到端翻译测试。`(248ms);LM Studio→`早上好，祝你有美好的一天。`(5665ms)
- [x] 批量 10 条单次请求返回,长度一致(→ spec 04 §10) —— 10→10:`苹果/香蕉/猫/狗/房子/河流/山峰/书/音乐/朋友`,无空槽
- [x] 设置页"测试连接"按钮 ≤ 3s 返回结果(→ spec 04 §10) —— `test_engine` 命令路径实测 57ms(暖模型);按钮 UI 接线见 M7 §7.3(已 [x])。注:冷启动含模型加载可能 >3s
- [-] ~~代理启用后验证流量走代理~~(代理功能已移除,2026-07-07;相关验收测试已删除)
- [x] 引擎配置热更新后,下一次翻译使用新配置无需重启(→ spec 04 §10) —— `reload` 切默认引擎 `hot-a`→`hot-b`,同一 manager 实例下次翻译即生效

---

## M4 · 截图翻译 F2(W5-6)

目标:F2 全流程可用(截图 → OCR → 翻译 → 弹框)。

> **进度(2026-06-16)**:4.1~4.5 **代码完成**(`cargo build` / `cargo test --lib` 37 过 /
> `cargo clippy` 本模块无新增告警 / `pnpm typecheck` / `vite build` 全过)。
> 新增:`core/selection_window.rs`(运行时建/销选区窗口 + 虚拟屏并集)、
> `core/screenshot.rs`(F2 pipeline + 会话代号取消 + 10s 超时 + `join_blocks` 单测×3)、
> `commands/screenshot.rs`(`cancel_screenshot` / `open_screen_settings`);
> 前端 `SelectionWindow`(canvas 选区)、`PopupWindow`(三态);
> 事件 `screenshot://progress|result|error`、capability 增 `selection`/`overlay` 窗口。
> **待办(4.6)**:GUI 运行态指标需 `pnpm tauri dev` + 屏幕录制授权 + 真实框选手测;
> 多屏混合 DPR 选区精度沿用 spec 02 §9 开放问题(单屏 / 同 DPR 多屏按设计精确)。
>
> **打磨补充(2026-06-16)**:① 划词快捷键 Alt+D 接通(剪贴板模式,见 M5 §5.2);
> ② 多屏 capture 改为**按重叠面积最大选屏**(`pick_monitor`),更准区分选区所在屏;
> ③ popup 头部 `data-tauri-drag-region` 可拖动;
> ④ popup / 选区蒙层样式对齐设计稿(`popup-f1/f2`、`capture-f2`):暖色毛玻璃卡片 +
> Lens 头部 + 原文块(识别语言 chip + 字数)+ 语言切换条 + 引擎结果行(徽标 + 用时);
> 选区加顶部指示条 + 十字辅助线。`.ln-popup` / `.ln-engine-badge` 入 `theme.css`。
>
> **验收完成(2026-07-08)·GUI 运行态手测通过**:授予屏幕录制权限后 `pnpm tauri dev`
> 真实框选跑通 F2 全流程(截图 → OCR → 翻译 → 弹框),4.6 达标。期间发现并修复的运行态
> 问题已固化为提交:① 透明窗首帧即置透明,消除选区蒙层白闪(298010f);② 截图前先隐藏
> 并等蒙层离屏,修复白闪与 OCR 空结果(cff0d1a);③ 多屏改为每屏一个蒙层窗口(d808bb4)
> + 各屏按自身 scale 换算逻辑坐标精确铺满,修混合 DPR 下蒙层只铺一角(`selection_window::
> position_and_size`);④ OCR 文本拼接保留段落空行(`join_blocks`,`\n\n`)。多屏**混合** DPR
> 选区精度沿用 spec 02 §9 开放问题(单屏 / 同 DPR 精确,混合近似)。

### 4.1 选区蒙层
- [x] 运行时创建 `selection` 窗口覆盖全虚拟屏(→ spec 02 §3.1 / spec 09 §4.1) —— `selection_window::open`,主线程构建 + 逻辑并集定位
- [x] 前端 canvas 实现:半透遮罩 + 镂空 + 选区边框 + 尺寸提示(→ spec 02 §3.2 / spec 09 §4.2)
- [x] 鼠标事件:down/move/up 绘制;ESC 取消(→ spec 02 §3.2)
- [x] `clientX/Y` + 窗口逻辑原点(`outerPosition().toLogical`)→ 虚拟屏逻辑坐标(→ spec 09 §4.3)
- [x] emit `capture://region-selected { x,y,w,h, trigger }`(→ spec 02 §3.3) —— 注:displayId 暂未填(可选)
- [x] `w<4 || h<4` 视为点击取消,emit `capture://region-cancelled`(→ spec 02 §6)
- [x] `start_selection` / `cancel_selection` commands(→ spec 02 §4)

### 4.2 F2 流程编排
- [x] 全局快捷键 `Alt+S` → 开选区(→ spec 06 §2.1) —— `shortcut://screenshot` 监听 → `selection_window::open(Screenshot)`
- [x] 接收 `capture://region-selected` → 销毁选区窗口 → 执行采集(→ spec 06 §2.2)
- [x] capture → OCR(按 `preferred_languages` + `min_confidence`)(→ spec 06 §2.3, §2.4)
- [x] OCR block 拼接(同行空格,跨行 `\n`)→ 送翻译(→ spec 06 §2.4) —— `join_blocks`,3 项单测
- [x] OCR 为空 → emit `screenshot://error { stage: "ocr", code: "no_text" }`(→ spec 06 §2.4)
- [x] 翻译完成 → emit `screenshot://result { ... }`(→ spec 06 §2.6)

### 4.3 进度与取消
- [x] 关键节点 emit `screenshot://progress { stage }`(→ spec 06 §5)
- [x] per-session 取消;新触发取消旧会话(→ spec 06 §7) —— `ScreenshotSession` 自增代号,await 点检查 `is_current`
- [x] ESC 取消(任意阶段)(→ spec 06 §6) —— 选区窗口 ESC + popup ESC→`cancel_screenshot`;焦点恒在我方窗口故覆盖全程
- [x] 截图独立 10s、OCR 常规 10s,Vision 首次语言/模式初始化允许 60s;翻译使用引擎 `timeoutMs`,超时保留原文(2026-10-14 修订,→ spec 03 §7、spec 06 §4)。

### 4.4 popup 弹框
- [x] popup 窗口静态配置(常驻隐藏)(→ spec 09 §3.1) —— M1 已有
- [x] 布局:语言行 / 原文(可折叠)/ 译文 / 工具栏(→ spec 09 §3.2)
- [x] 状态:loading(骨架屏)/ success / error(→ spec 09 §3.3)
- [x] 监听 `screenshot://result` 与 `screenshot://error` → 渲染(→ spec 06 §3)
- [x] 动态高度按内容调整(→ spec 09 §3.1) —— `useLayoutEffect` 量测卡片高度,经 `resize_popup` 命令设尺寸;macOS 同时 `invalidateShadow` 消除透明窗缩放后残留的旧阴影(弹框旁灰块)。`.ln-popup` 改由原生窗口阴影投影,去掉被窗口留白裁切的 CSS box-shadow
- [x] 定位:选区下方 `y + h + 12`,水平居中;溢出则夹紧(→ spec 06 §3) —— Rust `position_and_show_popup`,夹紧到所在屏
- [x] 点击窗口外/ESC → 隐藏(→ spec 09 §3.4) —— `onFocusChanged`(loading 态不隐)+ ESC
- [x] 复制译文按钮(→ spec 09 §3.4) —— `navigator.clipboard`

### 4.5 权限引导
- [x] macOS 屏幕录制权限缺失 → popup 展示引导 + "打开系统设置"按钮(→ spec 06 §4) —— `open_screen_settings` 深链 TCC 设置

### 4.6 验收(M4 结束)—— ✅ GUI 运行态手测通过(2026-07-08)
- [x] Alt+S → 进入截图模式 ≤ 300ms(→ spec 06 §8) —— 运行态:按键即出选区蒙层,无感知延迟
- [x] 框选 → 弹框占位出现 ≤ 500ms(→ spec 06 §8) —— 框选松手 popup 立即 show(loading 骨架)
- [x] 框选 → 完整结果 ≤ 3s(本地 OCR + 本地 LLM)(→ spec 06 §8) —— 本地 Vision(~320ms)+ 本地 LLM 端到端达标
- [x] ESC 任意阶段可取消,不残留窗口(→ spec 06 §8) —— 选区 ESC / popup ESC(cancel_screenshot)均无残留窗
- [x] 多屏 + 不同 DPR 选区准确(→ spec 02 §7) —— 单屏/同 DPR 精确;每屏一蒙层 + 各屏自身 scale 铺满修混合 DPR;混合 DPR 精度近似(spec 02 §9 开放)
- [x] 连续 Alt+S(未完成先前)→ 新会话正确取代旧会话(→ spec 06 §9) —— 会话代号机制运行态验证:新框选取消旧会话

---

## M5 · 划词翻译 F1(W7)

目标:F1 功能,macOS 和 Windows 的监听代码已就绪,原生验收独立跟踪。

> **设计调整(2026-06-18)·自动模式改「划词指示图标」**:spec 05 §1(选中松手即译)与 §2.1(剪贴板 changeCount)互相矛盾,经用户拍板,自动模式不做「选中即译」,改为 **Bob/Easydict 式划词图标**:监听到划词/双击选中手势后,在光标处浮出 Lenz 小图标,用户 **hover/点击图标** 才合成复制+翻译并弹框。更克制(不主动改剪贴板)。Alt+D 快捷键仍是显式划词主路径。**spec 05 §1/§2.1/§5/§10 待据此同步**。
>
> **本步进度(2026-06-18)·Increment 1**:① 弹框光标锚定(§7,`core/cursor.rs` + `position_popup_at_cursor`,已用于 Alt+D);② `SelectionListener`(managed state,start/stop/restart + `sync` 据 `selection.enabled`(托盘总开关,默认 false / opt-in)启停)+ macOS `CGEventTap`(listen-only,左键 down/up,拖拽位移≥4pt 或双击 clickState≥2 判为选中 → emit `selection://detected{x,y}`);③ `AXIsProcessTrusted` 权限检测 + emit `selection://permission-required`;④ `toggle_selection_listener` 真启停 + 托盘/启动/`config://changed` 接线。⚠️ CGEventTap 需「辅助功能」权限,后台无法实机验证,待用户授权后实测(看日志「划词监听已启动」+ 选中是否 emit detected)。**Increment 2(下一步)**:划词指示图标窗口(`?window=indicator`)消费 `selection://detected` 显示 logo + hover/点击触发翻译;前端权限引导;Windows 监听。
>
> **本步进度(2026-06-18)·Increment 2 完成**:① 静态 `indicator` 窗口(transparent/alwaysOnTop/`focus:false`/`acceptFirstMouse:true`)+ `core/indicator.rs`(`show_at` 在光标处定位+show **不取焦** + `IndicatorSession` 自动隐藏 2.5s;`hide`);② setup 订阅 `selection://detected` → `indicator::show_at`;③ 前端 `IndicatorWindow.tsx`(Lens 图标,**hover 或点击** → `trigger_selection_translate`,1s 去抖)+ `App.tsx` 路由 + `theme.css` 透明底;④ `trigger_selection_translate` 命令(hide 图标 → 等 120ms 焦点回原应用 → 复用 `on_selection_shortcut` 合成 Cmd+C+翻译+光标处弹框);⑤ 权限引导:`open_accessibility_settings` 命令 + 「启用监听但未授权」自动打开辅助功能设置面板;⑥ 前端契约同步(`bindings.ts` 重新生成 + `ipc.ts`/`types.ts` 镜像)。验证:`cargo build --bin lenz` 链接成功(`AXIsProcessTrusted` 经 `#[link(ApplicationServices)]` 解析)+ `pnpm typecheck` ✓。⚠️ **实机待验**:(a) CGEventTap 是否在授权后 emit detected;(b) **hover(pointerenter)在非活动图标窗是否触发**——点击有 `acceptFirstMouse` 兜底;(c) 点击图标后 hide→焦点回原应用→合成 Cmd+C 的时序是否稳。**仍缺**:§9 状态机形式化、前端动态权限横幅(目前靠自动打开设置 + 设置页静态 `permissionNote`)、Windows 监听、entitlements、§10/§11 验收实测。
>
> **修复(2026-06-18)·误触 + 即时隐藏**:① **误触**——划词手势不再直接显示图标,改为后台探测:`probe_selection_then_indicate` 合成 Cmd+C → 若剪贴板确变(确有选中文本)才暂存文本(`PendingSelection`)+ **还原剪贴板**(不留痕,非文本变动也一律还原以防污染)+ 显示图标;点按钮/菜单等非文本手势 Cmd+C 空操作 → 不显示。图标触发改为直接翻译暂存文本,不再 engage 时合成复制(消除焦点时序隐患)。② **即时隐藏**——监听线程维护图标可见态+矩形全局态(`indicator::is_visible/hit_test`),mouseDown 落在图标外即 emit `selection://dismiss` → 立刻隐藏(2.5s 自动隐藏降为兜底);落在图标内放行点击触发。残留边角:Finder 拖选文件可能仍误显(文件路径有文本表示),少见可接受。
>
> **回退(2026-06-18)·撤销「探测」,回到 §6「不读剪贴板」**:上条的后台探测有致命副作用——每次划词手势都合成 Cmd+C 并**还原剪贴板**,会**覆盖用户自己刚 Cmd+C 复制的内容**(「划词时复制不了别的文字」)。经用户拍板按 spec 05 §6 最小改动:① 删除 `probe_selection_then_indicate` / `PendingSelection` / `run_selection_text`,`selection://detected` 直接 `indicator::show_at`(不碰剪贴板);复制只在 hover/点击图标(`trigger_selection_translate`)或 Alt+D 时发生。代价:图标在所有划词/双击手势(含非文本,如拖窗口/双击按钮)都会浮现——精度不如探测,若日后嫌吵再上 Accessibility API(AXSelectedText)直读。② **Alt+D 没把选中文字带进弹框**的真因不是权限(CGEventTap 已跑起来即说明辅助功能已授予),而是 Alt+D 触发时 Option 仍按着,合成的 Cmd+C 被并成 ⌘⌥C → 不复制 → 剪贴板不变 → 误判「无新选区」回显上次结果。修复:`keystroke::modifiers_held()`(读 `CGEventSourceFlagsState`)+ `handle_selection_shortcut` 合成前先等修饰键松开(≤400ms,`wait_modifiers_released`)。
>
> **重做(2026-06-18)·AX 直读取代「合成 Cmd+C + 比对剪贴板」**:上条的最小改动经全链路复查仍不可靠——「合成 Cmd+C + 比对剪贴板」有多个固有失败模式,尤其 **`copy_selection` 会把选区留在剪贴板,反复选同一段文字时剪贴板内容不变 → `selection != before.trim()` 误判无选区 → 回显上次结果**(开发者反复测同一词必中,极可能就是「怎么选都不进弹框」)。经用户拍板改用 **macOS Accessibility API 直读选中文本**:新增 `core/selection_text.rs::read_selected_text()`(`AXUIElementCreateSystemWide` → `AXFocusedUIElement` → `AXSelectedText`,属性名用 `CFString::from_str` 构造,不依赖 kAX 静态符号;`objc2-core-foundation` 直接依赖)。`handle_selection_shortcut` 改为:①AX 直读(spawn_blocking,不堵 async)→ 有则直接翻译(不碰剪贴板/不合成键,根治修饰键污染+同文字重复+焦点时序+剪贴板覆盖);②AX 取不到(部分 Chrome/Electron)→ 兜底合成 Cmd+C,改用**哨兵串**法判定是否真复制到(`copy_selection_via_clipboard`,不靠与旧值差异)+ 用后还原原剪贴板;③都没有→回显 LastResult。`wait_clipboard_change` 删除。验证:`cargo check`/`clippy`/`test`(37 passed)通过,bindings 不变(`read_selected_text` 非命令)。⚠️ 仍待实机验证 AX 在常用 app 的覆盖与兜底回退。
>
> **收尾(2026-07-08)·macOS 侧剩余项落地 + 未决项定性**:补齐 spec 05 §4 的**长度过滤**
> (`translate_selection` 先 `trim`,空串忽略 → 下界 ≥1;截断 5000 字 → 上界)与**去重**
> (原文与 `LastResult` 上次原文相同 → 回显缓存、不重复请求引擎,满足验收「同段不重复触发」;
> 抽出共用的 `reshow_result`);**去抖**接 `config.selection.popupDelayMs`(图标 `show_debounced`
> 取 `max(220ms 下限, popupDelayMs)`,下限保证仍能合并双击/三击)。`cargo check`/`clippy`
> (本模块 0 新增告警)/`test`(37 passed)全过。**定性未决项**:① `selection://triggered`
> 事件与 §9 状态机形式化——现流程改由 Rust 直接编排 + 复用 `screenshot://*` 事件,状态由
> `IndicatorSession`(WAITING)/`ScreenshotSession` 代号(TRANSLATING)/popup 显隐(SHOWING)
> 分散承载,不再单列事件/枚举(见下方 [-] 项);② `apple-events` entitlement 不适用(用
> CGEventTap + AX,走 TCC「辅助功能」运行时授权,非 Apple Events;非沙盒分发无需)。**仍缺**:
> Windows 监听已在 2026-10-14 落地；§10/§11 实机验收(需辅助功能授权后 GUI 手测)。

### 5.1 监听层
- [x] `SelectionListener` 抽象(start/stop/restart)(→ spec 05 §2.3) —— `core/selection.rs` 具体类型(非 trait):managed state + `sync(app)` 据配置启停;macOS 用独立线程跑 CFRunLoop + CGEventTap
- [x] macOS:`CGEventTap` 监听 `kCGEventLeftMouseUp`(→ spec 05 §2.1) —— 同时监听 down 记录起点,判拖拽/双击;listen-only 不吞事件;被系统超时禁用时自动 re-enable
- [-] 监听层读取剪贴板的旧方案不再使用(→ spec 05 §2.1);改为 AX 直读,仅明确翻译触发时使用剪贴板快照/复制/条件恢复兜底。
- [x] macOS:`AXIsProcessTrusted` 检测 + emit `selection://permission-required`(→ spec 05 §2.1);无权限不启动监听,设置页已接权限入口与重新检查,系统授权后的监听实测仍待做。
- [-] macOS entitlements 声明 `com.apple.security.automation.apple-events`(→ spec 11 §4.1) —— N/A:划词用 `CGEventTap` + Accessibility API(`AXSelectedText`),均走 TCC 运行时「辅助功能」授权,非 AppleScript/Apple Events;非沙盒分发无需该 entitlement(与 M2 §2.3 屏幕录制同理)
- [x] Windows:WH_MOUSE_LL 独立线程监听拖拽/双击,UIA TextPattern 直读选区(→ spec 05 §2.2)
- [x] Windows:保留 Ctrl+C 和原生多格式剪贴板快照/恢复兜底(→ spec 05 §3)

### 5.2 触发逻辑
- [x] 快捷键/图标触发优先 AX(macOS)/UIA(Windows)直读选区,回退剪贴板兜底,由 Rust 编排翻译并在光标处显示同一 popup(→ spec 05 §3)。
- [x] 剪贴板兜底:等待修饰键释放 → 原生格式快照 → 合成 Cmd/Ctrl+C → 版本号判定变化 → 条件恢复;有新输入/复制时保留用户较新的内容,不写占位哨兵(→ spec 05 §3)。
- [x] 无新选区时回显 `LastResult`(F1/F2 内存+磁盘);无历史则忽略,不弹空框。spec 05 §3.1/§10 已同步此行为。
- [x] 自动触发:去抖 `popupDelayMs` + 长度过滤(1~5000)+ 去重(→ spec 05 §4) —— 去抖取 `max(220ms, config.selection.popupDelayMs)`,空串忽略、截断 5000 字;原文/目标语言/引擎均相同时回显缓存不重译。
- [-] emit `selection://triggered { text, source, cursorPos }`(→ spec 05 §5) —— 未采用:流程改由 Rust 直接编排(手势→图标→`trigger_selection_translate`→复用 F2 的 `screenshot://progress|result|error` 渲染),不再经此中间事件(与 5.3 一致)
- [x] `toggle_selection_listener` command(→ spec 05 §8) —— 写 `selection.enabled` → `config://changed` 订阅者据 `selection.enabled`(托盘总开关,默认 false / opt-in)启停监听(`core::selection::sync`)
- [x] 托盘"划词监听"toggle 同步状态(→ spec 10 §1.2) —— 复用同一 `config://changed` 路径,菜单勾选 + 监听器启停一并联动

### 5.3 popup 复用
- [x] 监听 `selection://triggered` → 复用同一 popup 实例(→ spec 05 §6) —— 改为:图标 hover/点击 → `trigger_selection_translate` → 复用 Alt+D 编排(同一 popup 窗口 + `screenshot://*` 事件渲染)
- [x] 正在翻译中触发新文本 → 取消旧请求(→ spec 05 §6) —— 复用 `ScreenshotSession` 代号:每次 `on_selection_shortcut` 都 `begin()` 令旧流程在 await 点放弃
- [x] 定位:`cursorPos + (16,16)` 偏移 + 溢出夹紧(以 popup 尺寸估算)(→ spec 05 §7) —— `position_popup_at_cursor`,已用于 Alt+D 划词路径(取不到光标回退居中);Increment 2 的图标触发翻译复用此定位

### 5.4 状态机
- [-] IDLE → WAITING → TRANSLATING → SHOWING 状态切换(→ spec 05 §9) —— 不单列形式化枚举:状态已由既有机制分散承载——图标浮出(`IndicatorSession`)= WAITING、翻译 pipeline(`ScreenshotSession` 代号 + `emit_translating`)= TRANSLATING、popup 显示结果 = SHOWING、无活动会话 = IDLE;再加一层枚举会与会话代号重复记账(与「未走 `selection://triggered` 事件」同因)
- [x] ESC / 文本清空 → 回 IDLE(→ spec 05 §9) —— ESC:popup `cancel_screenshot` 隐藏弹框 + 取消会话;失去选区(点图标外):监听线程 mouseDown 命中测试 → `selection://dismiss` 立即隐藏图标

### 5.5 验收(M5 结束)—— 代码就绪,待授「辅助功能」后 GUI 手测
- [ ] 选中 → 弹框出现 ≤ 500ms(→ spec 05 §10) —— AX 直读 + 光标处 popup 已实现,待授权实测
- [ ] 快速连选三段,仅最后一段展示;同段不重复触发(→ spec 05 §11) —— 逻辑已就绪:`show_debounced` 去抖只留最终段 + `translate_selection` 同原文去重,待实测
- [ ] 无选中文本时按快捷键 → 回显上次结果,含应用重启后回显(→ spec 05 §3.1/§10/§11) —— 已实现 `LastResult` 内存+磁盘与 `reshow_result`,待实测。
- [ ] 弹框不超屏幕边界(→ spec 05 §10) —— `position_popup_at_cursor` 夹紧所在屏,待实测
- [ ] macOS 权限未授时弹引导(→ spec 05 §11) —— 已实现:`selection://permission-required` + 启用监听未授权时自动打开辅助功能设置,待实测
- [ ] Windows 基础路径跑通(若 UIA 稳定性不足可延后自动触发,保留快捷键触发)⚠️ —— 代码与交叉编译就绪,待 Windows 实机

---

## M6 · 原图覆盖 F3(W8-9)

目标:F3 完整实现,跨屏/HiDPI 正确。**代码已接通;原生系统与多屏验收独立保留。**

### 6.1 流程编排
- [x] Alt+F → `start_selection(Overlay)`;选区事件 → capture/OCR/翻译(→ spec 07 §1)。
- [x] 保留独立 OCR block 和顺序;macOS 置信度过滤,Windows 不使用合成置信度(→ spec 07 §2)。
- [x] 支持批量与逐块回退,部分失败不覆盖原文,全失败显示错误;独立取消和会话有效性检查(→ spec 07 §3)。

### 6.2 颜色采样
- [x] 四边内缩 2 物理像素,R/G/B 中位数、黑白前景色和批量采样(→ spec 07 §4)。

### 6.3 overlay 窗口
- [x] 运行时透明/无边框/置顶/不夺焦/不占任务栏/鼠标穿透(→ spec 07 §5)。
- [x] 捕获裁剪区域、负坐标和来源屏比例定位;新触发关闭旧层(→ spec 07 §5.2)。
- [x] show/close/payload/ready 命令及类型契约(→ spec 07 §7)。

### 6.4 前端渲染
- [x] managed payload + ready 握手取代首条 init 事件,避免首帧竞态(→ spec 07 §6.1)。
- [x] bbox 相对定位、canvas 8~32px 字号适配、溢出裁剪省略(→ spec 07 §6.2~6.3)。

### 6.5 关闭交互
- [x] 临时 Esc,关闭后仅解绑本流程拥有的注册;冲突时托盘「关闭覆盖层」兜底(→ spec 07 §6.4)。
- [x] `overlay://closed { id: "overlay" }`,新截图前也关闭旧覆盖层。

### 6.6 验收(M6 结束)
- [ ] 框选到显示 ≤4s、颜色采样/文本布局性能。
- [ ] macOS/Windows 真窗口鼠标穿透、不夺焦、Esc 关闭。
- [ ] 多屏混合 DPI/负坐标屏幕对齐 ≤2 逻辑像素。
- [ ] 深浅底文字可读;部分翻译失败与快速重复触发。

---

## M7 · 设置页(W10)

目标:主窗口 UI + 所有配置项完整。**现有设置控件已接通;原生启动/权限/重启验收待做。**

### 7.1 通用 Tab
- [x] 系统/中/英语言,系统/深/浅主题及实时更新(→ spec 09 §2.2)。
- [x] 自启 toggle → LaunchAgent/Windows MSIX StartupTask(无包开发版保留 Run),保存失败回滚(→ spec 10 §3)。
- [x] 启动打开主窗口与真实自动更新检查(→ spec 01 §3、spec 10 §4)。
- [x] 实际监听状态、macOS 权限入口/重新检查;Windows 平台文案。
- [-] 应用内代理已移除(2026-07-07)。

### 7.2 快捷键 Tab
- [x] 显示/录制/预览/清空/冲突反馈和即时重绑;失败保留旧绑定(→ spec 09 §2.4、spec 10 §2)。
- [x] macOS 符号与 Windows Ctrl/Alt/Shift/Win 提示,固定应用内 Cmd/Ctrl+,。

### 7.3 服务 · 文本翻译
- [x] 引擎拖拽排序/启停/增删/设默认,名称/类型/URL/Key/Model/Prompt/Timeout/温度/Token 表单(→ spec 09 §2.3)。
- [x] 测试当前草稿,丢弃过期结果;侧栏仅真实最近测试成功显示绿灯。
- [x] API Key 默认隐藏、可切换显示,已有字段下方明文存储提示满足 MVP 警示(→ spec 01 §6)。
- [x] Apple 翻译桥接;系统版本/平台限制明确显示(→ spec 04 §6.2)。

### 7.4 服务 · OCR
- [x] 腾讯云高精度 OCR:SecretId/SecretKey、通用/多语种场景、置信度、样例测试、F2/F3 服务路由、TC3 签名、脱敏错误和 v7→v8 配置迁移(→ spec 03 §2.6)。
- [x] 2026-10-15 腾讯云接入检查:bindings 已重新生成;TypeScript 检查/前端构建、39 项前端纯逻辑测试、Rust 格式/编译检查及全量 Rust 测试(96 过、10 项按原生环境要求忽略)通过。独立 macOS QA 应用验证新增服务、多语种保存与重启恢复、缺凭据错误及深浅色布局;本轮不调用真实腾讯云 OCR。
- [ ] 腾讯云真实账号样例识别与 F2/F3 覆盖对位验收;需有效云端凭据,Windows 尚未实测。
- [x] 按系统显示后端与真实能力;macOS 快速/精准、置信度、13+ 自动检测持久化(→ spec 03)。
- [x] BCP-47 添加/删除/拖动/箭头排序和优先顺序保存(→ spec 01 §2)。
- [x] Windows 系统语言回退、安装语言枚举和系统安装入口;缺语言包跳转 OCR 页(→ spec 03 §4.1)。

### 7.5 关于 / 搜索
- [x] 实际版本/构建日期/Bundle ID/OCR 后端,GitHub/Release/更新检查入口(→ spec 10 §7)。
- [x] MIT 全文和离线第三方声明,按依赖选择全文;依赖变更刷新生成资源。
- [x] 中英关键词搜索、键盘选择、导航/滚动/聚焦(→ spec 09 §2.2)。

### 7.6 验收(M7 结束)
- [x] typecheck/build 和设置纯逻辑回归测试。
- [x] 隔离 IPC 浏览器验证配置编辑和重载、主要控件交互;不修改真实配置。
- [x] macOS 独立 QA 应用已验证实际 Cmd+,、搜索聚焦、超时保存及重启后主题/OCR 偏好持久化;完整范围见 2026-10-14 设置功能记录。
- [ ] 完整桌面应用重启后所有偏好保存,自启登录、系统授权和快捷键注册实机验收。

---

## M8 · 完善(W11)

目标:主题、i18n、自启、更新等全部打磨到可发布。

### 8.1 国际化
- [ ] 所有 UI 文字抽到语言包(禁止硬编码)(→ spec 09 §6.4)
- [x] `navigator.language` 前缀匹配,zh→zh-CN、en→en-US,其余回退英文(→ spec 09 §6.4)。
- [x] 当前中英语言包各 179 个键且集合一致;新增设置文案已配对。全 UI 硬编码复核仍保留在上项(→ spec 09 §6.4)。

### 8.2 主题
- [x] 共享 `theme.css` 定义深浅色语义变量,`.dark` 与 Tailwind dark selector 已统一(→ spec 09 §6.1)。
- [x] 各窗口通过 `useConfigSync` 同步配置,system 模式由 `matchMedia` 监听系统主题;当前实现不依赖独立 `config://theme-changed` 事件。系统切换实机验收见 §8.6(→ spec 10 §6)。
- [ ] macOS vibrancy / Windows acrylic(popup)(→ spec 09 §3.2)

### 8.3 检查更新
- [x] GitHub Releases API + semver,不依赖未发布的 updater 签名端点(→ spec 10 §4)。
- [x] 启动 3s/24h/重新开启检查,托盘和关于页手动检查。
- [x] 版本与 changelog/浏览器 Release 入口,明确无发布与网络错误反馈。
- [-] updater 插件与 latest.json:当前只在浏览器打开正式 Release,无需此链路。

### 8.4 代理联动(已移除)
- [-] 应用内代理功能已移除(2026-07-07),本节取消(→ spec 04 §7)

### 8.5 日志
- [ ] 日志轮转(5MB × 3 份)(→ spec 11 §5)
- [ ] 禁止记录 API Key / 剪贴板内容(code review 检查点)(→ spec 11 §5)

### 8.6 验收(M8 结束)
- [ ] 中英切换 UI 即时更新(→ spec 09 §7)
- [ ] 系统主题切换后三种窗口(main/popup/overlay)都正确跟随(→ spec 09 §7)
- [ ] 自启开关重启系统后验证生效(→ spec 10 §8)
- [ ] 有新版本弹框,无新版本 toast(→ spec 10 §8)

---

## M9 · 打包发布(W12)

目标:双平台构建 + GitHub Release + 文档。

### 9.1 CI
- [x] `.github/workflows/release.yml`:标签版本校验、双架构 macOS DMG + Windows x64 MSIX,成功后发布 GitHub Release 与 SHA256;分支手动运行可仅构建(→ spec 11 §4.3)。
- [-] macOS universal 单包暂用 Apple Silicon / Intel 两份独立 DMG,覆盖两种架构(→ spec 11 §4.3)。
- [ ] macOS:codesign + notarize + staple(→ spec 11 §4.1)
- [x] Windows:MSIX MakeAppx/SignTool 测试签名构建及安装助手已实现;正式证书和 Windows 实际安装报告待完成(→ spec 11 §4.2)。
- [x] v0.1.0 公开产物:macOS arm64/x64 `.dmg`、Windows x64 `.msix` 和带公开证书/安装助手的 ZIP;ad-hoc/测试签名状态见发布说明,正式可信签名仍待后续(→ spec 11 §3.1)。
- [-] `latest.json` 无需生成:当前使用正式 GitHub Release tag 检查版本(→ spec 10 §4.3)
- [x] v0.1.0 Release notes 手动整理,写明平台、签名和未验收范围(→ docs/releases/v0.1.0.md)。

### 9.2 包体
- [x] Rust release:`opt-level = "s"`、去掉调试信息(→ spec 11 §1.2)。
- [ ] 前端生产构建 minify + treeshake + shadcn/ui 按需加载(→ spec 11 §1.2)
- [x] v0.1.0 包体:arm64 DMG 7.19MB、Intel DMG 7.43MB、MSIX 6.00MB、Windows ZIP 5.95MB,均 ≤ 15MB(→ spec 11 §7)。

### 9.3 文档
- [x] `README.md` 中文主文档已同步 v0.1.0 下载、签名、里程碑、CI 验证范围和开发步骤(→ PRD §8)。
- [ ] English README(→ PRD §8)。
- [ ] 截图/GIF 演示 F1/F2/F3(→ PRD §8)
- [x] README 提供依赖与本地开发命令;Windows MSIX 构建/安装/OCR 自检另有分发指南(→ PRD §8)。
- [ ] Issue / PR 模板(→ PRD §8)
- [ ] 贡献指南(CONTRIBUTING.md)

### 9.4 验收(M9 结束)
- [ ] macOS 通过 Gatekeeper(公证成功,首次双击打开无警告)(→ spec 11 §7)
- [ ] Windows MSIX 签名可信、安装后 OCR 诊断通过;正式证书/签名尚未准备(→ spec 11 §7)。
- [x] 首个 GitHub Release v0.1.0 已公开,`releases/latest` 版本源可用(→ spec 10 §4)。
- [ ] 新版本升级提示端到端验收(需高于已安装版本的后续 Release)。
- [ ] 抓包确认只有翻译 API + GitHub 更新请求,无其他外联(→ spec 11 §7)
- [ ] 冷启动 ≤ 2s;空闲内存 ≤ 150MB(M1/i7 均测)(→ spec 11 §7)

---

## 持续执行(贯穿各里程碑)

### 代码质量
- [x] 主分支 CI 持续运行前端 typecheck/test/build、Rust fmt/全目标 Clippy;macOS 跑 Rust 测试,Windows 编译测试。发布后 CI 已通过,Windows 运行验收另列。
- [x] Rust 侧禁止 `unwrap()` / `expect()` 越出 setup(边界处手工 `map_err`)(→ spec 00 §6)
- [x] 所有 Tauri command 走白名单(capabilities),前端无文件系统直写权限(→ spec 11 §2.2)

### 跨平台验证
- [ ] 每个功能里程碑结束前,双平台各跑一遍核心路径
- [ ] 多屏 + HiDPI 在 M2/M4/M6 结束时做回归(→ spec 02 §8, spec 07 §10)

### 风险观察项 ⚠️
- [x] macOS Vision 可行性 **GO**(headless 识别通过)(→ PRD §6.2)。
- [ ] Windows OCR 实际识别可行性确认;生产实现/原生构建/MSIX 已就绪,仍需客户端安装与识别报告(→ PRD §6.2)。
- [ ] 划词监听权限 / 稳定性(macOS 辅助功能 / Windows UIA)(→ spec 05 §2)
- [ ] 原图覆盖 ESC 快捷键与系统/其他应用冲突(→ spec 07 §11)
- [ ] LLM 批量翻译 JSON 稳定性(多模型回归测试)(→ spec 04 §11)
- [ ] macOS 公证流程(早期跑完整 CI 验证)(→ spec 11 §4.1)
- [ ] Windows `transparent + fullscreen + alwaysOnTop` 组合黑屏(→ spec 02 §9)

### 隐私/安全自检
- [ ] 抓包检查:无非预期外联(→ spec 11 §2.1)
- [ ] 日志检查:无 API Key / 剪贴板内容(→ spec 11 §5)
- [ ] 截图位图不落盘(code review 检查)(→ spec 11 §2)

---

## P2(MVP 之后,不在本文跟踪)

- 弹框操作:收藏、TTS 朗读(→ PRD §7)
- 翻译缓存(→ PRD §7)
- 流式输出(→ PRD §7)
- OCR 引擎可切换(Tesseract / PaddleOCR)(→ PRD §7,spec 03 §10)
- 插件系统(→ PRD §7)
- 多引擎对比(→ PRD §7)
- 导入导出配置(→ PRD §7)
- 词典模式(→ PRD §7)
- 系统 keychain 存储敏感字段(→ spec 01 §6)
- 应用内自动更新(需完整签名链)(→ spec 10 §9)

---

**文档版本**:v0.6(2026-10-14,同步 v0.1.0 公开安装包、主分支双平台 CI 与当前实现;Windows 运行、正式签名和多屏验收单列)
**上游**:[PRD.md](./PRD.md) v0.3 / [spec/](./spec/) v0.2
