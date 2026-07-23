# AGENTS.md

## 当前主线目标

本仓库当前主线目标是将项目实现为 Rust/Tauri 桌面应用，并最终产出一个用户可直接运行的可执行程序。

最终形态：

```text
CoterEncrypt.exe
 - 直接双击运行
 - 内置或加载已打包的 Vue 前端资源
 - 业务能力由 Rust/Tauri 本地命令提供
 - 不要求用户打开浏览器访问 localhost
 - 不要求用户手动启动 HTTP 服务
```

后续开发必须围绕这个最终形态推进。

## Rust/Tauri 约束

1. 前端业务调用应使用 Tauri `invoke`，不再新增 `/api`、`localhost:8080` 或其他业务 HTTP 调用。
2. 不要设计 HTTP/Tauri 双运行模式、HTTP fallback、自动回退旧服务等兼容分支。
3. 不要为了“可回滚到旧架构”增加额外抽象、条件判断、环境开关或双份实现。
4. 应直接面向最终可执行程序实现缺失能力；如果命令尚未完成，应按计划实现该命令或返回明确错误。
5. 如确实需要临时脚本或验证命令，必须保持在开发/测试范围内，不进入最终运行链路。
6. 代码结构应优先简洁，避免为过渡期兜底策略引入长期维护成本。

## 当前推荐开发流程

如仓库存在当前计划文档，主线开发优先参考该计划，按其中拆分任务逐步推进。

每次执行迁移任务时：

```text
1. 如存在当前计划文档，先把当前小任务标记为“执行中”。
2. 只完成当前小任务，不顺手扩大范围。
3. 完成后运行必要验证。
4. 如存在当前计划文档，将任务标记为“已完成”，并记录验证结果。
```

开发期可以使用：

```bash
npm run tauri:dev
```

这只表示 Tauri 在开发模式下通过 Vite dev server 加载前端资源，属于前端热更新机制，不代表保留业务 HTTP API。

生产验证使用：

```bash
npm run tauri:build
```

当前 Rust/Tauri release 输出位置：

```text
target/release/CoterEncrypt.exe
```

Debian/Linux 桌面图标通常对应：

```text
target/release/CoterEncrypt
```

---

## 生产包常见问题（必读）

### 1. 打开 deb / 安装后应用显示 `Could not connect to 127.0.0.1: Connection refused`

#### 这是什么

Tauri 有两种加载前端的方式：

| 模式 | 命令 / 产物 | 前端从哪来 |
|------|-------------|------------|
| 开发 | `npm run tauri:dev` → 通常是 `target/debug/CoterEncrypt` | 连接 `tauri.conf.json` 里的 `devUrl`（当前为 `http://127.0.0.1:5173` Vite） |
| 生产 | `npm run tauri:build` → deb / nsis / `target/release/CoterEncrypt` | 加载打包进应用的 `frontendDist`（`frontend/dist`）静态资源 |

**Connection refused = 实际跑起来的是开发态二进制（或等价配置），去连本机 5173，但 Vite 没在跑。**  
这**不是**业务 API 又改回 HTTP，而是 WebView 连错了前端资源入口。

#### 为什么“改代码后有时又会遇到”

常见触发方式（改代码本身不会随机坏掉，而是启动路径被悄悄换掉了）：

1. **用户级 `.desktop` 覆盖了 deb 安装的入口（本仓库曾踩过）**  
   Linux/GNOME 优先使用：

   ```text
   ~/.local/share/applications/CoterEncrypt.desktop
   ```

   若其中 `Exec=` 指向 `target/debug/CoterEncrypt`，从应用菜单点开就会连 `127.0.0.1:5173`。  
   deb 装好了也没用——菜单根本没跑 `/usr/bin/CoterEncrypt`。

2. **直接启动了 debug 二进制**  
   例如 `target/debug/CoterEncrypt`、`cargo run`（未走 release bundle），且没有同时开着 `tauri:dev` / Vite。

3. **误把开发产物当安装包验证**  
   应用菜单、命令行别名、旧快捷方式仍指向 workspace 里的 debug 路径。

#### 如何避免 / 排查

**禁止：**

- 在代码或脚本里向 `~/.local/share/applications/` 写入会覆盖正式入口的 `CoterEncrypt.desktop`（尤其 `Exec=.../target/debug/...`、`Icon=utilities-terminal` 这类临时文件）。
- 用 debug 二进制验证“用户下载 deb 后能否用”。

**生产验证只认：**

```bash
npm run tauri:build
# 安装生成的 .deb，或直接运行：
./target/release/CoterEncrypt
```

**若再次出现 Connection refused，按顺序查：**

```bash
# 1) 是否存在用户级覆盖（有则删掉或改掉）
ls -l ~/.local/share/applications/CoterEncrypt.desktop
cat ~/.local/share/applications/CoterEncrypt.desktop

# 2) 当前启动的到底是哪个二进制
which CoterEncrypt
readlink -f "$(which CoterEncrypt)"
# 菜单启动可用：ps -ef | rg CoterEncrypt

# 3) 正式安装入口应类似
# Exec=CoterEncrypt
# Icon=CoterEncrypt
# 文件在：/usr/share/applications/CoterEncrypt.desktop
```

删除错误覆盖后，必要时注销/重登或执行 `update-desktop-database ~/.local/share/applications`，再从应用菜单打开。

---

### 2. 应用图标变成“命令行 / 终端”样式，而不是绿色锁

#### 原因

正式图标来自 `src-tauri/icons/`（绿色锁），deb 安装后一般为：

```text
Icon=CoterEncrypt
→ /usr/share/icons/hicolor/*/apps/CoterEncrypt.png
```

若菜单显示终端图标，几乎总是用户级 desktop 覆盖了系统项，例如：

```ini
Icon=utilities-terminal
```

（本仓库在调试 Linux 通知时曾误写过该文件；与“改 UI 代码导致图标资源丢失”无关。）

#### 如何避免 / 修复

1. **禁止**写入错误的覆盖文件：`Exec=.../target/debug/...` 或 `Icon=utilities-terminal`。  
2. 若必须使用用户级 `~/.local/share/applications/CoterEncrypt.desktop`（见下一节），必须：  
   - `Exec=CoterEncrypt`（系统 PATH / deb 安装的正式二进制）  
   - `Icon=CoterEncrypt`（绿色锁）  
   - `Categories=Utility;`（非空，否则 GNOME 应用网格可能不显示）  
3. 不要删掉入口后又不补 Categories——deb 默认模板若 `Categories=` 为空，删掉用户覆盖后菜单里会“消失”。

### 2.1 应用菜单里完全没有 CoterEncrypt

#### 原因

1. 用户级错误 `.desktop` 被删掉后，系统项  
   `/usr/share/applications/CoterEncrypt.desktop` 可能是 `Categories=` **空字符串**。  
   GNOME 应用概览常因此不把应用放进网格（搜索有时也找不到）。  
2. 桌面数据库未刷新 / Shell 缓存未更新。

#### 修复（本机立即恢复）

写入正确的用户级入口（覆盖空 Categories 的系统项）：

```bash
mkdir -p ~/.local/share/applications
cat > ~/.local/share/applications/CoterEncrypt.desktop <<'EOF'
[Desktop Entry]
Type=Application
Name=CoterEncrypt
Comment=Coter Encrypt desktop application
Exec=CoterEncrypt
Icon=CoterEncrypt
Terminal=false
Categories=Utility;
StartupWMClass=CoterEncrypt
StartupNotify=false
EOF
update-desktop-database ~/.local/share/applications
```

然后打开应用菜单搜索 `CoterEncrypt`；若仍无，注销重登或 `Alt+F2` 输入 `r` 重启 GNOME Shell（X11）。

#### 打包侧避免（以后打 deb）

`tauri.conf.json` 的 `bundle.category` 应设为非空（如 `"Utility"`），保证生成的 `.desktop` 带 `Categories=`，不依赖用户手写覆盖。

---

### 3. 开发 vs 生产对照（防混用）

```text
开发：npm run tauri:dev
  → WebView → http://127.0.0.1:5173（需 Vite）
  → 二进制多在 target/debug/

生产：npm run tauri:build / 安装 deb
  → WebView → 内置 frontend/dist
  → 二进制在 target/release/ 或 /usr/bin/CoterEncrypt
  → 不依赖本机 5173
```

改功能后若要确认“用户双击能用”，必须以 **release / deb** 验证，不能只看 `tauri:dev` 页面是否正常。
