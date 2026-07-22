# 全局 HTTP 代理设置设计

## 背景

CoterEncrypt 桌面应用的业务出站 HTTP 全部由 Rust 侧 `reqwest` 发起（云效流水线监控、OSS Key 互转）。当前两处各自 `Client::builder()`，未暴露代理配置；用户在公司网络或需翻墙访问云效/OSS 时，只能依赖隐式环境变量，无法在应用内选择「直连 / 系统代理 / 指定代理」。

## 目标

1. 提供全局代理设置：`直连` / `系统代理（环境变量）` / `指定代理`
2. 项目中所有出站 HTTP 请求遵从该配置
3. 首页弹窗配置，保存后热更新，无需重启
4. 仅支持 HTTP/HTTPS 代理（不做 SOCKS）
5. 不改变本地 bridge、MySQL、系统浏览器打开链接等非出站 HTTP 路径

## 非目标

- SOCKS5 / 复杂认证 UI（首版 custom URL 可内嵌 `user:pass@host`）
- 保存时强制探测连通性
- 为 MySQL（sqlx）或 `opener` 走 HTTP 代理
- 前端业务 HTTP（本就不存在）

## 方案选择

采用 **共享 HTTP 客户端工厂 + 保存时重建 Client（方案 A）**。

| 模式 | `reqwest` 行为 |
|------|----------------|
| `direct` | `Client::builder().no_proxy()`，忽略环境变量 |
| `system` | 不调用自定义 `.proxy()`，也不 `.no_proxy()`，跟随 `HTTP_PROXY` / `HTTPS_PROXY` / `NO_PROXY` |
| `custom` | `.proxy(Proxy::all(url))`，校验为 `http://` 或 `https://` |

## 设计

### 一、配置文件

路径：与现有配置相同目录（`ProjectDirs::from("com", "coter", "CoterEncrypt").config_dir()`）下的 `http-proxy.json`。

```json
{
  "mode": "system",
  "url": ""
}
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `mode` | string | `direct` \| `system` \| `custom`，默认 `system` |
| `url` | string | `custom` 时必填，如 `http://127.0.0.1:7890`；其他模式可为空 |

缺文件或字段非法时回落默认：`mode=system`，`url=""`。

### 二、Rust 模块

新建 `src-tauri/src/http_client.rs`：

- `HttpProxyMode` / `HttpProxyConfig`（serde）
- `load_http_proxy_config()` / `save_http_proxy_config(config)`
- `build_http_client(timeout: Duration, config: &HttpProxyConfig) -> Result<Client, String>`
- 校验：`custom` 时 URL 非空且 scheme 为 `http`/`https`

共享状态（可放在同模块或 `main` manage）：

- `HttpClientState`：持有当前 `HttpProxyConfig` + 可替换的 `reqwest::Client`（`Mutex` 或等价）
- `reload_http_client(state, config)`：写盘成功后重建默认超时 Client，并通知依赖方替换

默认超时：

- 流水线：20s（保持现状）
- OSS：60s（保持现状；可每次按 timeout 参数从配置重建，不强制与共享 Client 同超时）

推荐接口：

```rust
fn build_http_client(timeout: Duration, config: &HttpProxyConfig) -> Result<Client, String>;
```

OSS 每次 transfer 用 `build_http_client(60s, &current_config)`；流水线后台用共享 Client，保存后整体替换。

### 三、接线出站调用

| 模块 | 改动 |
|------|------|
| `pipeline_monitor.rs` | `create_state` / 后台循环使用的 `http` 改为来自共享状态或工厂；提供 `replace_http_client(client)` 或由 `HttpClientState` 统一持有后注入 |
| `oss_transfer.rs` | 删除本地裸 `Client::builder()`，改为 `build_http_client` + 当前配置 |
| `main.rs` | `mod http_client`；注册 `load_http_proxy_config` / `save_http_proxy_config`；`.manage` 共享状态；`save` 内热更新流水线 Client |

**明确不接线：** `browser_bridge.rs`、`cert_query.rs`（MySQL）、`open_external_url` / `opener`。

### 四、热更新流程

1. 前端调用 `save_http_proxy_config`
2. Rust 校验 → 写 `http-proxy.json`
3. 用新配置 `build_http_client` 重建流水线用 Client，替换 `MonitorState` / 共享状态中的 Client
4. 返回最新配置给前端；后续 OSS / 流水线请求使用新行为

进行中的 in-flight 请求仍使用旧 Client（可接受）；下一轮轮询与新 OSS 调用走新 Client。

### 五、前端 UI

仿 `MysqlDatasourceModal`：

1. `frontend/src/api/httpProxy.js`：`loadHttpProxyConfig` / `saveHttpProxyConfig`
2. `frontend/src/components/HttpProxyModal.vue`：模式单选 + URL 输入（仅 `custom` 可编辑）+ 保存
3. `frontend/src/composables/useHttpProxyConfig.js`（可选，若与 MySQL 模式一致则采用）
4. `HomePage.vue`：顶栏或工具区旁增加「网络代理」按钮，打开弹窗

文案建议：

- 直连：不使用任何代理
- 系统代理：使用环境变量 `HTTP_PROXY` / `HTTPS_PROXY`
- 指定代理：使用下方 HTTP/HTTPS 代理地址

### 六、权限与依赖

- 现有 `reqwest` 已足够支持 HTTP/HTTPS `Proxy::all`，**不新增** socks feature
- capabilities 无需为代理单独扩权（配置读写走现有本地文件模式）

### 七、验证

1. `mode=direct`：即便设置了 `HTTP_PROXY`，云效/OSS 请求仍直连（可用抓包或错误形态对比）
2. `mode=system`：设置环境变量后请求走代理
3. `mode=custom`：填入本地 HTTP 代理地址后请求经该代理；非法 URL 保存失败
4. 保存后不重启，流水线下一轮轮询即反映新模式
5. 本地 bridge / MySQL 行为不变

## 文件清单（预期）

| 路径 | 动作 |
|------|------|
| `src-tauri/src/http_client.rs` | 新建 |
| `src-tauri/src/main.rs` | 注册模块与命令、manage 状态 |
| `src-tauri/src/pipeline_monitor.rs` | 接入可替换 Client |
| `src-tauri/src/oss_transfer.rs` | 改用工厂 |
| `frontend/src/api/httpProxy.js` | 新建 |
| `frontend/src/components/HttpProxyModal.vue` | 新建 |
| `frontend/src/composables/useHttpProxyConfig.js` | 新建（可选） |
| `frontend/src/views/HomePage.vue` | 增加入口 |

## 约束（与 AGENTS.md 一致）

- 仅 Tauri `invoke`，不新增业务 HTTP API / localhost 业务服务
- 不为过渡期增加双运行模式或 HTTP fallback
