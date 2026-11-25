# 产品需求总表

本文登记全部产品需求的状态、实现版本与待定夺数量，并索引每个需求文档；需求规则以需求文档为准，本表不复述正文。

## 分类代码

| 代码 | 类型 | 模块 | 编号范围 |
| --- | --- | --- | --- |
| PLAT | 平台与发行 | 客户端 / 服务端 / 发行 | R001–R010 |
| CONT | 内容 | 内容 | R011–R018 |
| SYS | 玩法与系统 | 客户端 / 服务端 | R019–R031 |
| SOC | 社交与商业化 | 服务端 / 客户端 | R032–R033 |
| BAL | 平衡与质量 | 规则 / 模拟 | R034–R040 |
| ENG | 工程能力 | 服务端 / 客户端 | R041–R044 |
| DOC | 文档 | 文档 | R045–R047 |
| OPS | 发行运营 | 运营 | R048–R052 |

## 需求总表

| 编号 | 需求 | 状态 | 实现版本 | 待定夺 | 需求文档 |
| --- | --- | --- | --- | --- | --- |
| R001 | prod 服务端上线 | 实施中 | v0.2 | 1 项 | [platform-prod-deploy.md](platform-prod-deploy.md) |
| R002 | Android 真机验收 | 实施中 | v0.2 | 无 | [android-device-acceptance.md](android-device-acceptance.md) |
| R003 | Android 的 godotsteam 扩展处理 | 待确认 | v0.2 | 1 项 | [android-godotsteam-extension.md](android-godotsteam-extension.md) |
| R004 | 生产短信通道 | 待确认 | v0.2 | 1 项 | [production-sms-channel.md](production-sms-channel.md) |
| R005 | Steamworks 后台与商店页 | 待确认 | v0.2 | 1 项 | [steamworks-store.md](steamworks-store.md) |
| R006 | Steam 成就真实同步验收 | 实施中 | v0.2 | 无 | [steam-achievement-sync.md](steam-achievement-sync.md) |
| R007 | Apple 签名与公证 | 待确认 | v0.2 | 1 项 | [apple-signing-notarization.md](apple-signing-notarization.md) |
| R008 | Google Play 发布配置 | 待确认 | v0.2 | 无 | [google-play-release.md](google-play-release.md) |
| R009 | Windows 构建与 Steam depot | 待确认 | 待定 | 1 项 | [windows-build-steam-depot.md](windows-build-steam-depot.md) |
| R010 | 上线数据记录与查看 | 实施中 | v0.2 | 1 项 | [launch-telemetry.md](launch-telemetry.md) |
| R011 | v0.2 卡池增量 | 已确认 | v0.2 | 1 项 | [content-v02-card-pool.md](content-v02-card-pool.md) |
| R012 | v0.3 卡池增量 | 已确认 | v0.3 | 1 项 | [content-v03-card-pool.md](content-v03-card-pool.md) |
| R013 | v0.4 卡池增量 | 已确认 | v0.4 | 1 项 | [content-v04-card-pool.md](content-v04-card-pool.md) |
| R014 | 素材库存 152 张精选卡落地 | 待确认 | 待定 | 1 项 | [content-inventory-152-cards.md](content-inventory-152-cards.md) |
| R015 | 诺奖一人一卡全量名册 | 待确认 | 待定 | 1 项 | [content-nobel-laureates-cards.md](content-nobel-laureates-cards.md) |
| R016 | 未采用素材再评估 | 待确认 | 待定 | 1 项 | [content-unused-material-review.md](content-unused-material-review.md) |
| R017 | 逐卡音效 | 已确认 | 待定 | 1 项 | [content-per-card-sfx.md](content-per-card-sfx.md) |
| R018 | 内容多语言 | 待确认 | 待定 | 1 项 | [content-multilingual.md](content-multilingual.md) |
| R019 | 场景默认开启 | 已确认 | v0.3 | 无 | [system-scenarios-default-on.md](system-scenarios-default-on.md) |
| R020 | 大决战模式 | 已确认 | v0.3 | 无 | [system-showdown-mode.md](system-showdown-mode.md) |
| R021 | 英雄坐镇 | 已确认 | v0.3 | 无 | [system-hero-in-residence.md](system-hero-in-residence.md) |
| R022 | 陨落与遗嘱 | 已确认 | v0.3 | 无 | [system-fall-and-testament.md](system-fall-and-testament.md) |
| R023 | 赛季 | 待确认 | v0.3 | 1 项 | [system-season.md](system-season.md) |
| R024 | 剧情模式第一章 | 已确认 | v0.4 | 1 项 | [system-story-chapter-one.md](system-story-chapter-one.md) |
| R025 | 关卡编辑器与剧情文本 | 待确认 | v0.4 | 1 项 | [system-level-editor.md](system-level-editor.md) |
| R026 | 观战与赛事 | 待确认 | 待定 | 2 项 | [system-spectating-events.md](system-spectating-events.md) |
| R027 | 每日种子局与排行榜 | 待确认 | 待定 | 1 项 | [system-daily-seed-leaderboard.md](system-daily-seed-leaderboard.md) |
| R028 | 单人模式与难度递进 | 待确认 | 待定 | 1 项 | [system-single-player-difficulty.md](system-single-player-difficulty.md) |
| R029 | 局外解锁与教学局 | 待确认 | 待定 | 1 项 | [system-meta-unlock-tutorial.md](system-meta-unlock-tutorial.md) |
| R030 | 连续 run | 待确认 | 待定 | 1 项 | [system-continuous-run.md](system-continuous-run.md) |
| R031 | 违规与消极比赛处理 | 待确认 | 待定 | 1 项 | [system-misconduct-handling.md](system-misconduct-handling.md) |
| R032 | 好友榜与好友小队 | 已确认 | v0.3 | 1 项 | [social-friends-rank-squad.md](social-friends-rank-squad.md) |
| R033 | 商业化 | 待确认 | v0.3 | 2 项 | [commercialization.md](commercialization.md) |
| R034 | 13 个玩法夺冠率收敛 | 已确认 | 待定 | 无 | [balance-playstyle-win-rate.md](balance-playstyle-win-rate.md) |
| R035 | 平均回合数回落到 16–19 | 已确认 | 待定 | 无 | [balance-average-turns.md](balance-average-turns.md) |
| R036 | 单局 20–30 分钟目标验证 | 已确认 | 待定 | 无 | [balance-match-duration.md](balance-match-duration.md) |
| R037 | 其余待确认平衡项 | 待确认 | 待定 | 1 项 | [balance-open-questions.md](balance-open-questions.md) |
| R038 | 模拟报告字段补齐 | 已确认 | 待定 | 无 | [balance-sim-report-fields.md](balance-sim-report-fields.md) |
| R039 | 天梯积分与段位校准 | 待确认 | v0.2 | 1 项 | [balance-ranked-calibration.md](balance-ranked-calibration.md) |
| R040 | 已修复缺陷的环境复验 | 实施中 | v0.2 | 无 | [balance-fixed-defect-recheck.md](balance-fixed-defect-recheck.md) |
| R041 | 房间、对局状态、意图日志与遥测持久化 | 待确认 | 待定 | 1 项 | [engineering-state-persistence.md](engineering-state-persistence.md) |
| R042 | 多副本、多区部署与容量 | 待确认 | 待定 | 1 项 | [engineering-multi-replica-capacity.md](engineering-multi-replica-capacity.md) |
| R043 | 远端 `wss://` 压测 | 已确认 | 待定 | 无 | [engineering-remote-wss-loadtest.md](engineering-remote-wss-loadtest.md) |
| R044 | 客户端崩溃率采集 | 已确认 | v0.2 | 无 | [engineering-client-crash-rate.md](engineering-client-crash-rate.md) |
| R045 | v0.3 扩展项产品文档 | 已确认 | v0.3 | 无 | [docs-v03-product-docs.md](docs-v03-product-docs.md) |
| R046 | 第四版版本文档 | 已确认 | v0.4 | 无 | [docs-v04-version-docs.md](docs-v04-version-docs.md) |
| R047 | 登录页文档 | 已确认 | 待定 | 无 | [docs-login-page.md](docs-login-page.md) |
| R048 | Demo | 待确认 | 待定 | 1 项 | [ops-demo.md](ops-demo.md) |
| R049 | Steam 新品节 | 待确认 | 待定 | 1 项 | [ops-steam-next-fest.md](ops-steam-next-fest.md) |
| R050 | 愿望单 | 待确认 | 待定 | 1 项 | [ops-wishlist.md](ops-wishlist.md) |
| R051 | 宣发账号 | 待确认 | 待定 | 1 项 | [ops-marketing-accounts.md](ops-marketing-accounts.md) |
| R052 | 家长 / 教育渠道与 Apple Arcade | 待确认 | 待定 | 1 项 | [ops-education-apple-arcade.md](ops-education-apple-arcade.md) |

## 相关文档

- [版本路线与风险](../version-roadmap.md)
- [版本总览](../versions/README.md)
- [内容素材库](../content-backlog.md)
- [销量预测](../sales-forecast.md)
- [决策记录：AI 补位与托管](../decisions/0001-ai-bot-fill-and-no-takeover.md)
