# Quick Start - 本地开发环境

## 1. Docker Compose 启动基础服务

### PostgreSQL + Redis + Kafka

```bash
# 启动所有基础服务
docker-compose -f deploy/docker-compose.yml up -d

# 查看服务状态
docker-compose -f deploy/docker-compose.yml ps
```

### 默认配置

| 服务 | 端口 | 用户 | 密码 |
|------|------|------|------|
| PostgreSQL | 5432 | postgres | postgres |
| Redis | 6379 | - | - |
| Kafka | 9092 | - | - |

---

## 2. Rust 开发环境

### 安装 Rust

```bash
# 安装 rustup (如果没有)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 激活工具链
source ~/.cargo/env

# 验证安装
rustc --version
rustc 1.78.0
```

### 安装依赖

```bash
# 安装 build-essential (Linux)
sudo apt install build-essential pkg-config libssl-dev

# macOS
brew install pkg-config openssl
```

---

## 3. 运行项目

### 下载依赖

```bash
cargo fetch
```

### 编译项目

```bash
# Debug 模式
cargo build

# Release 模式 (推荐生产)
cargo build --release
```

### 运行服务

```bash
# API 服务 (端口 8080)
cargo run -p job-api

# 任务执行器 (端口 8081)
cargo run -p job-runner

# 缓存消费者 (端口 8082)
cargo run -p job-cache-consumer

# WebSocket 桥接 (端口 8083)
cargo run -p job-ws-bridge
```

---

## 4. 数据库迁移

```bash
# 安装 sqlx-cli
cargo install sqlx-cli

# 运行迁移
DATABASE_URL=postgres://postgres:postgres@localhost:5432/zquant sqlx migrate run
```

---

## 5. 环境变量

创建 `.env` 文件:

```bash
# 数据库
DATABASE_URL=postgres://postgres:postgres@localhost:5432/zquant

# Redis
REDIS_URL=redis://localhost:6379

# Kafka
KAFKA_BROKERS=localhost:9092

# 服务配置
API_HOST=0.0.0.0
API_PORT=8080
```

---

## 6. Docker 构建 (生产)

```bash
# 构建所有服务镜像
docker build -t zquant/job-api -f apps/job-api/Dockerfile .
docker build -t zquant/job-runner -f apps/job-runner/Dockerfile .

# 或使用 docker-compose
docker-compose -f deploy/docker-compose.prod.yml up -d --build
```
