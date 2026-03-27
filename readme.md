
# 加解密可视化工具

一个基于前后端分离架构的可视化加解密工具，支持多种加密算法，可通过拖拽组件创建工作流，配置参数后执行加密解密操作。

## 功能特性

### 支持的加密算法
- **Base64** - 编码/解码
- **MD5** - 哈希算法
- **SHA** - SHA系列哈希算法（SHA1/SHA256/SHA384/SHA512）
- **HmacMD5** - HMAC-MD5 消息认证码
- **HmacSHA** - HmacSHA系列消息认证码（HmacSHA1/HmacSHA256/HmacSHA384/HmacSHA512）
- **AES** - 加密/解密（ECB/CBC模式）
- **Hex** - 十六进制编码/解码
- **URL** - URL编码/解码（支持UTF-8/GBK/ISO-8859-1字符集）
- **Unicode** - Unicode编码/解码（支持标准格式/HTML实体/CSS格式）
- **RSA** - 非对称加密算法（PKCS1/OAEP填充）
- **SM2** - 国密椭圆曲线加密算法
- **SM3** - 国密哈希算法
- **SM4** - 国密分组密码算法

### 核心功能

1. **单页面工作流设计**
 - 三栏可调整布局：组件库 | 工作流配置 | 执行面板
 - 拖拽式组件添加和排序
 - 实时配置和执行，无需页面切换

2. **组件库**
 - 按类别分组：编码类、哈希类、对称加密、非对称加密
 - 可折叠分类面板
 - 拖拽添加组件到工作流

3. **组件配置**
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

4. **输入映射配置**
 - 在工作流配置区顶部定义全局输入参数
 - 为每个输入映射指定自定义名称
 - 所有组件都可以引用这些输入映射
 - 执行面板会根据输入映射自动生成输入表单

5. **输出配置**
 - 支持配置多个输出映射
 - 为每个输出指定自定义名称
 - 选择任意组件的输出作为最终结果

6. **项目管理**
 - 头部项目选择器快速切换项目
 - 保存时自动填充当前项目名
 - 侧边抽屉管理项目列表
 - 支持项目内联重命名
 - 重复名称自动提示覆盖确认

7. **执行面板**
 - 根据配置自动生成输入表单
 - 执行按钮带加载状态
 - 可折叠执行日志
 - 支持 Ctrl+Enter 快捷执行

8. **主题支持**
 - 支持亮色/暗色主题切换
 - 主题偏好自动保存

### 响应式布局
- **桌面端 (≥1200px)**：三栏布局
- **平板端 (768px-1199px)**：两栏布局
- **移动端 (<768px)**：标签页切换布局

## 技术栈

### 后端
- **语言**： 21
- **框架**：
- **存储**：基于文件的项目配置（无需数据库）
- **构建工具**：
- **目录**：``

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
- 前后端分离
- RESTful API 通信
- 单页面应用（SPA）

## 安装步骤

### 数据库配置

当前版本已改为文件存储，无需安装数据库与配置数据源。

### 后端安装

1. **安装 JDK 21**
 - 下载并安装 JDK 21

2. **安装 **
 - 下载并安装 3.9+
 - 配置 MAVEN_HOME 环境变量

3. **构建后端项目**
 ```bash
 cd 
 mvn clean install
 ```

4. **运行后端服务**
 ```bash
 mvn spring-boot:run
 ```

### 后端配置说明

- `src/main/resources/application.yml`
 - 通用默认配置，会随 jar 一起打包。
 - 项目配置目录默认指向 `${user.home}/Documents/encryt/projects`。

- `src/main/resources/application-dev.yml`
 - 本地开发专用配置。
 - 当前仓库中已将你的本机项目目录放到 `dev` profile 下。
 - 本地开发如需启用，使用：
 ```bash
 mvn spring-boot:run -Dspring-boot.run.profiles=dev
 ```

- `src/packaging/config/application.yml`
 - 发布包外置配置模板。
 - 执行 `mvn package` 时会复制到 `target/config/application.yml`。
 - 运行 jar 时， 会优先读取 `config/application.yml`，可直接在这里改部署环境配置。

### 前端安装

1. **安装 Node.js**
 - 推荐版本：Node.js 18+

2. **安装前端依赖**
 ```bash
 cd frontend
 npm install
 ```

3. **启动前端开发服务器**
 ```bash
 npm run dev
 ```

## 使用说明

### 1. 创建工作流
1. 从左侧组件库拖拽所需组件到中间工作流配置区
2. 通过拖拽调整组件的执行顺序
3. 点击组件卡片展开配置面板，设置组件参数

### 2. 配置输入映射
在工作流配置区顶部的"输入映射"区域：
1. 点击"添加输入映射"
2. 设置输入映射名称（用于执行面板显示）
3. 可添加多个输入映射供组件引用

### 3. 配置组件参数

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
- **SM4**：选择 encrypt/decrypt，设置 mode、key、iv
- **Hex**：选择 encode 或 decode 操作
- **URL**：选择 encode/decode，设置字符集（UTF-8/GBK/ISO-8859-1）
- **Unicode**：选择 encode/decode，设置格式（标准格式/HTML实体/CSS格式）
- **RSA**：选择 encrypt/decrypt，设置公钥/私钥，填充模式（PKCS1/OAEP）
- **SM2**：选择 encrypt/decrypt，设置公钥/私钥
- **SM3**：选择是否输出小写结果

### 4. 配置输出映射
在工作流配置区底部的"输出配置"区域：
1. 点击"添加输出映射"
2. 设置输出名称和选择组件输出
3. 可添加多个输出映射

### 5. 执行工作流
1. 在右侧执行面板填写输入参数（根据输入映射自动生成）
2. 点击"执行"按钮或按 Ctrl+Enter
3. 查看执行结果和日志

### 6. 项目管理
- **保存**：点击头部"保存"按钮，已加载项目自动使用原名称
- **另存为**：点击"另存为"创建新项目
- **切换项目**：使用头部项目选择器
- **管理项目**：点击文件夹图标打开项目抽屉

### 7. 主题切换
点击头部右侧的太阳/月亮图标切换亮色/暗色主题

## 项目结构

```
├──  # 后端项目
│ ├── src/
│ │ └── main/
│ │ ├── / # 源代码
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
│ │ │ └── MainPage.vue # 主页面
│ │ ├── App.vue # 根组件
│ │ └── main.js # 入口文件
│ ├── package.json
│ └── vite.config.js
└── README.md
```

## 开发说明

### 后端开发

1. **添加新算法**
 - 在 `service` 目录下实现新算法
 - 创建对应的 REST API 接口

2. **项目导入/导出**
 - 导出全部项目（HTTP）：`GET /api/projects/transfer/export` 返回 zip
 - 导入压缩包（HTTP）：`POST /api/projects/transfer/import`，表单字段 `file`
 - 命令行脚本：
 ```bash
 # 导出到当前目录
 bash tools/export-projects.sh
 # 从 zip 导入到 projects/
 bash tools/import-projects.sh projects-20260101-120000.zip
 ```

### 前端开发

1. **添加新组件类型**
 - 在 `store/index.js` 的 `componentCategories` 中注册
 - 在 `ComponentConfigPanel.vue` 中添加配置表单

2. **构建生产版本**
 ```bash
 cd frontend
 npm run build
 ```

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request！
