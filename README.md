# film_record_lite

一个使用 Rust 和 SQLite 实现的轻量级观影记录 API 服务，可与 LLM 交互来管理你的电影记录。

## 服务端安装

### Docker 部署（推荐）

镜像地址：`ghcr.io/pzweuj/film_record_lite:latest`

```bash
# 设置Token（至少8字符）并启动
export FILM_RECORD_TOKEN=your_secure_token_min_8_chars
docker compose up -d
```

数据通过 volume 挂载到 `./data` 目录持久化。

### 本地运行

```bash
FILM_RECORD_TOKEN=your_token cargo run --release -- --port 8000
```

服务会直接使用 `data/films.db`。从旧版本升级时不需要迁移数据库、更换 Token 或修改客户端调用。

也可以先编译为单一可执行文件：

```bash
cargo build --release --locked
FILM_RECORD_TOKEN=your_token ./target/release/film-record-lite --port 8000
```

启动后访问 Swagger 文档：`http://localhost:8000/docs`

## SKILL 安装

将 `SKILL.md` 内容导入你的 AI Agent（OpenClaw、Hermes-Agent等等）。

首次使用时，Agent 会询问服务 URL 和 Token，保存配置后即可通过自然语言管理观影记录。
