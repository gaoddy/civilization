# 文明战旗

## Runbook

本项目遵循 [YongChun](https://github.com/gaoddy/YongChun) 仓库的 runbook。

- 场景与必读对应见 [runbooks/index.md](https://github.com/gaoddy/YongChun/blob/main/runbooks/index.md)；修改代码、文档或配置前按场景读取对应 runbook。
- Claude Code 命令层通过公开 marketplace 使用：`/plugin marketplace add gaoddy/YongChun` 后安装插件 `yongchun`；本地开发可用 `claude --plugin-dir <YongChun 本地路径>`。项目内不复制 runbook 或命令文件。
- 项目特有场景在本文件补充入口，指向上述 runbook；runbook 与本文件冲突时以 runbook 为准，并修正本文件。

## 协作原则

1. 使用中文，结论先行：先给结果或判断，再给必要依据；过程更新控制在 1–2 句。
2. 修改类任务按声明的 runbook 判定场景并直接执行，不反问是否走流程。
3. 先查仓库事实，只对无法查证的关键歧义提问；产品规则、验收标准与方案取舍不明确时先确认再动手。
4. 用户当轮限定的范围与边界优先于默认流程；推送、部署与发版只在明确要求时执行。
5. 文档与流程手册留在 runbook 仓库，本文件只保留项目事实与项目独有规则。

## 项目阶段

当前阶段：**开发阶段**。未上线前不为旧版本保留兼容逻辑或兼容设计；切换阶段时同步修改本节。

## 项目事实

### 产品

- 产品名：文明战旗（Civilization）；对外名称与简称均为「文明战旗」。
- 产品描述：8 人联网酒馆战棋，把炉石酒馆战棋的对局骨架、文明系列的时代演进与科学史的人物和概念装进同一副牌。
- 产品形态：带服务端、带客户端。
- 用户规模：v0.1 面向小规模内测，房间制、8 人一局，服务端单实例；不做公开匹配与容量承诺，大规模用户量目标在 v0.2 上线时确定。
- 客户端平台：macOS 桌面端与 iOS 已实现（Godot 4 + GDScript）；Android 端在 v0.2。
- 视觉主题：客户端 UI 只有深色一套，不实现浅色主题与主题切换；UI 验证不做浅色检查。
- 包名：`com.civilization.banner`（macOS bundle id），应用名 `Civilization`。
- 初始版本号：`v0.1`；版本记录在 `docs/versions/v0.1/`，已实现的产品与技术基线在 `docs/current/`。

### 仓库与入口

- 服务端：`server/`，Rust workspace（`protocol`、`protocol-gen`、`rule-engine`、`room-service`、`sim`），房间服务入口 `server/crates/room-service/src/main.rs`，默认监听 `127.0.0.1:8080`。
- 客户端：`client/`，Godot 4.7.2 + GDScript，入口场景 `client/pages/login/login.tscn`。
- 客户端引擎补丁版本固定 `4.7.2`（`make/client.mk` 的 `GODOT_VERSION`）；`make client-import` 与导出目标先校验本机引擎，版本不符直接失败。
- 规则引擎：`server/crates/rule-engine`，纯库，不依赖网络、数据库、时钟或文件 I/O；所有对局结算只在这里。
- 卡牌数据：唯一数据源在 `docs/current/product/content/`，生成到 `server/data/v0.1/`；其中 `client-content.json` 是客户端下发的展示内容（`GET /content`），客户端包内不内置卡牌数据；生成物随源提交。
- 协议契约：唯一来源 `protocol/schema/v1/`；生成物 `protocol/generated/protocol.rs`、`protocol/generated/protocol.gd` 与 `client/generated/protocol.gd` 随源提交。
- 数据库迁移：`server/migrations/`，启动时由 `sqlx::migrate!` 执行。
- 纹理压缩工具：`tools/asset-compress/`，独立 Rust crate（不属于 `server/` workspace），把 `client/assets/**/*.png` 编码成 Godot 可识别的 BPTC/ASTC 产物与 `.import`/`.md5`；入口 `tools/asset-compress/src/main.rs`。
- 部署：`deploy/helm/` 与 `server/Dockerfile`，入口为 `make deploy-*`；另有 `make deploy-dev` / `deploy-prod` 整包别名与 `deploy-<步骤>-<环境>` 单步别名。

### 命令与验证

- 顶层入口统一走根目录 `Makefile`，不手写等效命令。根 `Makefile` 只保留共享变量与验收入口，目标分册在 `make/`：`client.mk`（客户端构建、运行、检查、导出）、`content.mk`（内容、协议、资产、文档）、`server.mk`（服务端测试、模拟器、压测、联调）、`deploy.mk`（部署）。
- 全量检查：`make check`（协议生成物、文档数据、测试、静态检查、确定性）；基础检查 `make check-core`（不含协议生成物）；客户端另有 `make client-check`、`make client-smoke`（`TESTS=<模块>`、`SMOKE=<场景>` 只跑单项，逗号分隔可多选）。
- 页面线框图：`make ui-wireframe PAGE=gallery`（`ALL=1` 出全部页面，`DEVICE=phone` 出手机档位），无头解算真实布局后写到 `output/wireframes/`（忽略目录）。
- 运行态视图预览：`make ui-preview PAGE=game FIXTURE=game FOCUS=PlayersPanel`（`SCENE=` 任意场景、`SETUP=` 组件脚本、`DEVICE=phone` 手机档位），真实窗口直接查看；自检用 `make ui-preview-check ... EXPECT=SeatRow=8`。
- 纹理产物：`make asset-compress`（增量，源图未变按 source_md5 跳过）、`make asset-compress-check`（工具测试 + clippy + 60 张卡面 BC7 门禁）。
- 服务端测试：`make test`（Python unittest + `cargo test --workspace`）；静态检查：`make lint`。
- 文档与数据：`make data-check`、`python3 scripts/validate_docs.py`。
- 端到端入口：
  - 服务端真实 WebSocket：`cargo test -p room-service --test e2e`（8 个机器人打完一局）；
  - 断线重连一致性：`cargo test -p room-service --test reconnect`；
  - 客户端到服务端：`make client-e2e`。
- 模拟器基线：`make sim-determinism`、`make sim-batch`、`make sim-report`。
- 并发容量压测：`make load-build` 后 `make loadtest`（稳态，默认 10 局/30s）、`make loadtest-ramp`（爬坡找上限）、`make loadtest-smoke`（2 局/5s 冒烟）；报告写 `output/loadtest-*.json`。
- 本地联调：`make dev-bots` 起服务端与 7 个机器人；客户端用 `make client-run`，桌面端产物用 `make client-export`，iOS 真机用 `make client-run PLATFORM=ios`（默认第一台已连接的 iPhone，`IOS_DEVICE_TYPE=ipad` 改选 iPad）。
- 快速别名：基础命令 `make run` 与 `make client-run` 不带环境，`ENV` 可选（默认 `local`）；设备别名 `run-<设备>-<环境>`（设备取 `desktop`/`iphone`/`ipad`/`phone`/`tablet`，环境取 `local`/`dev`/`prod`，如 `run-desktop-dev`）与真机别名 `run-ios-<设备>-<环境>`（如 `run-ios-ipad-dev`）都显式带环境；`run-iphone`/`run-ipad`/`run-desktop`/`run-phone`/`run-tablet`/`run-ios` 保留为对应 `local` 快捷入口。导出别名 `export-<平台>-<环境>` 同时固定平台与环境，`export-macos`/`export-local`/`export-steam`/`export-ios`/`export-android`/`export-aab` 转发到 `client-export`；`export-ipad-<环境>`（`local`/`dev`/`prod`）转发到 `PLATFORM=ios`，iOS 为 iPhone/iPad 通用包，产物同 `export-ios-<环境>`。`ENV`、`SERVER_URL`、`STEAM_APP_ID` 等参数照常透传。
- 数据库：`make db-up`、`make db-test`、`make db-reset`。
- 慢命令阈值：`120s`；预计超过该时长的命令放后台执行，读末尾输出与退出状态。

### 本地产物目录 output/

- `output/` 是纯缓存目录：任何内容都可随时整体删除，不保证下次还在。真源（素材源、发布记录、实验结论）一律不放这里。
- 固定路径只由 `make` 目标写入，不手改：`ios/`、`desktop/`、`desktop-local/`、`steam/`、`android/`、`wireframes/`、`ui_smoke_*.log`、`ui_preview.log`、`loadtest-*.json`、`sim-batch.json`、`sim-report.log`。
- 一次性探针、截图与临时数据统一放 `output/scratch/<日期>-<主题>/`；任务结束前要么迁到真源位置，要么删除。
- 效果牌封面源按需临时放到 `output/effect-covers/generated/`（`ASSET_COVERS` 默认值），`make assets-covers` 处理完即可删除。
- `make font-subset` 自动把源字体下载到 `output/fonts/`，并在缺少 fontTools 时创建 `output/fonttools-venv/`；两者都是缓存，删掉后重跑目标即可恢复。
- 服务端发布记录写到 `docs/current/technical/deployment/releases/<日期>-<env>-<提交>.md`，不写进 `output/`。

### 文档

- `docs/` 骨架：`current/` 写已实现状态，`versions/<版本>/` 写未实现或规划中的版本，`docs/plan/` 放实施计划（本地忽略、不提交）。
- 版本入口：`docs/versions/v0.1/README.md`；验收记录：`docs/versions/v0.1/acceptance.md`；平衡问题：`docs/current/product/content/balance-questions.md`。
- 跨版本路线：`docs/version-roadmap.md`。
