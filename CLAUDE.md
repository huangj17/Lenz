# Lenz

基于 Tauri 2 的跨平台(macOS + Windows)截图翻译桌面工具:划词翻译、截图翻译、原图覆盖翻译,主打本地 LLM + 隐私友好。技术栈 Tauri 2 + React 19 + TypeScript + Vite + Rust。

## 当前状态

里程碑 **M1(项目搭建)脚手架已完成**:Tauri 2 + React 19 + Vite 7 工程就位,`pnpm tauri dev` 可启动(已在 macOS 跑通:菜单栏托盘 app、全局快捷键注册 Alt+D/S/F、主窗口)。已落地配置体系(`ConfigStore`:原子写 + RFC7396 merge patch + `config://changed`)、托盘菜单、启动时序、tauri-specta 类型导出。OCR / 截图 / 翻译 / 划词 / 覆盖等命令目前均为占位(返回 `not_implemented`),留待 M2 起逐里程碑实现。下一步见 `docs/TODO.md` M2(OCR 桥接)。

## 文档导览(`docs/`)

- `PRD.md` — 产品需求,**产品定义的唯一权威来源**。
- `spec/` — 技术规格,按模块编号 `00`~`11`;`README.md` 是索引。注意 **`08`(历史记录)已移除,编号留空不复用**。
- `TODO.md` — 按里程碑 M0~M9 拆分的任务清单,每项 `(→ spec XX §Y)` 指向对应规格。

三者自上而下:PRD → spec → TODO。**改动需求时三处需保持同步**,尤其 spec/TODO 内部大量使用「spec NN §M」「§X.Y」形式的交叉引用,删改模块时务必一并更新引用与章节号。

## 设计文件(`docs/design/`)⚠️ 重要

这是从 **Claude Design(claude.ai/design)导出的设计交接包**,不是项目源码:

- 设计媒介是 HTML/CSS/JS(JSX)**原型**,主入口 `docs/design/project/index.html`,各界面在 `docs/design/project/src/*.jsx`(`popup-f1`/`popup-f2`/`capture-f2`/`overlay-f3`/`settings`/`tray`/`welcome` 等)。
- 实现时**按视觉效果用 Tauri/React 重建**,不要照搬原型的内部结构。
- `docs/design/project/uploads/` 是用户上传的素材快照(含一份**旧版 `PRD.md`** 和参考图)——这些是**只读快照,不要当作需维护/同步的源文档**。产品定义一律以 `docs/PRD.md` 为准。

## 开发

- 包管理 `pnpm`;启动 `pnpm tauri dev`;前端类型检查 `pnpm typecheck`;Rust 检查 `cargo check --manifest-path src-tauri/Cargo.toml`。
- 前端单 HTML 入口按 `?window=` 路由到 `src/windows/{main,popup,selection,overlay}`;`main`+`popup` 为静态窗口(`tauri.conf.json`),`selection`/`overlay` 运行时创建。
- **前后端类型契约**:Rust 侧 `#[specta::specta]` 命令 + `cargo test export_bindings` 生成 `src/lib/bindings.ts`(真源)。当前前端另有手写 `src/lib/types.ts` 作镜像契约,二者须同步;线缆格式统一 camelCase。
- ⚠️ **不要在 Tauri 命令签名里直接用 `serde_json::Value`**:specta(rc 版本)导出它会无限递归、运行时爆栈。需要传任意 JSON 时,线缆层改用 `String`(前端 `JSON.stringify`、后端 `serde_json::from_str`),参考 `set_config`。
- `bindings.ts` 中带 `_Serialize` / `_Deserialize` 后缀的类型,源于字段上的 `#[serde(skip_serializing_if)]`(序列化与反序列化形态不同);前端若要直接复用 bindings,需处理或去掉这些 skip 属性。
- 依赖版本不写死具体 patch:Cargo/npm 均用语义化范围(caret),取最新兼容版本。

## 约定

- 与用户对话使用中文(全局已设)。
