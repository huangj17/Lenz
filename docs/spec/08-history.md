# Spec 08 · 历史记录

> 覆盖 PRD §2.2 F6。定义历史条目结构、存储策略、查询接口与清理逻辑。

---

## 1. 存储位置

- 文件:`{app_config_dir}/history.json`
- 格式:JSON 数组,最新条目在数组末尾。
- 与 `config.json` 相同目录,但独立文件(避免混入配置写入事务)。

## 2. 数据结构

```typescript
interface HistoryItem {
  id: string;          // uuid v4
  timestamp: number;   // Unix 毫秒时间戳
  trigger: "selection" | "screenshot" | "overlay";
  original: string;
  translation: string;
  sourceLang: string | null;
  targetLang: string;
  engineId: string;
  engineName: string;  // 冗余存储,引擎被删除后仍可显示
}
```

不存储:截图图片、OCR 原始数据、bounding box。

## 3. 写入

```rust
// src-tauri/src/history/mod.rs

pub struct HistoryStore {
    items: RwLock<VecDeque<HistoryItem>>,
    max_items: usize,
    dirty: AtomicBool,
}

impl HistoryStore {
    pub fn append(&self, item: HistoryItem);
    pub fn flush(&self) -> Result<(), LenzError>;
}
```

- `append`:写入 `VecDeque` 末尾;若长度超过 `max_items` 则 `pop_front()`。
- `flush`:将当前 `VecDeque` 序列化为 JSON,原子写入(tmp → rename),清除 `dirty` 标志。
- flush 策略:
  - 每次 `append` 后异步 spawn flush(debounce 2s,避免 F3 批量写触发多次 I/O)。
  - 应用退出前同步 flush(在 `on_window_event CloseRequested` 或 `tauri::RunEvent::Exit`)。

## 4. 读取与查询

```rust
#[tauri::command]
async fn list_history(
    store: State<'_, HistoryStore>,
    query: HistoryQuery,
) -> Result<HistoryListResult, LenzError>;

pub struct HistoryQuery {
    pub search: Option<String>,   // 模糊匹配 original + translation
    pub trigger: Option<String>,  // 过滤触发类型
    pub page: u32,
    pub page_size: u32,           // 默认 50
}

pub struct HistoryListResult {
    pub items: Vec<HistoryItem>,
    pub total: u32,
}
```

- 搜索:对 `original` 和 `translation` 字段做**大小写不敏感子串匹配**(MVP,不需全文索引)。
- 排序:始终按 `timestamp` 降序(最新在前)。
- 分页:内存分页,无需 SQL。

## 5. 删除

```rust
#[tauri::command]
async fn delete_history_item(
    store: State<'_, HistoryStore>,
    id: String,
) -> Result<(), LenzError>;

#[tauri::command]
async fn clear_history(store: State<'_, HistoryStore>) -> Result<(), LenzError>;
```

- `delete_history_item`:从 `VecDeque` 中移除对应 id,触发 flush。
- `clear_history`:清空 `VecDeque`,立即 flush。

## 6. 容量管理

- `max_items`:取自 `config.history.maxItems`(默认 500,用户可改)。
- `config://changed` 中 `history.maxItems` 变更 → 若当前条数超出新限制,从队头截断。
- `config.history.enabled = false`:不写入新条目,但已有历史保留(不自动清空)。

## 7. 启动加载

```rust
pub fn load(path: &Path, max_items: usize) -> Result<HistoryStore, LenzError> {
    if !path.exists() { return Ok(HistoryStore::empty(max_items)); }
    let raw = fs::read_to_string(path)?;
    let items: Vec<HistoryItem> = serde_json::from_str(&raw).unwrap_or_default();
    // 取最新 max_items 条
    let deque = items.into_iter().rev().take(max_items).collect::<VecDeque<_>>();
    Ok(HistoryStore { items: RwLock::new(deque), max_items, dirty: AtomicBool::new(false) })
}
```

解析失败时 fallback 为空,不崩溃(损坏文件备份为 `history.json.bak.<timestamp>`)。

## 8. 前端 UI 要点

主窗口"历史记录"标签页:

- 搜索框(实时过滤,debounce 300ms)。
- 触发类型筛选器(全部 / 划词 / 截图 / 原图)。
- 条目列表(虚拟列表,避免大量 DOM 节点):
  - 显示:时间、触发类型图标、原文(单行截断)、译文(单行截断)、引擎名。
  - 点击展开:全文展示,含复制按钮。
  - 右键/悬停:删除单条。
- 底部:条数统计 + "清空全部"按钮(需二次确认)。

## 9. 验收标准

- [ ] F1/F2/F3 触发后均能在历史列表看到对应条目。
- [ ] 历史记录达到 500 条时,新增自动淘汰最旧条目。
- [ ] 搜索"hello"能匹配 original 或 translation 中含该词的条目。
- [ ] 删除单条、清空全部后,重启应用历史不复现。
- [ ] `config.history.enabled = false` 后翻译不写历史,已有历史不丢失。

## 10. 测试点

- 并发写入(F3 触发多条同时 append)不出现数据竞争。
- 历史文件损坏(随机字节)→ 降级为空列表,不崩溃。
- `max_items = 1`:每次翻译后列表始终只有 1 条。
- 搜索含特殊字符(如 `<script>`)不导致 XSS(前端使用 React 默认转义)。
