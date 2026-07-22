# 合并监控与系统通知改造设计

## 背景

CoterEncrypt 已有「流水线监控」：Rust 后台轮询云效 OpenAPI，Vue 配置页启停与展示。用户需要同类能力监控 Codeup **合并请求（ChangeRequest）**：在白名单作者提交合并后，跟踪其 **AI 评审**是否完成，完成后系统通知，并落入待办供浏览器打开。

此前验证：

- 列表：`GET .../changeRequests?projectIds={repoId}&state=opened`
- AI 完成暂无独立状态字段；评论列表中存在作者「云效AI助手」且内容含评审报告，可判定已完成
- 流水线失败/结束当前用 `MessageDialog` 弹窗（带对话框交互），希望改为系统级通知（Windows Toast / Linux 桌面通知），无按钮

## 目标

1. 新增「合并监控」功能，交互与架构对齐「流水线监控」（方案 1：平行模块，不抽通用框架）
2. 多仓库轮询 `opened` 合并列表；白名单按作者**显示名**匹配；一次只处理 `createdAt` 最早的一条
3. 列表轮询间隔与单条 AI 轮询间隔**分开配置**
4. AI 完成后：系统通知 + 写入待办；待办仅「浏览器打开」；无自动模式、无模式选项
5. 已处理 / 待办仅内存；下次列表轮询若该 MR 已不在 `opened` 中，主动从内存与待办移除
6. 流水线监控业务逻辑不变，仅将原 `MessageDialog` 结果提示改为系统通知（无按钮）
7. 符合 AGENTS.md：Tauri `invoke`，无业务 HTTP

## 非目标

- 人工评审通过/拒绝、合并 MR、触发 AI 评审等写操作
- 单次监控模式、自动模式
- 已处理/待办落盘或任何状态/临时文件
- 抽取跨监控的通用框架（本次不做方案 2）
- 改造流水线「待办」原有 Web Notification / 页内提示链路（仅改原先弹窗类结果提示）

## 方案选择

采用 **平行复制流水线监控结构（方案 1）**：

| 项 | 流水线监控 | 合并监控 |
|----|------------|----------|
| Rust | `pipeline_monitor.rs` | `merge_monitor.rs` |
| 配置 | `yunxiao-pipeline.json` | `yunxiao-merge.json` |
| 前端页 | `PipelineMonitorPage.vue` | `MergeMonitorPage.vue` |
| 路由 | `/pipeline-monitor` | `/merge-monitor` |
| 通知 | 结果提示改系统通知 | AI 完成用同一封装 |

## 设计

### 一、配置文件

路径：`ProjectDirs::from("com", "coter", "CoterEncrypt").config_dir()/yunxiao-merge.json`。

```json
{
  "token": "",
  "orgId": "",
  "listPollIntervalSecs": 30,
  "aiPollIntervalSecs": 10,
  "allowedAuthors": ["刘锦涛"],
  "repositories": [
    {
      "name": "platform-crawler",
      "repositoryId": "5284818",
      "enabled": true
    }
  ]
}
```

| 字段 | 类型 | 说明 | 默认 |
|------|------|------|------|
| `token` | string | 云效个人访问令牌 | `""` |
| `orgId` | string | 组织 ID | `""` |
| `listPollIntervalSecs` | u64 | 扫合并列表间隔（秒） | `30`（下限 5） |
| `aiPollIntervalSecs` | u64 | 盯单条 AI 完成间隔（秒） | `10`（下限 3） |
| `allowedAuthors` | string[] | 作者显示名白名单；**空列表 = 不匹配任何人** | `[]` |
| `repositories` | object[] | `name` / `repositoryId` / `enabled` | `[]` |

缺文件或非法字段时回落上表默认值。HTTP 出站复用全局代理配置（与流水线一致，保存代理后替换 Client）。

**不落盘**：当前跟踪 MR、已处理集合、待办列表、运行日志（内存环形缓冲）。

### 二、Rust 模块与状态机

新建 `src-tauri/src/merge_monitor.rs`，`main.rs` 注册命令并 `spawn_background`。

#### 运行开关

- `stopped`：后台空转约 1s，不调业务 API
- `running`：按状态机轮询（无 loop/single/auto 模式选项）

#### 内存结构

- `processed: HashSet<(project_id, local_id)>`：本进程内不再作为「新任务」选中
- `todos: Vec<MergeTodo>`：待办事项（可多条）
- `current: Option<TrackedMerge>`：当前正在盯 AI 的那一条（全局最多 1 条）
- `logs: VecDeque<LogEntry>`：内存日志

`MergeTodo` 至少包含：`projectId`、`localId`、`authorName`、`title`、`repoName`、`detailUrl`。

#### 状态机

```text
stopped
   │ start
   ▼
扫列表 ──(无候选)──► sleep(listPollIntervalSecs) ──► 扫列表
   │
   │ 有候选：白名单 ∩ 非 processed，按 createdAt 升序取最早
   ▼
跟踪 AI ──(未完成)──► sleep(aiPollIntervalSecs) ──► 跟踪 AI
   │
   │ AI 完成（见判定）
   ▼
系统通知 + 写入 todos + 写入 processed + 清空 current
   │
   ▼
回到扫列表
```

**扫列表细节**

1. 对每个 `enabled` 仓库：`GET /oapi/v1/codeup/organizations/{orgId}/changeRequests?projectIds={repositoryId}&state=opened`
2. 合并结果；过滤 `author.name` ∈ `allowedAuthors` 且 `(projectId, localId)` ∉ `processed`
3. 同时做 **清理**：对每个已在 `processed` / `todos` 中的条目，若本次所有仓库的 `opened` 聚合结果中已不存在 → 从 `processed` 与 `todos` **同时移除**（与是否点击「打开」无关）
4. 若存在 `current` 且该 MR 已不在 `opened` 中 → 清空 `current`（不通知；若尚未写入 processed 可视情况写入或不写入——约定：跟踪中途消失则仅清空 current，不进待办）
5. 无 `current` 且有候选 → 取 `createdAt` 最早一条进入跟踪

**跟踪 AI 细节**

1. `POST .../repositories/{projectId}/changeRequests/{localId}/comments/list`，body 可用 `{}` 或按类型过滤
2. **完成判定（暂定）**：存在评论满足  
   - `author.name == "云效AI助手"`（或等价 AI 助手身份）  
   - 且 `content` 含「代码评审报告」或「🔎」（与已观测报告格式一致）  
3. 未完成：按 `aiPollIntervalSecs` 继续盯  
4. 完成：系统通知（标题如「合并监控 · AI评审完成」，正文 `作者 · 标题 · 仓库名`）→ push 待办 → insert processed → clear current → 回扫列表  
5. API 失败：打日志，不停止 `running`，下轮重试

**停止**：`running=false`，清空 `current`；**保留**本进程内 `processed` 与 `todos`（避免同进程重复吵；列表清理规则仍在下次 running 扫列表时生效）。

### 三、Tauri 命令与事件

| 命令 | 作用 |
|------|------|
| `load_merge_monitor_config` | 读配置 |
| `save_merge_monitor_config` | 写盘 + 热更新内存配置 |
| `start_merge_monitor` | 启动 |
| `stop_merge_monitor` | 停止 |
| `get_merge_monitor_snapshot` | 快照 |
| `clear_merge_monitor_logs` | 清日志 |
| `open_merge_request_page` | 用系统浏览器打开待办/当前 MR 的 `detailUrl`（可复用 `open_external_url`） |

事件：`merge-monitor-state` → 推送快照（`running`、`current`、`todos`、`pipelines` 式仓库状态摘要、`logs` 等）。

前端兜底轮询快照间隔可对齐流水线页（约 3s）。

### 四、前端

1. **路由** `/merge-monitor`，`name: MergeMonitorTool`
2. **首页** `HomePage.vue` 增加「合并监控」卡片（可显示 running 徽章 / 待办数量）
3. **页面** `MergeMonitorPage.vue`：配置区（Token、Org、双间隔、白名单、仓库表）、启停、当前跟踪、待办列表（仅「打开」）、日志
4. **API** `frontend/src/api/mergeMonitor.js` + `invokeApi`

待办行操作：**仅**「打开」（浏览器）；无通过/拒绝/稍后等按钮。打开**不**移除待办。

### 五、弹窗改为系统消息通知（流水线 + 合并共用）

本需求的**另一半**是通知形态改造，与「合并监控」同等重要。

#### 5.1 现状（要改掉的）

流水线监控在 Rust 侧通过 `show_system_alert` → `tauri_plugin_dialog::MessageDialog` 弹出**模态对话框**：

| 场景（流水线） | 当前表现 |
|----------------|----------|
| 循环监控 · 运行失败 | 弹窗（Error） |
| 单次监控 · 运行结束 / 失败 | 弹窗 |
| 单次监控 · 已停止 | 弹窗（Warning） |

问题：Windows / Linux 都是对话框，需用户点掉；不符合「只提示、不打断」的期望。

#### 5.2 目标形态

| 项 | 约定 |
|----|------|
| 形态 | **系统级消息提示**（非应用内 Modal） |
| Windows | 右下角 Toast / 操作中心通知 |
| Linux | 桌面通知（如 GNOME Notification） |
| 交互 | **无按钮**、不阻塞主流程、无需用户点确认 |
| 内容 | 仅 `title` + `body` 文本 |

#### 5.3 实现方式

1. 引入 `tauri-plugin-notification`
2. 新建共用封装，例如 `src-tauri/src/system_notify.rs`：

```text
show_system_notification(app, title, body)
```

- 内部走插件 Notification API
- **不**再使用 `MessageDialog` / `dialog().message(...).show(...)` 做结果提示
- 不附加 action button

3. `capabilities/default.json` 增加 notification 相关权限；按平台做最小授权配置

4. **流水线改造点**（业务逻辑不变，只换通知通道）：
   - 删除或停用 `pipeline_monitor.rs` 中面向结果提示的 `show_system_alert`（MessageDialog）
   - 上述失败 / 结束 / 停止场景全部改为 `show_system_notification`
   - 轮询、白名单、待办审批、`auto_mode`、卡点处理等**一律不改**

5. **合并监控**：
   - AI 评审完成时调用同一 `show_system_notification`
   - 标题示例：`合并监控 · AI评审完成`
   - 正文示例：`刘锦涛 · feat: xxx · platform-crawler`

#### 5.4 明确不改的通知路径

| 路径 | 本次是否改 |
|------|------------|
| 流水线 `MessageDialog` 结果弹窗 | **改** → 系统消息 |
| 流水线待办 `pipeline-monitor-notify` + 前端 Web `Notification` / `n-message` | **不改**（本来就不是 Dialog 弹窗） |
| 合并监控 AI 完成 | **用**系统消息（新建） |

#### 5.5 降级

若系统未授予通知权限或发送失败：写日志；可选前端 `message` 轻提示；**禁止**回退为阻塞式 MessageDialog。

### 六、错误处理与测试要点

- Token/Org 缺失：启动失败或运行中打错误日志，不崩溃
- 单仓库 API 失败：跳过该仓，其它仓继续；日志记录
- 白名单为空：永不进入跟踪（符合「空=不匹配」）
- 验证：启停、双间隔分别生效、AI 完成后待办与通知、列表消失后清理、打开不删待办、流水线结果改为系统通知且无按钮

## 架构关系

```text
HomePage ──► MergeMonitorPage
                 │ invoke / listen
                 ▼
           merge_monitor.rs
                 │
     ListChangeRequests / comments/list
                 │
           system_notify  ◄── pipeline_monitor（结果提示改造）
```

## 已确认产品约定摘要

| 项 | 约定 |
|----|------|
| 白名单 | 作者显示名 |
| 仓库 | 可配置多个，合并列表后取最早 |
| 排序 | `createdAt` 升序 |
| AI 完成 | 暂定：云效AI助手评论含评审报告 |
| 配置 | 完全独立 `yunxiao-merge.json` |
| 已处理 | 仅内存；列表中消失则清理 |
| 待办 | AI 完成后写入；仅打开浏览器；打开不删除；列表中消失则清理 |
| 模式 | 无；仅启停 |
| 间隔 | `listPollIntervalSecs` / `aiPollIntervalSecs` |
| 流水线通知 | MessageDialog → 系统通知，无按钮 |
