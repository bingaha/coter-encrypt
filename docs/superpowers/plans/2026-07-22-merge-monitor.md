# 合并监控与系统通知改造 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 新增「合并监控」（多仓库轮询 opened MR → 白名单 → 盯 AI 评审完成 → 系统通知 + 待办），并将流水线监控的 MessageDialog 结果弹窗改为系统级消息提示（无按钮）。

**Architecture:** 平行复制流水线监控：`merge_monitor.rs` + `MergeMonitorPage.vue` + 独立配置 `yunxiao-merge.json`；共用 `system_notify.rs`（`tauri-plugin-notification`）。已处理/待办仅内存；列表轮询时清理已不在 opened 中的条目。

**Tech Stack:** Tauri 2、Rust（reqwest/tokio/serde）、Vue 3 + Naive UI、`tauri-plugin-notification`、云效 Codeup OpenAPI（`openapi-rdc.aliyuncs.com` + `x-yunxiao-token`）。

**Spec:** `docs/superpowers/specs/2026-07-22-merge-monitor-design.md`

## Global Constraints

- 前端业务调用只用 Tauri `invoke`，不新增业务 HTTP（AGENTS.md）
- 不建状态/临时文件；仅配置落盘 `yunxiao-merge.json`
- 白名单按作者显示名；空白名单 = 不匹配任何人
- 列表间隔与 AI 间隔分开；无自动模式/无模式选项
- 待办仅「浏览器打开」；打开不移除；opened 消失才清内存与待办
- 流水线业务逻辑不变，只换结果通知通道；待办 Web Notification 路径不改
- 系统通知无按钮、不阻塞；失败时禁止回退 MessageDialog

## File Structure

| 文件 | 职责 |
|------|------|
| `src-tauri/src/system_notify.rs` | 共用系统通知封装 |
| `src-tauri/src/merge_monitor.rs` | 合并监控配置、状态机、OpenAPI、命令 |
| `src-tauri/src/pipeline_monitor.rs` | 结果弹窗改为调用 `system_notify` |
| `src-tauri/src/main.rs` | 注册插件/模块/命令/后台；代理保存时同步替换 merge HTTP Client |
| `src-tauri/Cargo.toml` | `tauri-plugin-notification` |
| `src-tauri/capabilities/default.json` | `notification:default` |
| `frontend/src/api/mergeMonitor.js` | invoke 封装 |
| `frontend/src/views/MergeMonitorPage.vue` | 配置/启停/跟踪/待办/日志 |
| `frontend/src/router/index.js` | `/merge-monitor` |
| `frontend/src/views/HomePage.vue` | 卡片 + running/待办徽章 |
| `readme.md` | 简短功能说明（若现有流水线章节旁有文档） |

---

### Task 1: 系统通知封装 + 流水线弹窗改造

**Files:**
- Create: `src-tauri/src/system_notify.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/capabilities/default.json`
- Modify: `src-tauri/src/main.rs`（`mod system_notify` + `.plugin(tauri_plugin_notification::init())`）
- Modify: `src-tauri/src/pipeline_monitor.rs`（替换 `show_system_alert` 实现）

**Interfaces:**
- Produces: `pub fn show_system_notification(app: &AppHandle, title: &str, body: &str)`
- Consumes: `tauri_plugin_notification::NotificationExt`

- [ ] **Step 1: 添加依赖与权限**

`Cargo.toml` dependencies 增加：

```toml
tauri-plugin-notification = "2"
```

`capabilities/default.json` permissions 增加：`"notification:default"`。

- [ ] **Step 2: 实现 `system_notify.rs`**

```rust
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

/// 系统级消息提示（无按钮、不阻塞）。失败只打日志，不回退 Dialog。
pub fn show_system_notification(app: &AppHandle, title: &str, body: &str) {
    if let Err(err) = app
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show()
    {
        eprintln!("[system_notify] failed: {err}; title={title}; body={body}");
    }
}
```

- [ ] **Step 3: 在 `main.rs` 注册**

```rust
mod system_notify;
// Builder 中与 dialog/opener 并列：
.plugin(tauri_plugin_notification::init())
```

- [ ] **Step 4: 改造 `pipeline_monitor.rs`**

1. 删除 `use tauri_plugin_dialog::{DialogExt, MessageDialogKind};`（若本文件无其它 dialog 用途）。
2. 将 `show_system_alert` 改为：

```rust
fn show_system_alert(app: &AppHandle, title: &str, message: &str, _kind: /* 删除 kind 参数更干净 */) {
    crate::system_notify::show_system_notification(app, title, message);
}
```

推荐直接改三个调用点，去掉 `MessageDialogKind`：

- `show_loop_run_failed_alert` → `show_system_notification(app, "循环监控 · 运行失败", &format!(...))`
- `show_single_run_ended_alert` → 同上，标题仍区分失败/结束
- `show_single_aborted_alert` → `"单次监控 · 已停止"`

**不要**改 `maybe_notify_pending` / `pipeline-monitor-notify` 事件路径。

- [ ] **Step 5: 验证编译**

```bash
cd src-tauri && cargo check
```

Expected: 通过，无 `MessageDialogKind` 残留于结果提示路径。

- [ ] **Step 6: Commit（若用户要求提交时再执行）**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/capabilities/default.json \
  src-tauri/src/system_notify.rs src-tauri/src/main.rs src-tauri/src/pipeline_monitor.rs
git commit -m "$(cat <<'EOF'
feat: use system notifications instead of dialogs for pipeline alerts

EOF
)"
```

---

### Task 2: 合并监控配置与状态骨架

**Files:**
- Create: `src-tauri/src/merge_monitor.rs`（本任务只放配置/状态/快照/落盘，不含完整状态机）
- Modify: `src-tauri/src/main.rs`（`mod merge_monitor` + `manage(create_state())`）

**Interfaces:**
- Produces:
  - `MergeMonitorConfig` / `RepoConfigItem`（camelCase）
  - `MergeTodo` / `TrackedMerge` / `MergeSnapshot` / `LogEntry`
  - `MergeMonitorState`：`inner: Mutex<MergeRuntime>` + `loop_started: AtomicBool` + `http: Mutex<Client>`
  - `create_state() -> MergeMonitorState`
  - `replace_http_client(&self, client: Client)`
  - `load_config_from_disk` / `save_config_to_disk`
  - 命令桩：`load_merge_monitor_config` / `save_merge_monitor_config` / `get_merge_monitor_snapshot` / `clear_merge_monitor_logs`

- [ ] **Step 1: 定义配置结构（含 Default 与间隔钳制）**

```rust
const CONFIG_FILE_NAME: &str = "yunxiao-merge.json";
const STATE_EVENT: &str = "merge-monitor-state";
const MAX_LOGS: usize = 200;
const OPENAPI_BASE: &str = "https://openapi-rdc.aliyuncs.com";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoConfigItem {
    pub name: String,
    pub repository_id: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeMonitorConfig {
    pub token: String,
    pub org_id: String,
    pub list_poll_interval_secs: u64, // default 30, clamp >= 5
    pub ai_poll_interval_secs: u64,   // default 10, clamp >= 3
    pub allowed_authors: Vec<String>,
    pub repositories: Vec<RepoConfigItem>,
}
```

落盘路径与流水线相同：`ProjectDirs::from("com", "coter", "CoterEncrypt").config_dir()`。

保存/加载后对间隔做 clamp：`list = list.max(5)`，`ai = ai.max(3)`。

- [ ] **Step 2: 定义运行时与快照**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeTodo {
    pub project_id: i64,
    pub local_id: i64,
    pub author_name: String,
    pub title: String,
    pub repo_name: String,
    pub detail_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackedMerge {
    pub project_id: i64,
    pub local_id: i64,
    pub author_name: String,
    pub title: String,
    pub repo_name: String,
    pub detail_url: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeSnapshot {
    pub running: bool,
    pub todo_count: usize,
    pub current: Option<TrackedMerge>,
    pub todos: Vec<MergeTodo>,
    pub repositories: Vec<RepoStatusView>, // id/name/enabled/summary
    pub logs: Vec<LogEntry>,
}

struct MergeRuntime {
    config: MergeMonitorConfig,
    running: bool,
    current: Option<TrackedMerge>,
    processed: HashSet<(i64, i64)>,
    todos: Vec<MergeTodo>,
    logs: VecDeque<LogEntry>,
}
```

- [ ] **Step 3: `create_state` + HTTP Client（20s，走全局代理）**

对齐 `pipeline_monitor::create_state`：用 `http_client::load`/`build_http_client(20s)`。

- [ ] **Step 4: 实现 load/save/get_snapshot/clear_logs + emit**

`emit_snapshot`：`app.emit("merge-monitor-state", snapshot)`。

- [ ] **Step 5: `cargo check`**

Expected: 通过。

---

### Task 3: 纯函数 — 过滤、AI 判定、清理（先测后码）

**Files:**
- Modify: `src-tauri/src/merge_monitor.rs`（`#[cfg(test)] mod tests`）

**Interfaces:**
- Produces:
  - `fn is_ai_review_complete(comments: &[Value]) -> bool`
  - `fn filter_whitelist_candidates(items, allowed_authors, processed) -> Vec<Candidate>`
  - `fn pick_earliest_by_created_at(candidates) -> Option<Candidate>`
  - `fn cleanup_stale(processed, todos, opened_keys: &HashSet<(i64,i64)>)`

- [x] **Step 1: 写失败测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ai_complete_when_assistant_report_present() {
        let comments = vec![json!({
            "author": {"name": "云效AI助手"},
            "content": "## 🔎 代码评审报告\n..."
        })];
        assert!(is_ai_review_complete(&comments));
    }

    #[test]
    fn ai_incomplete_without_assistant() {
        let comments = vec![json!({
            "author": {"name": "张三"},
            "content": "LGTM"
        })];
        assert!(!is_ai_review_complete(&comments));
    }

    #[test]
    fn empty_whitelist_matches_nobody() {
        let items = vec![dummy_mr("刘锦涛", 1)];
        let out = filter_whitelist_candidates(&items, &[], &HashSet::new());
        assert!(out.is_empty());
    }

    #[test]
    fn pick_earliest_created_at() {
        // older createdAt wins
    }

    #[test]
    fn cleanup_removes_missing_from_processed_and_todos() {
        // opened 不含 (1,24) 时两者都删掉
    }
}
```

- [x] **Step 2: 运行测试确认失败**

```bash
cd src-tauri && cargo test is_ai_review_complete -- --nocapture
```

Expected: 编译失败或 FAIL（函数未实现）。

- [x] **Step 3: 实现最小逻辑**

AI 判定：

```rust
fn is_ai_review_complete(comments: &[Value]) -> bool {
    comments.iter().any(|c| {
        let name = c.pointer("/author/name").and_then(|v| v.as_str()).unwrap_or("");
        let content = c.get("content").and_then(|v| v.as_str()).unwrap_or("");
        name == "云效AI助手" && (content.contains("代码评审报告") || content.contains('🔎'))
    })
}
```

白名单：精确匹配 `author.name`（trim 后相等）。  
排序：解析 `createdAt` 字符串，升序取第一条；解析失败则排后面。  
清理：`processed.retain(|k| opened.contains(k))`；`todos.retain(|t| opened.contains(&(t.project_id, t.local_id)))`。

- [x] **Step 4: 跑通测试**

```bash
cd src-tauri && cargo test merge_monitor::tests -- --nocapture
```

Expected: PASS。

---

### Task 4: OpenAPI 调用 + 状态机后台循环 + 命令

**Files:**
- Modify: `src-tauri/src/merge_monitor.rs`
- Modify: `src-tauri/src/main.rs`（注册全部 merge 命令、`spawn_background`、代理保存时 `replace_http_client`）

**Interfaces:**
- Produces commands:
  - `start_merge_monitor` / `stop_merge_monitor`
  - `open_merge_request_page(detail_url: String)`（内部用 opener，或复用已有 `open_external_url`）
- HTTP:
  - `GET {OPENAPI_BASE}/oapi/v1/codeup/organizations/{org}/changeRequests?projectIds={id}&state=opened`
  - `POST {OPENAPI_BASE}/oapi/v1/codeup/organizations/{org}/repositories/{projectId}/changeRequests/{localId}/comments/list` body `{}`
  - Header: `x-yunxiao-token`

- [ ] **Step 1: 实现 API helpers**

```rust
async fn list_opened_change_requests(http: &Client, token: &str, org: &str, repo_id: &str) -> Result<Vec<Value>, String>;
async fn list_change_request_comments(http: &Client, token: &str, org: &str, project_id: i64, local_id: i64) -> Result<Vec<Value>, String>;
```

单仓失败：返回 Err，调用方打日志并跳过该仓，不停止 `running`。

- [ ] **Step 2: 实现 `run_monitor_cycle` 返回 sleep 秒数**

逻辑严格按规格：

1. `!running` → return 1  
2. 若有 `current`：拉评论 → AI 完成则通知 + todo + processed + clear current → return `list_poll`（或立即 0/1 进入下轮扫列表也可，推荐完成后 `return 1` 快进列表）  
3. 未完成 AI → return `ai_poll_interval_secs`  
4. 无 current：扫所有 enabled 仓 opened → 聚合 keys → `cleanup_stale` → 若 current 不在 opened 则 clear（不通知不进待办）→ 过滤白名单∩未处理 → earliest → set current → return `ai_poll`；无候选 return `list_poll`

通知：

```rust
crate::system_notify::show_system_notification(
    app,
    "合并监控 · AI评审完成",
    &format!("{} · {} · {}", author, title, repo_name),
);
```

- [ ] **Step 3: `spawn_background` + start/stop**

`start`：校验 token/org 非空、至少一条 enabled 仓库；`running=true`。  
`stop`：`running=false`；`current=None`；保留 `processed`/`todos`。

- [ ] **Step 4: `main.rs` 接线**

```rust
.manage(merge_monitor::create_state())
// setup:
merge_monitor::spawn_background(app.handle().clone());
// invoke_handler 增加全部 merge 命令
```

修改 `save_http_proxy_config`：在替换 pipeline client 后，同样：

```rust
let merge_client = http_client::build_http_client(Duration::from_secs(20), &config)?;
merge_state.replace_http_client(merge_client);
```

（可共用同一个 client 实例 clone，或 build 两次；`reqwest::Client` 可 clone。）

- [ ] **Step 5: `cargo test` + `cargo check`**

Expected: 全部通过。

---

### Task 5: 前端 API + 路由 + 监控页

**Files:**
- Create: `frontend/src/api/mergeMonitor.js`
- Create: `frontend/src/views/MergeMonitorPage.vue`
- Modify: `frontend/src/router/index.js`

**Interfaces:**
- Produces JS：`loadMergeMonitorConfig` / `saveMergeMonitorConfig` / `startMergeMonitor` / `stopMergeMonitor` / `getMergeMonitorSnapshot` / `clearMergeMonitorLogs` / `openMergeRequestPage`

- [ ] **Step 1: `mergeMonitor.js`**

```js
import { invokeApi } from './tauriClient'

export const loadMergeMonitorConfig = () => invokeApi('load_merge_monitor_config')
export const saveMergeMonitorConfig = (config) =>
  invokeApi('save_merge_monitor_config', { config })
export const startMergeMonitor = () => invokeApi('start_merge_monitor')
export const stopMergeMonitor = () => invokeApi('stop_merge_monitor')
export const getMergeMonitorSnapshot = () => invokeApi('get_merge_monitor_snapshot')
export const clearMergeMonitorLogs = () => invokeApi('clear_merge_monitor_logs')
export const openMergeRequestPage = (detailUrl) =>
  invokeApi('open_merge_request_page', { detailUrl })
```

- [ ] **Step 2: 路由**

```js
{
  path: '/merge-monitor',
  name: 'MergeMonitorTool',
  component: () => import('../views/MergeMonitorPage.vue') // 或静态 import
}
```

- [ ] **Step 3: 页面（对齐 PipelineMonitorPage 布局，但更简单）**

必须包含：

- 配置：token、orgId、listPollIntervalSecs、aiPollIntervalSecs、allowedAuthors（动态标签/列表）、repositories 表格（name/repositoryId/enabled）
- 按钮：保存、启动、停止、清日志
- **无**自动模式、无 loop/single 模式切换
- 当前跟踪卡片（有则显示作者/标题/仓库）
- 待办列表：每行「打开」→ `openMergeRequestPage(todo.detailUrl)`；打开后列表项仍在
- 日志区
- `listen('merge-monitor-state')` + 约 3s 兜底 `getMergeMonitorSnapshot`

视觉/组件：复用现有 Naive UI 与流水线页样式变量，不新造设计体系。

- [ ] **Step 4: 前端构建检查**

```bash
npm run build
```

Expected: 通过（或项目惯用的 `npm run build:frontend` / vite build；以 `package.json` scripts 为准）。

---

### Task 6: 首页入口 + 文档

**Files:**
- Modify: `frontend/src/views/HomePage.vue`
- Modify: `readme.md`（若存在流水线监控章节，在旁增加合并监控小节）

- [x] **Step 1: 首页工具卡**

在 tools 数组增加：

```js
{
  id: 'merge-monitor',
  title: '合并监控',
  routeName: 'MergeMonitorTool',
  icon: /* 选一个现有 ionicons，如 GitMergeOutline / GitPullRequestOutline */,
  status: '可用',
  description: '监控云效合并请求 AI 评审完成，写入待办并系统通知。',
  capabilities: ['多仓库', '作者白名单', 'AI评审']
}
```

徽章：`running` →「监控中」；`todoCount > 0` → `待办 N`。监听 `merge-monitor-state` + 短轮询，模式对齐流水线卡。

- [x] **Step 2: readme 补充 3～6 行用法**（配置文件名、双间隔、待办仅打开）

- [x] **Step 3: 端到端手工验证清单**（清单见 `.superpowers/sdd/task-6-report.md`；本环境仅完成构建验证，E2E 需人工）

1. 启动应用，打开合并监控，填 Token/Org/仓库/白名单，保存 → 生成 `yunxiao-merge.json`  
2. 启动监控 → 首页徽章「监控中」  
3. 白名单作者提交 opened MR → 日志出现跟踪 → AI 完成后系统通知（非弹窗）且待办出现  
4. 点「打开」→ 浏览器打开详情，待办仍在  
5. MR 合并/关闭后下一轮列表轮询 → 待办与已处理清除  
6. 流水线制造失败/结束 → **系统通知**而非 MessageDialog  
7. 停止合并监控后再启动：同进程内已处理仍跳过仍 opened 的已通知 MR  

---

## Spec Coverage Checklist

| 规格要求 | Task |
|----------|------|
| 系统通知替换 MessageDialog | Task 1 |
| 独立配置 yunxiao-merge.json | Task 2 |
| 双间隔 / 白名单显示名 / 多仓库 | Task 2–4 |
| AI 评论判定 | Task 3–4 |
| 一次只盯最早 createdAt | Task 3–4 |
| 通知+待办+processed；打开不删 | Task 4–5 |
| opened 消失清理内存与待办 | Task 3–4 |
| 无自动/模式选项 | Task 5 |
| 首页入口 | Task 6 |
| 代理热更新 HTTP Client | Task 4 |
| 不落盘状态文件 | Task 2–4（仅配置落盘） |

## Self-Review Notes

- 无 TBD 步骤；AI 判定与规格「暂定」一致并写死在 `is_ai_review_complete`
- `open_merge_request_page` 参数用 `detailUrl`，与前端一致
- 流水线待办 Web Notification **刻意不改**（Task 1 已写明）
