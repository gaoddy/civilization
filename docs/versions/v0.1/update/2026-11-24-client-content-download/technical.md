方案未实施

# 客户端卡牌数据改为服务端下发

## 服务端

### 数据来源

- `server/data/v0.1/cards.json` 保持规则字段不变；展示字段 `display_cost`（随从固定 3，其余取 `cost`）只在下发 bundle 中附加，避免规则引擎的 `RawCard`（`deny_unknown_fields`）反序列化失败。
- 新增生成物 `server/data/v0.1/client-content.json`：由 `scripts/gen_client_data.py` 用 `gen_card_data.build()` 的同一份结果组装，结构与下发一致：

  ```json
  {"version": "<内容版本>", "cards": [...], "heroes": [...], "skills": [...], "taxonomy": {...}}
  ```

- 内容版本取 `cards`、`heroes`、`skills`、`taxonomy` 四段确定性 JSON 文本拼接后的 SHA-256 前 16 位十六进制；生成物随源提交，`--check` 检测漂移。

### 接口

- `GET /content`：返回 `client-content.json` 原文，`Content-Type: application/json; charset=utf-8`，`ETag: "<version>"`，`Cache-Control: no-cache`。
- 请求头 `If-None-Match` 与当前版本一致时返回 `304 Not Modified`，无响应体。
- 路由挂在现有 axum router（`net.rs`），与 `/ws`、`/healthz`、`/readyz` 同级；无鉴权，只读公开数据。
- 内容随构建 `include_str!` 内嵌，运行时不读磁盘；版本在首次请求时解析一次并缓存。

### 测试

- `content` 模块单测：版本非空、四个数据段可解析且非空。
- router `oneshot` 集成测试：无请求头返回 200 且 ETag 与版本一致；带匹配 `If-None-Match` 返回 304。
- `make data-check` 保证 `client-content.json` 与文档一致。

## 客户端

### 数据加载

- `CardDB` 保持 autoload 与现有查询接口不变，新增状态与信号：
  - `content_changed`：数据可用或更新后发出，供页面重建。
  - `content_failed(reason)`：无缓存且请求失败时发出。
  - `is_loaded()`、`has_failed()`、`fetch()`（重试）、`content_url_for(ws_url)`。
- 启动顺序：
  1. 设置 `CIVILIZATION_CONTENT_DIR` 时从该目录读取 `client-content.json`（本地调试、冒烟、线框图与预览专用，不进入导出包）。
  2. 否则同步读取 `user://content_cache.json`，有则先应用（离线可用）。
  3. 再发起 `GET /content`：`If-None-Match` 带缓存版本；`304` 保留缓存，`200` 覆盖缓存并应用；失败时保留缓存并标记不可用。
- 服务端地址由 `ws_client.gd` 的 `default_url()` 推导：`ws://` → `http://`、`wss://` → `https://`，去掉末尾 `/ws`，追加 `/content`。
- 缓存写入 `user://content_cache.json`（先写临时文件再改名），内容为下发原文。
- `Net.connected` 后若数据仍缺失或过期，自动重试一次。

### 页面

- 图鉴页：`CardDB.content_changed` 后重建目录；未就绪时叠加加载/失败提示，失败时提供重试按钮。
- 选人页：保持服务端候选 id 为准；数据未就绪时提示「内容未就绪」，收到 `content_changed` 后重建候选卡并重挂详情。
- 其他页面沿用现有查询接口；数据缺失时名称回退为 id，不阻塞页面。

### 文本与美术

- 卡名、技能文案、时代名等仍由本地文本表提供；美术索引与纹理仍随包。
- 服务端内容版本与客户端文本版本不绑定：新增实体缺文本时回退显示 id，作为已知边界记录。

### 构建与测试入口

- 删除 `client/generated/{cards,heroes,skills,taxonomy}.json`，保留同目录的 `protocol.gd` 与 `build_config.gd`。
- `client-smoke`、`ui-preview`、`ui-preview-check`、`ui-wireframe` 在命令前设置
  `CIVILIZATION_CONTENT_DIR="$(CURDIR)/server/data/v0.1"`，避免冒烟依赖真实服务端。
- `scripts/gen_client_data.py` 输出目录改为 `server/data/v0.1`，只产出 `client-content.json`；`make data` / `data-check` 入口不变。
- `tests/test_gen_data.py` 改为解析 bundle 并断言四个数据段与 `gen_card_data.build()` 一致；`tests/test_i18n.py` 检查目录改为 `server/data/v0.1`。

## 文档

- `docs/current/technical/client.md`：`CardDB` 数据来源、缓存与状态。
- `docs/current/product/ui/pages/gallery.md`：图鉴数据来源与加载/失败态。
- `docs/current/technical/rule-engine-and-simulator.md`：客户端生成物位置。
- `AGENTS.md`：卡牌数据生成物位置。

## 验证

- `make data` 后 `make data-check` 无差异。
- `cargo test -p room-service` 覆盖 `/content` 200/304。
- `make test`（Python）通过。
- `make client-import` 后 `make client-smoke SMOKE=gallery,cards` 通过。
- 真实联调：启动服务端后 `make client-run`，首次启动下载并展示图鉴；删除缓存后断网启动显示失败态。
