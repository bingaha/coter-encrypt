# 在线凭证查询与默认浏览器打开计划

## 背景

项目当前新增的是一个“工具”类查库入口，不作为项目业务配置存储能力：

1. 用户配置 MySQL 数据源。
2. 输入主体名，选择办理类型。
3. 先查询 `robot_website_account` / `robot_account` / `robot_website`，拿到 `login_key`、`area_id` 等信息。
4. 再用 `login_key` 查询 `assistant_cert.valid = 1` 的最新在线凭证。
5. 页面展示并使用 `cert_info`。

默认浏览器打开能力在此基础上继续工作：

1. 根据 `area_id + 办理类型` 从本地 JSON 映射中找到目标网站地址。
2. 从 `cert_info.cookies` 或 `cert_info.cookie` 解析 Cookie。
3. 桌面程序启动一次性的 `127.0.0.1` 本地桥接服务。
4. 默认浏览器打开本地桥接页。
5. 浏览器插件在本地桥接页注入 content script，连接 WebSocket。
6. 插件 background 调用 `chrome.cookies.set()` 写 Cookie。
7. 插件跳转到目标网站。

## 当前约束

- 新功能只走 Rust/Tauri 本地命令。
- 不新增 后端能力。
- 不新增业务 HTTP API。
- 不把旧 或 HTTP 服务作为运行时 fallback。
- 浏览器 Cookie 写入必须由本地安装的浏览器插件完成。
- 插件不走插件商店，通过本地解压目录或 zip 解压后安装。
- 插件目录固定放在仓库根目录 `browser-extension/`。
- 桌面应用只保存一个插件 ID。默认浏览器如果不是安装该插件的浏览器，由用户自行调整默认浏览器或重新配置插件 ID。
- 第一版只支持 Chromium 系浏览器，按 Chrome / Edge 使用。
- URL 映射第一版只用本地 JSON 文件手工维护，不做编辑 UI。

## 关键配置文件

应用配置目录由 `directories::ProjectDirs` 生成，Windows 下通常位于：

```text
%APPDATA%\coter\CoterEncrypt\config\
```

当前配置文件：

```text
mysql-datasource.json # MySQL 数据源
browser-bridge.json # 浏览器插件 ID
website-url-mappings.json # area_id + 办理类型 -> 网站地址
```

网站地址映射示例：

```json
[
 {
 "areaId": "289",
 "businessType": "社保",
 "url": "https://sbwx.rst.shanxi.gov.cn:8007/ylwxsb/index.shtml"
 }
]
```

## 已确认决策

1. 浏览器范围：Chrome + Edge。
2. 插件 ID：用户本地安装插件后，手动复制插件 ID 到桌面应用配置。
3. 插件 ID 数量：桌面应用只保存一个插件 ID。
4. URL 映射：本地 JSON 文件，第一版手工维护。
5. 插件 host permissions：第一版使用 `<all_urls>`。
6. WebSocket：`127.0.0.1` + 随机端口 + 30 秒超时。
7. Cookie domain：第一版不做桌面端校验，交给浏览器。
8. `cert_info` 字段：先找 `cookies`，再找 `cookie`。
9. 多条查询结果：每条结果各自显示“默认浏览器打开”按钮。
10. 插件打包：维护根目录插件文件夹，并提供脚本生成 zip。

## 当前问题

旧实现由桌面程序调用系统默认打开器打开：

```text
chrome-extension://<插件ID>/bridge.html?port=...&token=...
```

在 Windows 上这会失败，提示没有可打开 `chrome-extension` 链接的应用。原因是 `chrome-extension://` 是浏览器内部协议，操作系统默认打开器不能可靠地把它路由到 Chrome / Edge。

## 修复方案

保留“默认浏览器 + 一个插件 ID”的使用方式，但不再让桌面程序直接打开 `chrome-extension://`。

新的打开流程：

```text
用户点击“默认浏览器打开”
 -> 桌面程序生成 token，启动 127.0.0.1:<随机端口>
 -> 桌面程序打开 http://127.0.0.1:<端口>/bridge?token=...&extensionId=...
 -> 默认浏览器正常打开本地 HTTP 页面
 -> 插件 content script 匹配 http://127.0.0.1/* 并注入本地页面
 -> content script 校验插件 ID，连接 ws://127.0.0.1:<端口>/ws?token=...
 -> 桌面程序发送 targetUrl + cookies
 -> content script 转发给 background
 -> background 写入 Cookie 并跳转当前标签页到 targetUrl
 -> 插件通过 WebSocket 回传执行结果
 -> 桌面程序关闭本地桥接服务
```

这个方案不再依赖操作系统识别 `chrome-extension://` 协议，默认浏览器只需要能打开普通的 `http://127.0.0.1` 页面。

## 开发任务

1. [已完成] 新增在线凭证查询工具入口。
2. [已完成] MySQL 数据源配置保存到本地配置文件。
3. [已完成] 办理类型改为下拉选择，并将“公积金 / 社保 / 医保 / 养老 / 工伤 / 失业”放在前面。
4. [已完成] 查询 `login_key` 后继续查询 `assistant_cert.cert_info`。
5. [已完成] 新增浏览器插件 ID 配置。
6. [已完成] 新增 URL 映射本地配置。
7. [已完成] 新增浏览器插件目录与打包脚本。
8. [已完成] 修复默认浏览器打开方式：改为 localhost bridge + content script + background。
9. [已完成] 更新 README 插件安装和 bridge 说明。
10. [已完成] 重新打包插件 zip。
11. [已完成] 运行前端构建、Rust 测试/检查验证。

## 验证记录

旧实现验证记录：

- `powershell -ExecutionPolicy Bypass -File tools/package-browser-extension.ps1`：通过，生成 `dist/coter-cookie-bridge.zip`。
- `npm --prefix frontend run build`：通过。
- `cargo test -p coter-encrypt-desktop browser_bridge -- --nocapture`：通过。
- `cargo check -p coter-encrypt-desktop`：通过。
- `npm run tauri:build`：通过。

本次修复验证记录：

- `powershell -ExecutionPolicy Bypass -File tools\package-browser-extension.ps1`：通过，重新生成 `dist/coter-cookie-bridge.zip`。
- `npm --prefix frontend run build`：通过。
- `cargo test -p coter-encrypt-desktop browser_bridge -- --nocapture`：通过，3 个相关测试通过。
- `cargo check -p coter-encrypt-desktop`：通过。
- `npm run tauri:build`：通过，生成 `target/release/CoterEncrypt.exe` 和 `target/release/bundle/nsis/CoterEncrypt_0.1.0_x64-setup.exe`。

## 小窗口滚动修复

### 开发任务

1. [已完成] 修复 Tauri 主窗口较小时二级页面内容无法滚动导致按钮/输入框不可达的问题。
2. [已完成] 返工在线凭证查询页滚动结构，改为页面内容区独立滚动。

## 在线凭证查询表单调整

### 开发任务

1. [已完成] 去掉查询条件中主体名输入框的示例占位文本，并将办理类型默认值改为“公积金”。

## 项目配置目录调整

### 开发任务

1. [已完成] 将项目 JSON 默认读写目录从用户文档目录 `encryt/projects` 改为应用配置目录下的 `projects/`。
2. [已完成] 按要求不迁移、不复制、不自动读取旧目录中的现有文件。
3. [已完成] 更新 README 中的默认项目目录说明。

## 在线凭证查询布局与版本调整

### 开发任务

1. [已完成] 将“查询条件”面板移动到在线凭证查询左侧配置区第一个位置。
2. [已完成] 将“数据源”和“浏览器插件”配置面板后移，保留为低频配置。
3. [已完成] 将项目版本号统一调整为 `0.1.1`。

## 浏览器插件 Cookie 写入调整

### 开发任务

1. [已完成] 浏览器插件写入新 Cookie 前，先清理目标网站当前域名及父域相关 Cookie，避免旧登录态冲突。
2. [已完成] 浏览器插件版本同步调整为 `0.1.1`。
3. [已完成] 重新生成 `dist/coter-cookie-bridge.zip`。

## 网站地址映射配置入口

### 开发任务

1. [已完成] 在线凭证查询“默认浏览器打开”遇到缺少网站地址映射时，首次点击提示无配置并提示可再次点击配置首页地址。
2. [已完成] 2 秒内再次点击同一条结果时打开网站首页地址配置弹窗，保存 `areaId + businessType -> url` 映射。
3. [已完成] 工具台首页新增“打开本地配置目录”按钮。

### 验证记录

- `npm --prefix frontend run build`：通过。
- 返工后再次运行 `npm --prefix frontend run build`：通过。
- 表单调整后再次运行 `npm --prefix frontend run build`：通过。
- `cargo test -p coter-core project_store -- --nocapture`：通过，4 个项目存储相关测试通过。
- `cargo check -p coter-encrypt-desktop`：通过。
- 布局与版本调整后再次运行 `npm --prefix frontend run build`：通过。
- 布局与版本调整后再次运行 `cargo check -p coter-encrypt-desktop`：通过。
- `powershell -ExecutionPolicy Bypass -File tools/package-browser-extension.ps1`：通过，重新生成插件 zip。
- Cookie 写入调整后再次运行 `cargo check -p coter-encrypt-desktop`：通过。
- Cookie 写入调整后再次运行 `npm --prefix frontend run build`：通过。
- 网站地址映射配置入口调整后运行 `npm --prefix frontend run build`：通过。
- 网站地址映射配置入口调整后运行 `cargo check -p coter-encrypt-desktop`：通过。

## 加解密工具进制转换组件

### 开发任务

1. [已完成] 新增加解密工具“进制转换”组件，走 Rust/Tauri 本地执行链路，不新增 或 HTTP 兜底。

### 验证记录

- `cargo test -p coter-core radix -- --nocapture`：通过，2 个进制转换核心测试通过。
- `cargo test -p coter-encrypt-desktop execute_batch_runs_radix_algorithm -- --nocapture`：通过，桌面执行链路 RADIX 测试通过。
- `npm --prefix frontend run build`：通过。
- `cargo check -p coter-encrypt-desktop`：通过。
