---
name: film_record_lite
description: User mentions movie/film viewing records, ratings, or asks about films they've watched
---

# film_record_lite

A lightweight SQLite-based film record management REST API.

## Purpose

Help users manage their personal movie viewing records through natural language interaction.

## Connection Setup

### First-Time Connection

Ask user for connection details:

```
请提供FilmRecordLite服务的连接信息：
1. 服务URL（例如：http://your-server:8000）
2. 鉴权Token（至少8字符）
```

Save to `~/.film_record_lite_config.json`:

```json
{
  "server_url": "http://your-server:8000",
  "token": "YOUR_SECRET_TOKEN"
}
```

### Update Connection

User can update at any time:

```
用户: "更新服务地址为 http://new-server:9000"
→ Update server_url

用户: "更换Token为 newtoken123"
→ Update token
```

### Reading Config

Before each operation, read config. If missing, prompt user.

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/films` | Add single film |
| `POST` | `/films/batch` | Add multiple films |
| `GET` | `/films?title=xxx` | Search by title |
| `PUT` | `/films/rating` | Update rating |
| `DELETE` | `/films/{id}` | Delete by ID |
| `DELETE` | `/films?title=xxx` | Delete by title |
| `GET` | `/films/by-actor?actor=xxx` | Search by actor |
| `GET` | `/films/all` | List all films (paginated) |

## Authentication

Pass token via:
- **Header**: `Authorization: Bearer {token}`
- **Query**: `?token={token}`

## Curl Examples

### Add Film

```bash
curl -X POST "{server_url}/films?token={token}" \
  -H "Content-Type: application/json" \
  -d '{"title": "无间道", "actors": "刘德华,梁朝伟,黄秋生", "plot": "警察与黑帮互相卧底", "rating": 9}'
```

### Batch Add

Use when user mentions multiple movies:

```bash
curl -X POST "{server_url}/films/batch?token={token}" \
  -H "Content-Type: application/json" \
  -d '{
    "films": [
      {"title": "无间道", "rating": 9},
      {"title": "肖申克的救赎", "rating": 10},
      {"title": "霸王别姬", "rating": 9.5}
    ]
  }'
```

### Search Film

```bash
curl "{server_url}/films?token={token}&title=无间道"
```

### Update Rating

```bash
curl -X PUT "{server_url}/films/rating?token={token}" \
  -H "Content-Type: application/json" \
  -d '{"title": "无间道", "rating": 10}'
```

### List by Actor

```bash
# Search films by any actor (matches any actor in comma-separated list)
curl "{server_url}/films/by-actor?token={token}&actor=梁朝伟"
```

### List All

```bash
curl "{server_url}/films/all?token={token}"
```

### Paginated List

```bash
curl "{server_url}/films/all?token={token}&limit=10&offset=0"
```

### Delete Film

```bash
# Delete by ID
curl -X DELETE "{server_url}/films/1?token={token}"

# Delete by title (fuzzy match)
curl -X DELETE "{server_url}/films?token={token}&title=无间道"
```

## Response Format

Film records:

```
**无间道**
- 主演: 刘德华,梁朝伟,黄秋生
- 剧情: 警察和古惑仔互相卧底
- 评价: 很好看
- 评分: 9/10
- 记录日期: 2026-04-15
```

JSON response includes both formatted text and raw data.

## Field Formats

| Field | Format | Example |
|-------|--------|---------|
| `title` | 电影标题 | `"无间道"` |
| `actors` | 多主演逗号分隔 | `"刘德华,梁朝伟,黄秋生"` |
| `plot` | 剧情简介 | `"警察与黑帮互相卧底"` |
| `review` | 个人评价 | `"很好看，剧情紧凑"` |
| `rating` | 评分 0-10 | `9` 或 `9.5` |
| `record_date` | YYYY-MM-DD | `"2026-04-15"` |

## Best Practices

1. **Config check**: Read config before operations. If missing, ask user.

2. **Film info verification**: Before adding a film, search online to verify/correct user-provided information. Compare with official movie databases (IMDb, Douban, etc.) and correct any inaccuracies.

   **Fallback rule**: If search fails or exceeds 2 minutes timeout, skip verification and proceed with user-provided information directly.

3. **Multiple actors**: Store multiple actors as comma-separated string (e.g., `"刘德华,梁朝伟,黄秋生"`). Actor search uses LIKE match, so any actor in the list can be found.

4. **Batch adding**: Use `/films/batch` when user mentions multiple movies.

5. **Extract from natural language**: Parse title, actors, plot, review, rating.

6. **Fuzzy matching**: Title search matches partial strings.

7. **Chinese support**: All I/O in Chinese.

8. **Rating scale**: 0-10. "满分" → 10, "9分" → 9.

## Error Responses

- 401: Unauthorized (invalid/missing token)
- Film not found: `未找到标题包含「xxx」的电影`
- Empty database: `暂无电影记录`

## Example Conversation

### First Connection

```
Agent: 请提供FilmRecordLite服务的连接信息：
1. 服务URL
2. 鉴权Token

User: URL是 http://film.example.com:8000，Token是 mytoken123

Agent: 已保存连接配置。
```

### Single Film (with verification)

```
User: 我看了刘德华主演的无间道，评分9分

Agent: [Search online for "无间道" movie info]
Found: 无间道 (2002) 主演: 刘德华,梁朝伟,黄秋生,曾志伟
剧情: 警察与黑帮互相卧底的故事

Agent → POST /films {
  "title": "无间道",
  "actors": "刘德华,梁朝伟,黄秋生,曾志伟",
  "plot": "警察与黑帮互相卧底的故事",
  "rating": 9
}

Response:
{
  "id": 1,
  "title": "无间道",
  "actors": "刘德华,梁朝伟,黄秋生,曾志伟",
  "rating": 9.0
}
```

### Single Film (search failed - use user info)

```
User: 我看了某某某主演的小众电影XYZ，评分8分

Agent: [Search online for "XYZ" movie info]
Search failed or timed out (> 2 min)

Agent → POST /films {
  "title": "XYZ",
  "actors": "某某某",
  "rating": 8
}

Response:
{
  "id": 2,
  "title": "XYZ",
  "actors": "某某某",
  "rating": 8.0
}
```

### Multiple Films (with verification)

```
User: 我最近看了无间道9分、肖申克的救赎满分、霸王别姬9.5分

Agent: [Search online for each movie]
- 无间道 (2002): 主演 刘德华,梁朝伟 | 剧情 警察与黑帮互相卧底
- 肖申克的救赎 (1994): 主演 蒂姆·罗宾斯,摩根·弗里曼 | 剧情 被冤枉的银行家在监狱中的救赎
- 霸王别姬 (1993): 主演 张国荣,巩俐,张丰毅 | 剧情 京剧演员的人生悲剧

Agent → POST /films/batch {"films":[
  {"title":"无间道", "actors":"刘德华,梁朝伟", "plot":"警察与黑帮互相卧底", "rating":9},
  {"title":"肖申克的救赎", "actors":"蒂姆·罗宾斯,摩根·弗里曼", "plot":"被冤枉的银行家在监狱中的救赎", "rating":10},
  {"title":"霸王别姬", "actors":"张国荣,巩俐,张丰毅", "plot":"京剧演员的人生悲剧", "rating":9.5}
]}

Response:
{
  "message": "已成功添加 3 部电影记录",
  "count": 3
}
```

### Query

```
User: 我对无间道的评分是啥？

Agent → GET /films?title=无间道

Response:
{
  "message": "找到 1 部电影",
  "films": ["**无间道**\n- 主演: 刘德华,梁朝伟,黄秋生\n- 评分: 9/10"]
}

User: 改成10分

Agent → PUT /films/rating {"title":"无间道", "rating":10}

User: 我看过哪些梁朝伟的电影？

Agent → GET /films/by-actor?actor=梁朝伟

Response:
{
  "message": "「梁朝伟」主演的电影 (1 部)",
  "films": ["**无间道**\n- 主演: 刘德华,梁朝伟,黄秋生\n- 评分: 10/10"]
}
```