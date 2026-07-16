
# 加解密可视化工具

一个基于 Rust/Tauri + Vue 的桌面可视化加解密工具，支持多种加密算法，可通过拖拽组件创建工作流，配置参数后执行加密解密操作。

当前主线版本不要求用户打开浏览器访问 `localhost`。开发期通过 Tauri 加载 Vite 前端页面，生产构建后生成可直接运行的桌面程序。

应用启动后首先进入工具台首页（`/`）。当前加解密工具作为首页入口进入 `/encrypt` 二级页面，二级页面提供返回首页功能；后续 OCR、浏览器跳转、固定表查询等独立 Rust/Tauri 功能可按相同入口布局继续扩展。路由切换仍在同一个 Tauri 桌面窗口内完成，不创建多窗口。

## 功能特性

### 支持的加密算法
- **Base64** - 编码/解码
- **MD5** - 哈希算法
- **SHA** - SHA系列哈希算法（SHA1/SHA256/SHA384/SHA512）
- **HmacMD5** - HMAC-MD5 消息认证码
- **HmacSHA** - HmacSHA系列消息认证码（HmacSHA1/HmacSHA256/HmacSHA384/HmacSHA512）
- **AES** - 加密/解密（ECB/CBC模式）
- **Blowfish** - 加密/解密（ECB/CBC模式）
- **Hex** - 十六进制编码/解码
- **进制转换** - 2 到 36 进制数字转换
- **URL** - URL编码/解码（支持UTF-8/GBK/ISO-8859-1字符集）
- **Unicode** - Unicode编码/解码（支持标准格式/HTML实体/CSS格式）
- **RSA** - 非对称加密算法（PKCS1/OAEP填充）
- **SM2** - 国密椭圆曲线加密算法
- **SM3** - 国密哈希算法
- **SM4** - 国密分组密码算法

### 核心功能

1. **工具台首页与路由**
 - 默认路由 `/` 为工具台首页
 - 当前提供“加解密工具”“日志语句生成”“在线凭证查询”“任务反馈生成”等工具入口
 - `/encrypt` 二级页面提供返回首页入口
 - 首页使用可扩展工具入口网格，后续可继续加入 OCR、浏览器跳转、固定表查询等独立功能
 - 页面切换在同一个 Tauri 窗口内完成，不打开额外窗口

2. **加解密工具工作流设计**
 - `/encrypt` 页面保留三栏可调整布局：组件库 | 工作流配置 | 执行面板
 - 拖拽式组件添加和排序
 - 实时配置和执行，无需页面切换

3. **组件库**
 - 按类别分组：编码类、哈希类、对称加密、非对称加密
 - 可折叠分类面板
 - 拖拽添加组件到工作流

4. **组件配置**
 - 组件卡片支持展开/折叠
 - 配置不完整时显示警告徽章
 - 支持键盘快捷键（Delete 删除选中组件）
 - **输入参数配置**：
 - **使用输入映射**：选择预定义的输入映射作为输入来源
 - **使用组件输出**：选择前置组件作为输入来源
 - **表达式输入**：使用模板语法组合固定文本和组件输出
 - 语法：`${outputRef}` 引用组件输出
 - 示例：`固定前缀${base64-output-xxx}固定后缀`
 - 转义：使用 `/` 转义特殊字符 `$`、`{`、`}`、`/`
 - 示例：`价格: /$100` 输出为 `价格: $100`

5. **输入映射配置**
 - 在工作流配置区顶部定义全局输入参数
 - 为每个输入映射指定自定义名称
 - 所有组件都可以引用这些输入映射
 - 执行面板会根据输入映射自动生成输入表单

6. **输出配置**
 - 支持配置多个输出映射
 - 为每个输出指定自定义名称
 - 选择任意组件的输出作为最终结果

7. **项目管理**
 - 头部项目选择器快速切换项目
 - 保存时自动填充当前项目名
 - 侧边抽屉管理项目列表
 - 支持项目内联重命名
 - 重复名称自动提示覆盖确认

8. **执行面板**
 - 根据配置自动生成输入表单
 - 执行按钮带加载状态
 - 可折叠执行日志
 - 支持 Ctrl+Enter 快捷执行

9. **主题支持**
 - 支持亮色/暗色主题切换
 - 主题偏好自动保存

### 响应式布局
- **桌面端 (≥1200px)**：三栏布局
- **平板端 (768px-1199px)**：两栏布局
- **移动端 (<768px)**：标签页切换布局

## 技术栈

### 桌面端与本地业务
- **语言**：Rust
- **桌面框架**：Tauri 2
- **业务调用**：Tauri `invoke` 本地命令
- **核心逻辑目录**：`crates/coter-core/`
- **桌面壳目录**：`src-tauri/`
- **存储**：基于文件的项目配置（无需数据库）
- **默认项目目录**：应用配置目录下的 `projects/`（Windows 通常为 `%APPDATA%\coter\CoterEncrypt\config\projects\`）

### 前端
- **框架**：Vue 3
- **UI组件库**：Naive UI
- **状态管理**：Pinia
- **路由**：Vue Router
- **布局**：Splitpanes（可调整分栏）
- **拖拽**：vuedraggable
- **图标**：@vicons/ionicons5
- **构建工具**：Vite
- **目录**：`frontend/`

### 架构
- Tauri 桌面壳加载 Vue 单页面应用
- 前端通过 Tauri `invoke` 调用 Rust 本地命令
- 不暴露业务 HTTP API
- 不需要用户手动打开浏览器访问本地地址

## 安装步骤

### 环境要求

1. **安装 Rust**
 - 推荐使用稳定版 Rust 工具链。
 - Windows 需要可用的 MSVC 构建环境。

2. **安装 Node.js**
 - 推荐版本：Node.js 18+。

3. **WebView2 Runtime**
 - Windows 11 通常已内置。
 - 较旧的 Windows 10 环境如无法启动 Tauri 窗口，需要安装 Microsoft Edge WebView2 Runtime。

当前核心加解密功能使用文件存储，无需数据库。在线凭证查询、任务反馈生成等查库工具需要在首页“数据库配置”中配置 MySQL 数据源。

### 安装依赖

在仓库根目录执行：

```bash
npm install
npm --prefix frontend install
```

### 开发启动

在仓库根目录执行：

```bash
npm run tauri:dev
```

该命令会：

1. 启动 Vite 前端开发服务器，用于页面热更新。
2. 启动 Tauri 桌面窗口。
3. 在桌面窗口中加载 Vue 页面。
4. 业务能力通过 Rust/Tauri 本地命令执行。

### 生产构建

在仓库根目录执行：

```bash
npm run tauri:build
```

该命令会先构建 `frontend/dist`，再构建 Rust/Tauri release 程序，并生成 Windows 发布产物。

当前主要输出路径：

```text
target/release/CoterEncrypt.exe
target/release/bundle/nsis/CoterEncrypt_0.1.4_x64-setup.exe
```

其中：

- `target/release/CoterEncrypt.exe` 可用于开发内测或便携运行。
- `target/release/bundle/nsis/CoterEncrypt_0.1.4_x64-setup.exe` 是 NSIS 安装包，适合分发给普通用户。

运行这些产物不需要打开浏览器，也不需要访问 `localhost`。

## 使用说明

### 1. 从首页进入加解密工具
1. 启动应用后进入工具台首页
2. 点击“加解密工具”进入 `/encrypt` 二级页面
3. 在加解密工具页面可通过头部返回首页入口回到工具台

### 2. 创建工作流
1. 从左侧组件库拖拽所需组件到中间工作流配置区
2. 通过拖拽调整组件的执行顺序
3. 点击组件卡片展开配置面板，设置组件参数

### 3. 配置输入映射
在工作流配置区顶部的"输入映射"区域：
1. 点击"添加输入映射"
2. 设置输入映射名称（用于执行面板显示）
3. 可添加多个输入映射供组件引用

### 4. 配置组件参数

**输入来源配置**（所有组件通用）：
- **使用输入映射**：选择预定义的输入映射作为输入来源
- **使用组件输出**：选择前置组件的输出作为输入
- **表达式输入**：使用模板语法组合固定文本和组件输出

**表达式输入语法说明**：
- 使用 `${outputRef}` 引用组件输出，outputRef 为组件的输出标识符
- 可在一个表达式中使用多个占位符
- 纯文本表达式（不含占位符）可作为组件的固定默认值
- **转义字符**：使用 `/` 转义特殊字符

**转义规则**：
| 转义序列 | 输出字符 |
|---------|---------|
| `/$` | `$` |
| `/{` | `{` |
| `/}` | `}` |
| `//` | `/` |

**表达式示例**：
```
# 组合固定文本和组件输出
固定前缀${base64-output-xxx}固定后缀

# 多个占位符
用户:${user-output}，密码:${pwd-output}

# 包含特殊字符的文本（使用 / 转义）
价格: /$100 → 输出: 价格: $100
//${var} → 输出: /${var}（字面量，不会被解析为占位符）
路径: C://Users → 输出: 路径: C:/Users
```

**算法特定参数**：
- **Base64**：选择 encode 或 decode 操作
- **MD5**：选择是否输出小写结果，选择输出位数（16位/32位）
- **SHA**：选择算法类型（SHA1/SHA256/SHA384/SHA512），选择是否输出小写结果
- **HmacMD5**：设置密钥，选择是否输出小写结果，选择输出位数（16位/32位）
- **HmacSHA**：设置密钥，选择算法类型（HmacSHA1/HmacSHA256/HmacSHA384/HmacSHA512），选择是否输出小写结果
- **AES**：选择 encrypt/decrypt，设置 mode、key、iv
- **Blowfish**：选择 encrypt/decrypt，设置 mode、key、iv
- **SM4**：选择 encrypt/decrypt，设置 mode、key、iv
- **Hex**：选择 encode 或 decode 操作
- **进制转换**：设置源进制、目标进制和输出字母大小写（支持 2 到 36 进制，输入可使用 `0b`/`0o`/`0x` 前缀）
- **URL**：选择 encode/decode，设置字符集（UTF-8/GBK/ISO-8859-1）
- **Unicode**：选择 encode/decode，设置格式（标准格式/HTML实体/CSS格式）
- **RSA**：选择 encrypt/decrypt，设置公钥/私钥，填充模式（PKCS1/OAEP）
- **SM2**：选择 encrypt/decrypt，设置公钥/私钥
- **SM3**：选择是否输出小写结果

### 5. 配置输出映射
在工作流配置区底部的"输出配置"区域：
1. 点击"添加输出映射"
2. 设置输出名称和选择组件输出
3. 可添加多个输出映射

### 6. 执行工作流
1. 在右侧执行面板填写输入参数（根据输入映射自动生成）
2. 点击"执行"按钮或按 Ctrl+Enter
3. 查看执行结果和日志

### 7. 项目管理
- **保存**：点击头部"保存"按钮，已加载项目自动使用原名称
- **另存为**：点击"另存为"创建新项目
- **切换项目**：使用头部项目选择器
- **管理项目**：点击文件夹图标打开项目抽屉

### 8. 主题切换
点击头部右侧的太阳/月亮图标切换亮色/暗色主题

### 9. 在线凭证查询与默认浏览器打开
1. 从首页进入“在线凭证查询”
2. 如首页数据库状态不是“连接成功”，点击首页“数据库配置”保存并测试 MySQL 数据源；也可以在查询时根据弹窗提示配置
3. 在浏览器中安装本地插件 `browser-extension/`
4. 将浏览器扩展页面中显示的插件 ID 填入“浏览器插件”配置并保存
5. 输入主体名并选择办理类型后执行查询
6. 查询结果存在 `cert_info` 时，可点击“默认浏览器打开”

“默认浏览器打开”会根据 `area_id + 办理类型` 从本地网站地址映射文件中查找目标地址，再从 `cert_info.cookies` 或 `cert_info.cookie` 解析 Cookie。桌面应用会打开 `http://127.0.0.1:<随机端口>/bridge?...` 本地桥接页，浏览器插件在该页面注入脚本并通过本地 WebSocket 接收 Cookie，随后调用浏览器扩展 API 先清理目标网站当前域名及父域相关 Cookie，再写入桌面应用传递的 Cookie 并跳转到目标网站。

插件只支持 Chromium 系浏览器，当前按 Chrome / Edge 使用。桌面应用只保存一个插件 ID；如果默认浏览器不是安装该插件的浏览器，需要调整系统默认浏览器或重新配置插件 ID。

### 10. 任务反馈生成
1. 从首页进入“任务反馈生成”
2. 确认首页“数据库配置”中的 MySQL 数据源已连接成功；如果未配置或连接失败，点击查询时也会弹出配置窗口
3. 输入 `task_id`，按需要确认库名，点击“按 task_id 查询数据库”
4. 页面会查询 `robot_task_user` 和 `robot_task_user_ins`，自动填入两张表对应记录的主键
5. 选择反馈结果、反馈消息和是否排除已反馈订单
6. 复制生成的查询 SQL、更新 SQL 或 `handFeedBack` curl

其中 `handFeedBack` curl 的 `userIdSet` 使用 `robot_task_user.id`，`robot_task_user_ins.id` 只用于生成直接更新险种订单表的 SQL。

#### 本地插件安装

插件目录：

```text
browser-extension/
```

Chrome / Edge 安装步骤：

1. 打开扩展管理页
 - Chrome: `chrome://extensions/`
 - Edge: `edge://extensions/`
2. 打开“开发者模式”
3. 点击“加载已解压的扩展程序”
4. 选择仓库根目录下的 `browser-extension/`
5. 安装后复制扩展详情中显示的插件 ID
6. 回到桌面应用“在线凭证查询”页面，将插件 ID 保存到“浏览器插件”配置

如果已经安装过旧版本插件，更新 `browser-extension/` 文件后需要在扩展管理页点击该插件的“重新加载”。否则浏览器不会加载新的 content script / background 逻辑。

如需生成 zip 包，可在仓库根目录执行：

```powershell
powershell -ExecutionPolicy Bypass -File tools/package-browser-extension.ps1
```

输出文件：

```text
dist/coter-cookie-bridge.zip
```

Chrome / Edge 不能直接加载 zip 文件作为已解压扩展，分发给用户后仍需要先解压，再选择解压后的目录加载。

#### 网站地址映射

第一次加载网站地址映射时，桌面应用会在用户配置目录创建默认 JSON 文件：

```text
%APPDATA%\coter\CoterEncrypt\config\website-url-mappings.json
```

文件格式：

```json
[
 {
 "areaId": "289",
 "businessType": "社保",
 "url": "https://sbwx.rst.shanxi.gov.cn:8007/ylwxsb/index.shtml"
 }
]
```

如果点击“默认浏览器打开”时缺少 `area_id + 办理类型` 对应的网站地址映射，页面会提示无配置；2 秒内再次点击同一条结果可打开配置弹窗，输入首页地址并保存后即可立即使用。也可以在工具台首页点击“打开本地配置目录”，手工维护该 JSON 文件。

## 项目结构

```
├── browser-extension/ # 本地浏览器插件，用于写入 Cookie 并跳转默认浏览器
│ ├── manifest.json
│ ├── background.js # 写入 Cookie 并跳转当前标签页
│ └── content-script.js # 注入本地 bridge 页面并连接桌面 WebSocket
├── crates/
│ └── coter-core/ # Rust 核心业务逻辑
│ └── src/
│ ├── crypto.rs # 加密算法实现
│ ├── executor.rs # 工作流执行引擎
│ ├── expression.rs # 表达式解析
│ ├── har.rs # HAR 处理
│ ├── project_store.rs # 项目文件管理
│ ├── schema.rs # 项目配置结构
│ └── validation.rs # 配置校验
├── src-tauri/ # Tauri 桌面壳与命令绑定
│ ├── src/
│ │ ├── main.rs # Tauri 入口与命令注册
│ │ ├── browser_bridge.rs # 浏览器插件联动与本地 WebSocket
│ │ ├── executor.rs # 桌面侧执行命令适配
│ │ ├── har.rs # 桌面侧 HAR 命令适配
│ │ ├── crypto.rs # 桌面侧算法导出
│ │ └── project_store.rs # 桌面侧项目存储适配
│ ├── icons/ # 应用图标
│ ├── Cargo.toml
│ └── tauri.conf.json
│ ├── src/
│ │ └── main/
│ │ │ └── com/coter/encrypt/
│ │ │ ├── config/ # 配置类
│ │ │ ├── controller/ # 控制器层
│ │ │ ├── dto/ # 数据传输对象
│ │ │ ├── entity/ # 实体类
│ │ │ ├── controller/ # 控制器层
│ │ │ └── service/ # 服务层
│ │ └── resources/
│ │ └── application.yml # 应用配置
│ └── pom.xml
├── frontend/ # 前端项目
│ ├── src/
│ │ ├── api/ # API 接口
│ │ ├── assets/ # 静态资源
│ │ ├── components/ # Vue 组件
│ │ │ ├── config/ # 配置相关组件
│ │ │ │ ├── ComponentLibrary.vue # 组件库面板
│ │ │ │ ├── WorkflowConfig.vue # 工作流配置
│ │ │ │ └── ComponentCard.vue # 组件配置卡片
│ │ │ ├── execute/ # 执行相关组件
│ │ │ │ └── ExecutePanel.vue # 执行面板
│ │ │ ├── layout/ # 布局组件
│ │ │ │ ├── MainLayout.vue # 主布局
│ │ │ │ └── AppHeader.vue # 头部组件
│ │ │ └── project/ # 项目管理组件
│ │ │ └── ProjectDrawer.vue # 项目抽屉
│ │ ├── composables/ # 组合式函数
│ │ │ ├── useKeyboardShortcuts.js # 键盘快捷键
│ │ │ └── useProjectManager.js # 项目管理
│ │ ├── router/ # 路由配置
│ │ ├── store/ # 状态管理（Pinia）
│ │ ├── views/ # 页面视图
│ │ │ ├── HomePage.vue # 工具台首页
│ │ │ └── MainPage.vue # 加解密工具页面
│ │ ├── App.vue # 根组件
│ │ └── main.js # 入口文件
│ ├── package.json
│ └── vite.config.js
├── Cargo.toml # Rust workspace
├── package.json # 根级 Tauri/NPM 脚本
├── tools/
│ └── package-browser-extension.ps1 # 浏览器插件 zip 打包脚本
└── readme.md
```

## 开发说明

### Rust/Tauri 开发

1. **添加新算法**
 - 优先在 `crates/coter-core/src/crypto.rs` 中实现核心逻辑
 - 在 `crates/coter-core` 中补充单元测试
 - 如需暴露给前端执行，在 `src-tauri/src/executor.rs` 中接入算法分发
 - 前端通过 `frontend/src/api/*.js` 调用 Tauri 命令，不新增 `/api` 或 `localhost` 业务调用

2. **添加新的桌面命令**
 - 在 `src-tauri/src/main.rs` 中定义并注册 Tauri command
 - 复杂业务逻辑优先下沉到 `crates/coter-core`
 - Tauri 层只负责窗口、文件路径、文件对话框和命令参数适配

3. **项目数据**
 - 默认保存目录：应用配置目录下的 `projects/`（Windows 通常为 `%APPDATA%\coter\CoterEncrypt\config\projects\`）
 - 项目文件为 JSON，文件名为 `项目名.json`

### 前端开发

1. **添加新工具入口或页面**
 - 在 `frontend/src/views/` 中新增页面视图
 - 在 `frontend/src/router/index.js` 中新增路由
 - 在 `frontend/src/views/HomePage.vue` 的工具入口列表中添加入口配置
 - 新功能的本地能力优先通过 `src-tauri/src/` 注册 Tauri command，并将复杂业务逻辑下沉到 `crates/coter-core`
 - 不为新功能新增业务 HTTP API、`localhost` 调用

2. **添加新加解密组件类型**
 - 在 `store/index.js` 的 `componentCategories` 中注册
 - 在 `ComponentConfigPanel.vue` 中添加配置表单
 - 在 `frontend/src/api/*.js` 中通过 Tauri `invoke` 调用桌面命令

3. **开发期只构建前端**
 ```bash
 npm --prefix frontend run build
 ```

4. **完整桌面生产构建**
 ```bash
 npm run tauri:build
 ```

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request！
