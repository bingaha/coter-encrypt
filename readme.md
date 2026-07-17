# 加解密可视化工具

基于 Rust/Tauri + Vue 的桌面可视化加解密工具。支持多种加密算法，可通过拖拽组件创建工作流，配置参数后执行加解密操作。

生产构建后生成可直接运行的桌面程序，无需打开浏览器访问 `localhost`，也无需手动启动 HTTP 服务。

应用启动后进入工具台首页（`/`）。「加解密工具」从首页进入 `/encrypt` 二级页面，并提供返回首页入口。后续 OCR、浏览器跳转、固定表查询等功能可按相同入口布局扩展。路由切换在同一个 Tauri 窗口内完成。

## 功能特性

### 支持的加密算法

- **Base64** - 编码/解码（解码支持输出格式：HEX / UTF-8 / GBK / BASE64）
- **MD5** - 哈希算法
- **SHA** - SHA1 / SHA256 / SHA384 / SHA512
- **HmacMD5** - HMAC-MD5 消息认证码
- **HmacSHA** - HmacSHA1 / HmacSHA256 / HmacSHA384 / HmacSHA512
- **AES** - 加密/解密（ECB/CBC）
- **Blowfish** - 加密/解密（ECB/CBC）
- **Hex** - 十六进制编码/解码
- **进制转换** - 2 到 36 进制数字转换
- **URL** - URL 编码/解码（UTF-8 / GBK / ISO-8859-1）
- **Unicode** - Unicode 编码/解码（标准 / HTML 实体 / CSS）
- **RSA** - 非对称加密（PKCS1 / OAEP）
- **SM2** - 国密椭圆曲线加密
- **SM3** - 国密哈希
- **SM4** - 国密分组密码

### 核心功能

1. **工具台首页与路由**
   - 默认路由 `/` 为工具台首页
   - 提供「加解密工具」「日志语句生成」「在线凭证查询」「任务反馈生成」「OSS Key 转换」「流水线监控」等入口
   - `/encrypt` 二级页面可返回首页
   - 页面切换在同一 Tauri 窗口内完成

2. **加解密工作流**
   - 三栏布局：组件库 | 工作流配置 | 执行面板
   - 拖拽添加、排序组件，实时配置并执行

3. **组件配置**
   - 卡片展开/折叠，配置不完整时显示警告
   - 支持 Delete 删除选中组件
   - 输入来源：输入映射 / 前置组件输出 / 表达式
   - 表达式语法：`${outputRef}` 引用输出；`/` 转义 `$`、`{`、`}`、`/`

4. **输入/输出映射**
   - 顶部定义全局输入参数，执行面板自动生成表单
   - 底部配置多个输出映射，选择任意组件输出作为最终结果

5. **项目管理**
   - 头部项目选择器、保存/另存为、侧边抽屉管理与内联重命名

6. **执行面板**
   - 自动输入表单、加载状态、可折叠日志、Ctrl+Enter 快捷执行

7. **主题**
   - 亮色/暗色切换，偏好自动保存

8. **流水线监控**
   - 监控云效流水线人工卡点与分支选择
   - 支持自动模式、配置热更新、后台轮询与待办通知

## 技术栈

- **桌面端**：Rust + Tauri 2，业务通过 `invoke` 本地命令调用
- **前端**：Vue 3、Naive UI、Pinia、Vue Router、Vite
- **存储**：基于文件的项目配置（无需数据库）；默认目录为应用配置目录下的 `projects/`（Windows 通常为 `%APPDATA%\coter\CoterEncrypt\config\projects\`）

## 环境要求

1. **Rust** 稳定版工具链（Windows 需可用的 MSVC 构建环境）
2. **Node.js** 18+
3. **WebView2 Runtime**（Windows 11 通常已内置）

核心加解密使用文件存储。在线凭证查询、任务反馈生成等查库工具需在首页「数据库配置」中配置 MySQL 数据源。

## 安装与运行

```bash
# 安装依赖
npm install
npm --prefix frontend install

# 开发启动（Vite 热更新 + Tauri 窗口）
npm run tauri:dev

# 生产构建
npm run tauri:build
```

主要输出：

```text
target/release/CoterEncrypt.exe
target/release/bundle/nsis/CoterEncrypt_0.1.10_x64-setup.exe
```

- `CoterEncrypt.exe`：便携/内测运行
- NSIS 安装包：适合分发

运行产物无需打开浏览器或访问 `localhost`。

## 使用说明

### 1. 进入加解密工具

1. 启动应用进入工具台首页
2. 点击「加解密工具」进入 `/encrypt`
3. 可通过头部入口返回首页

### 2. 创建与配置工作流

1. 从左侧组件库拖拽组件到中间工作流区，并调整顺序
2. 展开组件卡片配置参数
3. 在顶部配置输入映射，在底部配置输出映射

**输入来源（通用）**

- **使用输入映射**：选择预定义输入
- **使用组件输出**：选择前置组件输出
- **表达式输入**：模板组合固定文本与引用

**表达式**

- `${outputRef}` 引用组件输出；可多个占位符；纯文本可作为固定默认值
- 转义：`/$` → `$`，`/{` → `{`，`/}` → `}`，`//` → `/`

示例：

```text
固定前缀${base64-output-xxx}固定后缀
用户:${user-output}，密码:${pwd-output}
价格: /$100
```

**算法参数摘要**

- **Base64**：encode / decode；解码时可配置输出格式（HEX / UTF-8 / GBK / BASE64），HEX 时可配置大小写
- **MD5**：结果大小写、输出位数（16/32）
- **SHA**：算法类型与结果大小写
- **HmacMD5 / HmacSHA**：密钥、算法类型、结果格式等
- **AES / Blowfish / SM4**：encrypt/decrypt、mode、key、iv、输入/输出格式等
- **Hex**：encode / decode
- **进制转换**：源进制、目标进制、字母大小写（2–36，支持 `0b`/`0o`/`0x` 前缀）
- **URL**：encode/decode、字符集
- **Unicode**：encode/decode、格式
- **RSA / SM2**：encrypt/decrypt、密钥与填充等
- **SM3**：结果大小写

### 3. 执行工作流

1. 在右侧填写输入参数
2. 点击「执行」或 Ctrl+Enter
3. 查看结果与日志

### 4. 项目管理

- **保存**：已加载项目自动使用原名称
- **另存为**：创建新项目
- **切换 / 管理**：头部选择器与项目抽屉

### 5. 主题

点击头部太阳/月亮图标切换亮色/暗色主题。

### 6. 在线凭证查询与默认浏览器打开

1. 从首页进入「在线凭证查询」
2. 确保首页数据库状态为「连接成功」，否则先配置 MySQL
3. 安装本地插件 `browser-extension/`，将插件 ID 填入「浏览器插件」配置
4. 输入主体名并选择办理类型后查询
5. 结果含 `cert_info` 时可点「默认浏览器打开」

「默认浏览器打开」会按 `area_id + 办理类型` 查找本地网站地址映射，解析 Cookie 后经本地桥接页与浏览器插件写入 Cookie 并跳转目标站。插件仅支持 Chromium 系（Chrome / Edge）。

#### 本地插件安装

目录：`browser-extension/`

1. 打开扩展管理页（Chrome: `chrome://extensions/`，Edge: `edge://extensions/`）
2. 开启「开发者模式」→「加载已解压的扩展程序」→ 选择 `browser-extension/`
3. 复制扩展 ID，保存到桌面应用「浏览器插件」配置
4. 更新插件文件后需在扩展管理页点「重新加载」

打包 zip：

```powershell
powershell -ExecutionPolicy Bypass -File tools/package-browser-extension.ps1
```

输出：`dist/coter-cookie-bridge.zip`（需解压后再加载）。

#### 网站地址映射

默认文件：

```text
%APPDATA%\coter\CoterEncrypt\config\website-url-mappings.json
```

格式示例：

```json
[
  {
    "areaId": "289",
    "businessType": "社保",
    "url": "https://example.com/index.shtml"
  }
]
```

缺少映射时页面会提示；2 秒内再次点击同一条结果可打开配置弹窗。也可在首页「打开本地配置目录」后手工维护。

### 7. 任务反馈生成

1. 从首页进入「任务反馈生成」
2. 确认 MySQL 已连接
3. 输入 `task_id` 查询 `robot_task_user` / `robot_task_user_ins`
4. 选择反馈结果、消息与是否排除已反馈订单
5. 复制查询 SQL、更新 SQL 或 `handFeedBack` curl

`handFeedBack` 的 `userIdSet` 使用 `robot_task_user.id`。

## 许可证

MIT License
