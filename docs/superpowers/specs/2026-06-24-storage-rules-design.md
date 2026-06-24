# 存储写入规则功能设计

## 背景

当前"在线凭证查询"功能仅支持从 `cert_info` 的 `cookies`/`cookie` 字段读取 Cookie 数组，并通过浏览器扩展写入浏览器 Cookie。

部分城市（如济南）使用 token 认证机制，认证信息存储在 `sessionStorage` 或 `localStorage` 中，而非 Cookie。需要扩展系统以支持这些场景。

## 目标

1. 支持从 `cert_info` JSON 的任意字段（包括嵌套路径）取值
2. 支持写入浏览器的 `sessionStorage` 和 `localStorage`
3. 支持固定值写入（如 `{}`）
4. 按地区 + 业务类型配置不同的写入规则
5. 提供前端 UI 用于配置和管理规则
6. 保持现有 Cookie 逻辑不变

## 设计

### 一、配置格式

扩展 `website-url-mappings.json`，每个映射条目可选 `storageRules` 字段。

**完整配置示例：**

```json
[
 {
 "areaId": "05310",
 "businessType": "公积金",
 "url": "https://wt.jngjj.net/home",
 "storageRules": [
 {
 "storage": "sessionStorage",
 "key": "token",
 "source": { "path": "token" }
 },
 {
 "storage": "localStorage",
 "key": "redux",
 "source": { "value": "{}" }
 }
 ]
 },
 {
 "areaId": "289",
 "businessType": "社保",
 "url": "https://sbwx.rst.shanxi.gov.cn:8007/ylwxsb/index.shtml"
 }
]
```

**字段说明：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `storage` | string | 是 | 存储类型：`"sessionStorage"` 或 `"localStorage"` |
| `key` | string | 是 | 写入的键名 |
| `source` | object | 是 | 值来源，`path` 和 `value` 二选一 |
| `source.path` | string | 否 | 从 cert_info 取值的路径，支持嵌套如 `"data.token"` |
| `source.value` | string | 否 | 固定值 |

### 二、前端 UI 设计

#### 2.1 移除双击配置 URL 功能

当前逻辑：双击"默认浏览器打开"按钮会提示"再次点击可配置首页地址"。此交互将被移除。

网站地址配置移至查询结果的配置面板中。

#### 2.2 查询结果配置面板

每个查询结果条目新增配置区域：

```
┌──────────────────────────────────────────────────────────────┐
│ 公司名称 [复制cert_info] [默认浏览器打开] │
│ login_key: xxx 办理类型: 公积金 area_id: 05310 │
│ │
│ ┌─ 网站地址 ─────────────────────────────────────────────────┐ │
│ │ 登录地址: [https://wt.jngjj.net/home ] [保存] │ │
│ └────────────────────────────────────────────────────────────┘ │
│ │
│ ┌─ 存储写入规则 ─────────────────────────────────────────────┐ │
│ │ [+ 添加规则] │ │
│ │ │ │
│ │ ┌─ 规则 1 ──────────────────────────────────────────────┐ │ │
│ │ │ 存储类型: [sessionStorage ▼] │ │ │
│ │ │ 键名: [token ] │ │ │
│ │ │ 来源类型: [从cert_info取值 ▼] │ │ │
│ │ │ 路径: [token ] │ │ │
│ │ │ [删除] [上移] [下移] │ │ │
│ │ └────────────────────────────────────────────────────────┘ │ │
│ │ │ │
│ │ ┌─ 规则 2 ──────────────────────────────────────────────┐ │ │
│ │ │ 存储类型: [localStorage ▼] │ │ │
│ │ │ 键名: [redux ] │ │ │
│ │ │ 来源类型: [固定值 ▼] │ │ │
│ │ │ 值: [{} ] │ │ │
│ │ │ [删除] [上移] [下移] │ │ │
│ │ └────────────────────────────────────────────────────────┘ │ │
│ │ │ │
│ │ [保存规则] │ │
│ └────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

#### 2.3 规则配置项

每条规则包含：

- **存储类型**：下拉选择 `sessionStorage` / `localStorage`
- **键名**：文本输入
- **来源类型**：下拉选择
 - `从 cert_info 取值`：显示路径输入框
 - `固定值`：显示值输入框

#### 2.4 数据流

1. 查询时自动加载该地区已有的 `storageRules` 配置
2. 用户可增删改规则
3. 点击"保存规则"写入 `website-url-mappings.json`
4. 点击"默认浏览器打开"时，规则随配置一起传递给后端

### 三、Rust 端改动

#### 3.1 数据结构扩展

```rust
// 扩展现有结构体
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebsiteUrlMapping {
 pub area_id: String,
 pub business_type: String,
 pub url: String,
 #[serde(default, skip_serializing_if = "Option::is_none")]
 pub storage_rules: Option<Vec<StorageRule>>,
}

// 新增结构体
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageRule {
 pub storage: String, // "sessionStorage" | "localStorage"
 pub key: String,
 pub source: StorageSource,
}

// 值来源：path 和 value 二选一
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageSource {
 // 从 cert_info 取值的路径，支持嵌套如 "data.token"
 #[serde(default, skip_serializing_if = "Option::is_none")]
 pub path: Option<String>,
 // 固定值（与 path 二选一）
 #[serde(default, skip_serializing_if = "Option::is_none")]
 pub value: Option<String>,
}

// 传递给浏览器扩展的结构体（已解析的值）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageItem {
 pub storage: String,
 pub key: String,
 pub value: String,
}
```

#### 3.2 值提取逻辑

```rust
fn extract_storage_value(cert_info: &str, source: &StorageSource) -> Result<String, String> {
 // 优先使用 path，其次使用 value
 if let Some(path) = source.path.as_deref() {
 let path = path.trim();
 if path.is_empty() {
 return Err("source.path 不能为空".to_string());
 }
 
 let value: serde_json::Value = serde_json::from_str(cert_info)
 .map_err(|e| format!("cert_info 不是合法 JSON: {e}"))?;
 
 let parts: Vec<&str> = path.split('.').collect();
 let mut current = &value;
 
 for part in parts {
 current = current.get(part)
 .ok_or_else(|| format!("cert_info 中未找到字段: {path}"))?;
 }
 
 match current {
 serde_json::Value::String(s) => Ok(s.clone()),
 other => Ok(other.to_string()),
 }
 } else if let Some(value) = source.value.as_deref() {
 Ok(value.to_string())
 } else {
 Err("source 必须指定 path 或 value".to_string())
 }
}
```

#### 3.3 修改 open_default_browser_with_cookies

扩展请求和响应结构，包含存储写入信息：

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenWithCookiesRequest {
 pub area_id: String,
 pub business_type: String,
 pub cert_info: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgePayload {
 pub r#type: String,
 pub request_id: String,
 pub target_url: String,
 pub cookies: Vec<BridgeCookie>,
 #[serde(default, skip_serializing_if = "Vec::is_empty")]
 pub storage_items: Vec<StorageItem>,
}
```

逻辑流程：
1. 解析 `cert_info` 提取 Cookie（现有逻辑）
2. 读取 `storageRules` 配置
3. 根据规则从 `cert_info` 提取值
4. 如果任何规则的 `path` 字段不存在，返回错误
5. 将 Cookie 和 StorageItem 一起发送给浏览器扩展

### 四、浏览器扩展改动

#### 4.1 manifest.json

添加权限：

```json
{
 "permissions": ["cookies", "tabs", "scripting"],
 "host_permissions": ["<all_urls>"]
}
```

#### 4.2 background.js 流程改造

```javascript
async function writeCookiesAndOpen(payload, sender) {
 const { targetUrl, cookies, storageItems } = payload;
 
 // 1. 清除并写入 Cookie（现有逻辑）
 await clearCookiesForTarget(targetUrl);
 for (const cookie of cookies) {
 await setCookie(targetUrl, cookie);
 }
 
 // 2. 打开目标页面
 const tab = await chrome.tabs.create({ url: targetUrl });
 
 // 3. 等待页面加载完成
 await waitForTabLoad(tab.id);
 
 // 4. 注入脚本写入 sessionStorage/localStorage
 if (storageItems && storageItems.length > 0) {
 await chrome.scripting.executeScript({
 target: { tabId: tab.id },
 func: injectStorage,
 args: [storageItems]
 });
 
 // 5. 刷新页面，让页面读取预设的存储值
 await chrome.tabs.reload(tab.id);
 }
}

// 注入到页面的函数
function injectStorage(items) {
 for (const item of items) {
 if (item.storage === 'sessionStorage') {
 sessionStorage.setItem(item.key, item.value);
 } else if (item.storage === 'localStorage') {
 localStorage.setItem(item.key, item.value);
 }
 }
}

// 等待标签页加载完成
function waitForTabLoad(tabId) {
 return new Promise((resolve) => {
 chrome.tabs.onUpdated.addListener(function listener(updatedTabId, info) {
 if (updatedTabId === tabId && info.status === 'complete') {
 chrome.tabs.onUpdated.removeListener(listener);
 resolve();
 }
 });
 });
}
```

#### 4.3 WebSocket 消息格式

传递给扩展的消息格式：

```json
{
 "type": "openWithCookies",
 "requestId": "abc123",
 "targetUrl": "https://wt.jngjj.net/home",
 "cookies": [
 { "name": "SESSION", "value": "abc", "path": "/", "secure": true }
 ],
 "storageItems": [
 { "storage": "sessionStorage", "key": "token", "value": "eyJ..." },
 { "storage": "localStorage", "key": "redux", "value": "{}" }
 ]
}
```

### 五、错误处理

| 场景 | 处理方式 |
|------|----------|
| `source.path` 指定的字段不存在 | 返回错误：`cert_info 中未找到字段: xxx` |
| cert_info 不是合法 JSON | 返回错误：`cert_info 不是合法 JSON` |
| storageRules 为空 | 跳过存储写入，仅执行 Cookie 逻辑 |
| storageRules 配置无效 | 返回错误：`存储规则配置无效` |
| 浏览器扩展注入失败 | 返回错误：`写入浏览器存储失败` |

### 六、兼容性

- **现有 Cookie 逻辑不变** — `cookies`/`cookie` 字段的处理完全保持原样
- **storageRules 可选** — 没有配置则不执行存储写入
- **配置格式向后兼容** — 旧配置文件没有 `storageRules` 字段也能正常解析
- **移除双击配置** — 不再支持双击"默认浏览器打开"配置 URL，改为在配置面板中统一管理

### 七、实现范围

本次改动涉及：

1. **前端** (`frontend/src/views/CertQueryPage.vue`)
 - 移除双击配置 URL 交互
 - 新增网站地址配置区域
 - 新增存储规则配置 UI
 - 规则的增删改查和保存

2. **前端 API** (`frontend/src/api/certQuery.js`)
 - 新增保存存储规则的 API 调用

3. **Rust 后端** (`src-tauri/src/browser_bridge.rs`)
 - 扩展 WebsiteUrlMapping 结构体
 - 新增 StorageRule、StorageSource、StorageItem 结构体
 - 新增值提取逻辑
 - 修改 open_default_browser_with_cookies 处理逻辑

4. **浏览器扩展** (`browser-extension/`)
 - manifest.json 添加权限
 - background.js 改造写入流程
