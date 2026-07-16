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
