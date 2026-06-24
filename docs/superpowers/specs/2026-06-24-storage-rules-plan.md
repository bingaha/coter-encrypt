# 存储写入规则功能 - 实现计划

基于设计文档：`docs/superpowers/specs/2026-06-24-storage-rules-design.md`

## 任务列表

### 任务 1：Rust 后端 - 扩展数据结构

**文件：** `src-tauri/src/browser_bridge.rs`

**步骤：**
1. 新增 `StorageRule` 结构体
2. 新增 `StorageSource` 结构体
3. 新增 `StorageItem` 结构体
4. 扩展 `WebsiteUrlMapping` 添加 `storage_rules` 可选字段
5. 扩展 `BridgePayload` 添加 `storage_items` 可选字段

**验证：** `cargo check` 通过

---

### 任务 2：Rust 后端 - 实现值提取逻辑

**文件：** `src-tauri/src/browser_bridge.rs`

**步骤：**
1. 新增 `extract_storage_value` 函数
2. 新增 `resolve_storage_items` 函数（遍历 rules，调用 extract_storage_value）
3. 添加单元测试

**验证：** `cargo test` 通过

---

### 任务 3：Rust 后端 - 修改 open_default_browser_with_cookies

**文件：** `src-tauri/src/browser_bridge.rs`

**步骤：**
1. 修改 `open_default_browser_with_cookies` 函数
2. 在提取 Cookie 后，读取 storageRules 配置
3. 调用 `resolve_storage_items` 提取存储值
4. 将 storage_items 添加到 BridgePayload

**验证：** `cargo check` 通过

---

### 任务 4：前端 API - 确保 storageRules 随配置保存

**文件：** `frontend/src/api/certQuery.js`

**步骤：**
1. 检查现有 `saveWebsiteUrlMapping` API 是否需要调整
2. 确保 storageRules 能够通过现有 API 保存

**验证：** API 调用格式正确

---

### 任务 5：前端 UI - 移除双击配置 URL 交互

**文件：** `frontend/src/views/CertQueryPage.vue`

**步骤：**
1. 移除 `pendingMappingKey`、`pendingMappingTimer` 相关状态
2. 移除 `handleMissingMappingClick` 函数
3. 移除 `clearPendingMappingClick` 函数
4. 修改 `handleOpenDefaultBrowser` 移除双击检测逻辑
5. 移除 `onBeforeUnmount` 中的 clearPendingMappingClick 调用

**验证：** 页面正常渲染，点击"默认浏览器打开"不再有双击提示

---

### 任务 6：前端 UI - 在结果条目中添加网站地址配置

**文件：** `frontend/src/views/CertQueryPage.vue`

**步骤：**
1. 在每个结果条目中添加网站地址输入框和保存按钮
2. 实现加载已有配置的逻辑（从 websiteUrlMappings 中查找）
3. 实现保存网站地址的逻辑

**验证：** 可以在结果条目中配置并保存网站地址

---

### 任务 7：前端 UI - 添加存储规则配置 UI

**文件：** `frontend/src/views/CertQueryPage.vue`

**步骤：**
1. 为每个结果条目添加 storageRules 本地状态
2. 实现规则的增删改 UI
3. 每条规则包含：存储类型下拉、键名输入、来源类型下拉、路径/值输入
4. 实现保存规则的逻辑
5. 查询时自动加载已有规则配置

**验证：** 可以增删改规则并保存

---

### 任务 8：浏览器扩展 - 更新 manifest.json

**文件：** `browser-extension/manifest.json`

**步骤：**
1. 添加 `scripting` 权限
2. 添加 `host_permissions: ["<all_urls>"]`

**验证：** 扩展加载无报错

---

### 任务 9：浏览器扩展 - 改造 background.js

**文件：** `browser-extension/background.js`

**步骤：**
1. 添加 `waitForTabLoad` 辅助函数
2. 添加 `injectStorage` 函数（注入到页面执行）
3. 修改 `writeCookiesAndOpen` 函数：
 - 写入 Cookie 后，创建新标签页打开目标 URL
 - 等待页面加载完成
 - 如果有 storageItems，注入脚本写入存储
 - 如果注入了存储，刷新页面
4. 修改消息监听器，传递完整的 sender 信息

**验证：** 扩展加载无报错，手动测试写入功能

---

## 执行顺序

```
任务 1 → 任务 2 → 任务 3 → 任务 4 → 任务 5 → 任务 6 → 任务 7 → 任务 8 → 任务 9
```

## 完成标准

- [x] Rust 后端编译通过 (`cargo build`)
- [x] 前端构建通过 (`npm run build`)
- [x] 浏览器扩展加载无报错
- [x] 能够配置存储规则并保存
- [x] 点击“默认浏览器打开”能正确写入 Cookie 和存储值
