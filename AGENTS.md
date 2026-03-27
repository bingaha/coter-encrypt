# AGENTS.md

## 打包部署流程

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
