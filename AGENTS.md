# Lenz 开发指南

本文件适用于整个仓库。与用户沟通使用中文，代码标识符沿用现有英文命名。

## 项目定位与工作入口

Lenz 是面向 macOS / Windows 的桌面翻译工具，核心场景为划词翻译、截图翻译和原图覆盖翻译，主打本地 OCR 与 OpenAI 兼容模型接入。技术栈为 Tauri 2 + Rust、React 19 + TypeScript、Vite、Tailwind CSS、Zustand、i18next。

- 开始修改前检查工作区状态，阅读目标模块及对应规格，保留用户已有改动；避免顺带重构无关模块。
- 产品定义以 `docs/PRD.md` 为准；技术规格入口为 `docs/spec/README.md`；进度与验收记录见 `docs/TODO.md`。
- 文档中的完成状态、旧注释和占位说明可能滞后，判断当前能力时应核对实际代码与验证记录，不把规划当成已实现功能。
- 需求变更时同步 PRD、相关 spec 和 TODO，并检查章节交叉引用；普通修复只更新受影响的文档。
- `docs/design/project/` 是只读视觉参考，按视觉效果在生产代码中实现；其中 `uploads/PRD.md` 是旧版快照，不作为需求依据或同步目标。

## 目录与职责

| 路径 | 职责 |
|---|---|
| `src/windows/` | `main` 设置页、`popup` 翻译弹窗、`selection` 选区、`overlay` 覆盖层、`indicator` 划词图标；入口路由在 `src/App.tsx` |
| `src/components/`、`src/hooks/` | 共享组件与 hooks |
| `src/stores/` | 配置与翻译会话的前端状态 |
| `src/lib/` | IPC 封装、生成类型、前端类型别名、默认配置与工具函数 |
| `src/locales/`、`src/styles/theme.css` | 中英界面文案与主题样式 |
| `src-tauri/src/commands/` | Tauri 命令入口；业务逻辑放入 `core/` |
| `src-tauri/src/core/` | 截图、OCR、翻译、划词监听、窗口与快捷键逻辑 |
| `src-tauri/src/config/` | 配置 schema、默认值、迁移与持久化 |
| `src-tauri/src/lib.rs`、`setup.rs`、`tray.rs` | 命令与类型注册、启动接线、托盘 |
| `src-tauri/tauri.conf.json`、`src-tauri/capabilities/` | 窗口与安全能力配置 |
| `tests/` | 前端纯逻辑测试；Rust 单测位于实现旁的 `#[cfg(test)]` 模块 |
| `src/assets/`、`src-tauri/icons/` | 界面素材与应用图标 |

## 实现约定

### 代码与界面

- TypeScript 使用两空格缩进、双引号、分号；组件使用 `PascalCase`，hooks 使用 `useCamelCase`，工具与 store 使用 `camelCase`。优先使用 `@/` 导入别名，保持严格类型检查通过。
- Rust 使用 `cargo fmt` 格式化，模块与函数用 `snake_case`，类型用 `PascalCase`；沿用 `LenzError` 处理可恢复错误。
- 复用现有组件、主题变量与 IPC 封装。新增用户可见文案同步维护 `src/locales/zh-CN.json` 和 `en-US.json`，检查深浅色主题。
- 平台专用代码使用条件编译隔离。修改窗口交互时关注焦点、透明背景、置顶、关闭/隐藏及事件监听清理，避免影响其他窗口。

### IPC 与类型

- Rust 命令及共享类型是契约源头；新增命令在 `src-tauri/src/lib.rs` 的 `specta_builder()` 中注册，事件载荷类型按现有方式显式导出。
- 修改契约后重新生成 `src/lib/bindings.ts`，禁止手工编辑生成文件。`src/lib/types.ts` 只维护前端别名、必要的类型收窄和本地类型，不复制 Rust schema。
- 同步检查 `src/lib/ipc.ts` 的参数、返回值、事件名称与载荷。命令名使用 `snake_case`，前端 wrapper 和序列化字段使用 `camelCase`。
- 避免在 Specta 暴露的命令签名中使用 `serde_json::Value`，当前导出链路存在递归风险。任意 JSON 使用字符串承载并在后端解析，参考 `set_config`。

### 配置与桌面流程

- 配置持久化通过 `ConfigStore`，保留原子写入、损坏备份和 `config://changed` 同步机制；前端通过现有配置 API/store 更新。
- 修改配置结构或默认值时，检查 `config/schema.rs`、`config/migrate.rs`、`src/lib/default-config.ts` 及消费方，兼容已有用户配置；内置引擎预设也需保持前后端一致。
- 截图与 OCR 变更需明确区分屏幕逻辑坐标、物理像素与 OCR 归一化坐标，复用坐标转换函数，并考虑负坐标、多屏和不同缩放比例。
- 修改异步翻译流程时保留取消与会话有效性检查，避免旧结果覆盖新请求；截图前确保选区蒙层已离屏。
- 不提交 API Key、`.env` 或本地翻译凭据；日志和测试输出不要暴露密钥或用户选中的敏感内容。

## 常用命令

以下命令从仓库根目录执行，JavaScript 依赖统一使用 pnpm。

| 命令 | 用途 |
|---|---|
| `pnpm install` | 安装前端依赖 |
| `pnpm tauri dev` | 启动完整桌面应用 |
| `pnpm dev` | 仅启动 Vite 前端，不能验证原生 IPC、截图或系统权限 |
| `pnpm typecheck` | TypeScript 类型检查 |
| `pnpm build` | TypeScript 检查与前端生产构建，不是桌面打包 |
| `pnpm test` | Node 内置测试运行器执行快捷键、异步队列、会话和绘制调度纯逻辑测试，实际范围以 `package.json` 为准 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml` | 格式化 Rust |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 检查当前平台 Rust 编译 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 运行 Rust 测试，包括类型导出测试 |
| `cargo test export_bindings --manifest-path src-tauri/Cargo.toml` | 重新生成前端 bindings |

## 按改动选择验证

- **仅文档**：检查内容、路径、命令与差异，无需运行应用构建。
- **前端/UI**：运行 `pnpm typecheck`、`pnpm build`；涉及快捷键逻辑时运行 `pnpm test`。新增测试时检查测试脚本是否包含该文件。
- **Rust 逻辑**：运行格式检查（`cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`）、`cargo check` 和受影响的测试；公共核心逻辑变更运行完整 Rust 测试。
- **IPC/配置契约**：先生成 bindings，再执行前端检查；为配置迁移、翻译适配、坐标转换等纯逻辑修复补充有针对性的回归测试。
- **桌面交互**：在受影响的 Tauri 窗口中验证。按改动范围检查辅助功能/屏幕录制权限、选区取消、焦点恢复、剪贴板、连续请求和多屏缩放。
- 当前没有 React 组件测试框架；纯逻辑测试不能代替窗口验收。macOS 验证通过不代表 Windows 已验证，未实测的平台与被环境阻塞的检查应明确列出。
- 完成后检查 diff，确认生成文件及 lockfile 变化与任务相关；汇报修改结果、实际执行的检查及剩余限制，不把未运行的检查标为通过。

## 依赖文档查询

涉及库、框架、SDK、API、CLI 或云服务的用法、配置、迁移和专有问题时，使用 Context7 获取当前文档，不仅依赖记忆：

1. 先用 `resolve-library-id` 提供库名与具体查询目标；用户已给出精确 library ID 时可跳过。
2. 按名称匹配、相关性和来源可信度选择 ID，指定版本时优先使用对应版本。
3. 使用 `query-docs` 查询具体概念，不相关的概念分开查询；依据返回文档实施或回答。

纯业务逻辑调试、代码审查、常规重构和仓库文件核对无需查询。Context7 不可用时说明限制，必要时使用官方文档，不声称已查证。

## 提交与 PR

沿用 Conventional Commits，例如 `feat(popup): add language swap`、`fix(capture): hide selection overlay`，保持提交聚焦。PR 说明用户可见变化、相关 issue/spec、验证命令与实际平台；UI 变更附截图或录屏，并说明新增权限或配置。
