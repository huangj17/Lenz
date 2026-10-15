# Lenz 技术规格(Spec)索引

> 本目录为 PRD 的技术落地文档,按功能模块拆分。每份 spec 聚焦单一模块,约定接口、数据结构、流程与验收标准。
>
> 上游来源:[`../PRD.md`](../PRD.md)(v0.3,2026-06-15)

## 文档结构

| 编号 | 文档 | 范围 | 对应 PRD |
|---|---|---|---|
| 00 | [总览与架构](./00-architecture.md) | 模块划分、进程/窗口模型、前后端通信约定 | PRD §3 |
| 01 | [配置与存储](./01-config-storage.md) | 配置 schema、持久化路径、读写时序 | PRD §2.2 F4, §3 |
| 02 | [截图与选区](./02-capture.md) | 屏幕截图、全屏选区蒙层、坐标系与 HiDPI | PRD §2.2 F2/F3, §3.4 |
| 03 | [OCR 引擎](./03-ocr.md) | OCR trait、系统/百度/腾讯云 OCR、视觉模型与服务管理 | PRD §2.2 F5, §3.4 |
| 04 | [翻译引擎](./04-translator.md) | Translator trait、OpenAI 兼容、传统 API、多引擎管理 | PRD §2.2 F4, §3.4 |
| 05 | [划词翻译 (F1)](./05-selection-translation.md) | 选中监听、弹框触发、结果展示 | PRD §2.2 F1 |
| 06 | [截图翻译 (F2)](./06-screenshot-translation.md) | 截图 → OCR → 翻译 → 弹框流程 | PRD §2.2 F2 |
| 07 | [原图覆盖翻译 (F3)](./07-overlay-translation.md) | 交互蒙层、选区截图、bbox 定位、×/Esc 关闭 | PRD §2.2 F3 |
| 09 | [窗口与 UI](./09-windows-ui.md) | 主窗口、弹框、选区、覆盖层四类窗口规格 | PRD §2.3, §4 |
| 10 | [系统功能](./10-system-features.md) | 托盘、快捷键、自启、更新、i18n、主题 | PRD §2.3 |
| 11 | [非功能需求](./11-nfr.md) | 性能、隐私、兼容性、打包签名 | PRD §5 |

## 阅读顺序建议

1. 新加入者:按编号顺序通读 00 → 04。
2. 功能开发者:先读 00 与对应功能 spec(05/06/07),再按需查阅 02/03/04。
3. 审阅或验收:查看每份 spec 末尾的 **验收标准** 与 **测试点** 小节。

## 约定

- 所有 Tauri command 命名:`snake_case`,前端调用侧用 `camelCase` wrapper。
- 所有 Rust 类型定义统一在 `src-tauri/src/types.rs`,通过 `specta`/`ts-rs` 导出到前端(或手动保持同步)。
- 坐标系若无特殊说明,一律使用**屏幕逻辑坐标**(points),HiDPI 缩放由各平台 capture 层内部处理。
- 字符串语言代码统一使用 BCP-47(如 `zh-CN`, `en-US`, `ja-JP`)。

## 版本

- v0.2(2026-06-15):移除历史记录(原 08-history),编号 08 留空不复用。
- v0.1(2026-04-24):初版,基于 PRD v0.2 拆分。
