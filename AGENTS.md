# AGENTS.md

## 当前主线目标

本仓库当前主线目标是将项目重构为 Rust/Tauri 桌面应用，并最终产出一个用户可直接运行的 Rust 可执行程序。

最终形态：

```text
CoterEncrypt.exe
 - 直接双击运行
 - 内置或加载已打包的 Vue 前端资源
 - 业务能力由 Rust/Tauri 本地命令提供
 - 不依赖 后端
 - 不要求用户打开浏览器访问 localhost
 - 不要求用户手动启动 HTTP 服务
```

后续开发必须围绕这个最终形态推进。迁移过程中可以保留旧 后端作为行为参考、数据格式参考或 golden tests 对拍来源，但不能把旧 后端或 HTTP API 设计成新程序的运行时兜底路径。

## Rust/Tauri 重构约束

1. 前端业务调用应逐步迁移到 Tauri `invoke`，不再新增 `/api`、`localhost:8080` 或其他业务 HTTP 调用。
2. 不要设计 HTTP/Tauri 双运行模式、HTTP fallback、 fallback、自动回退旧服务等兼容分支。
3. 不要为了“可回滚到旧架构”增加额外抽象、条件判断、环境开关或双份实现。
4. 迁移时应直接面向最终 Rust 可执行程序实现缺失能力；如果 Rust 命令尚未完成，应按计划实现该命令或返回明确错误，而不是回退到旧 HTTP 后端。
5. 旧 代码只用于理解现有行为、迁移算法、验证兼容性和生成对拍用例；不作为新桌面程序的运行依赖。
6. 如确实需要临时脚本或验证命令，必须保持在开发/测试范围内，不进入最终运行链路。
7. 代码结构应优先简洁，避免为过渡期兜底策略引入长期维护成本。

## 旧 代码约束

1. `` 下的 / 代码已属于废弃业务，除非任务明确要求维护旧 版本、生成对拍数据或验证历史行为，否则不要修改。
2. 新增或迁移算法时，默认只实现 Rust/Tauri 本地执行链路，不同步改造 `CryptoUtil`、`RemovedCodeService`、 Controller、 DTO 或旧 HTTP API。
3. 不要为了新 Rust/Tauri 功能补 后端实现、 fallback、 兼容分支或 代码生成支持。
4. 如必须读取旧 代码，只能把它作为行为参考或 golden tests 来源；读取不代表允许修改。
5. 如某个旧 生成能力不支持新 Rust/Tauri 算法，应保持明确不支持或跳过，不要擅自扩展 生成链路。

## 当前推荐开发流程

如仓库存在当前计划文档，主线开发优先参考该计划，按其中拆分任务逐步推进。历史 Rust/Tauri 迁移计划已完成并移除，后续以新的计划文档为准。

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

当前 Rust/Tauri release exe 输出位置：

```text
target/release/CoterEncrypt.exe
```

## 打包部署流程

以下为旧 / 版本的历史打包流程。除非任务明确要求维护旧 版本、生成对拍数据或验证历史构建，否则不要把该流程作为当前主线交付目标。

### 前端打包

```bash
cd frontend
npm run build
```

### 复制前端文件到后端

```bash
cd 
rm -rf src/main/resources/static/*
cp -r ../frontend/dist/* src/main/resources/static/
```

### 后端打包

```bash
cd 
mvn clean package -DskipTests
```

### 打包产物

- 前端打包输出: `frontend/dist/`
- 后端打包输出: `target/-1.0-SNAPSHOT.jar`

### 打包说明

- 后端打包使用 `mvn clean package -DskipTests`，避免 `target/classes/static/` 残留上一次构建的旧前端资源。

### 完整打包命令

```bash
# 1. 打包前端
cd frontend && npm run build && cd ..

# 2. 复制前端文件到后端
cd 
del src/main/resources/static/*
cp -r ../frontend/dist/* src/main/resources/static/

# 3. 打包后端
mvn clean package -DskipTests
```
