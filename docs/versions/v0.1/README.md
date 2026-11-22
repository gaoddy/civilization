# v0.1

本文是 v0.1 的版本入口，描述版本主题、目标、能力清单、状态与文档导航；已实现的产品与技术基线在 [docs/current/](../../current/README.md)，本文件只保留版本记录。

> 状态：已完成（M0–M3 与剩余四项能力均已实现；M2 断线重连一致性由自动化用例覆盖，M3 不做单独验收记录）

## 主题

《文明战旗》首版：把炉石酒馆战棋的对局骨架、文明系列的时代演进与科学史的人物和概念装进同一副牌，8 人联网对战、20–30 分钟一局。

## 目标

1. 新玩家第一局能在 3 分钟内完成一次购买与出战；
2. 老玩家能连续打 20 局而不觉得每局都一样；
3. 8 人完整对局在 20–30 分钟内结束；
4. 前 100 局不出现单一玩法胜率超过 60%。

## 能力清单

| 内容 | 范围 |
| --- | --- |
| 对局 | 8 名真人、40 点生命、招募 / 战斗 / 结算三段状态机；不设 AI 补位 |
| 时代 | 7 个：史前、原始、古典、封建、工业、现代、未来 |
| 玩法 | 13 个，每局随机启用 5 个 |
| 卡牌 | 332 张随从 + 100 张效果牌，共 432 张（其中 24 张 `in_shop_pool: false` 的衍生物与生成卡）；另有 32 张独立英雄卡（以 `make data-check` 输出为准） |
| 星级 | 1–7 |
| 英雄卡 | 32 张，技能写在卡的 `skills` 中；开局每人从服务端分配的 4 张候选中选 1 张 |
| 技能 | 255 个，由卡牌与英雄卡的 `skills` 按用途引用（以 `make data-check` 输出为准） |
| 关键词 | 16 个：通用 12 个 + 装配、播种、进化、分解专属 4 个 |
| 模式 | 会战（默认）；房间制、不计分；v0.1 建房不提供场景/陨落开关 |
| 成局 | 创建房间、6 位房间码、好友邀请、房主移除成员/解散房间；对局结束即销毁房间，结算页点「确定」仅导航回大厅；首版不做自动匹配 |
| 账号 | 仅手机号验证码登录；好友与房间状态由服务端同步 |
| 好友 | 最近对局与玩家 ID 添加、申请、列表、邀请加入房间 |
| 赛后回顾 | 结算页展示本人本局统计与 8 人名次，只覆盖本局；不做跨对局统计、排行榜与服务端后台 |
| 客户端 | macOS 桌面端与 iOS 端（iPhone/iPad，开发签名真机包），Godot 4 + GDScript；规则只在服务端运行；导出见[客户端技术现状](../../current/technical/client.md) §8、§9 |
| 音效 | 刷新、冻结/解冻、购入、攻击、死亡、回合开始商店入场与战斗开始/结束；卡牌音效按卡牌 id 挂钩，当前统一使用默认公共音效 |
| 服务端 | Rust；规则引擎与房间服务同进程，权威结算全部在服务端 |
| 新手引导 | 首局非阻塞引导：高亮与提示条、可跳过与手动重看；不做教学局 |
| 扩展层 | 场景、陨落、英雄坐镇、大决战只保留设计与配置项；v0.1 建房不提供开关，默认关闭 |

M0–M1b 之后的首版剩余能力已实现：赛后数据回顾、房间生命周期（移除/解散/对局结束销毁/主动退出）、首局界面引导与 macOS 桌面端导出。已实现状态以 `docs/current/` 为准。验收口径：M2 不设真人完成率验收，断线重连后状态一致由自动化用例覆盖；M3 不做单独验收记录，桌面端完整对局与崩溃率、意图拒绝率、单局时长不记入验收文档。自动化验收结果见[验收记录](acceptance.md)。

## 阅读顺序

按依赖链阅读：总览与规则 → 数值 → 内容 → 系统规则 → 技术约束。

1. [设计总纲](../../current/product/overview/design-overview.md) 与[概念与定位](../../current/product/overview/concept-and-positioning.md)
2. [核心规则](../../current/product/rules/core-rules.md)、[关键词与机制](../../current/product/rules/keywords-and-mechanics.md)、[时代与场景](../../current/product/rules/eras-and-scenarios.md)
3. [数值与经济](../../current/product/systems/economy-and-numbers.md)
4. [卡牌规范与数据](../../current/product/content/card-system.md) 与[卡牌索引](../../current/product/content/cards/README.md)；英雄卡见[英雄卡索引](../../current/product/content/heroes/README.md)
5. [技能规范与数据](../../current/product/content/skill-system.md) 与[技能索引](../../current/product/content/skills/README.md)
6. [平衡测算](../../current/product/content/balance-analysis.md)（从卡表回算的校验输出）与[平衡问题](../../current/product/content/balance-questions.md)（未解决的平衡与节奏问题）
7. [模式](../../current/product/systems/modes.md)、[房间与邀请](../../current/product/systems/rooms-and-invites.md) 与[好友](../../current/product/systems/friends.md)
8. [联网与技术架构](../../current/technical/architecture/network-architecture.md)

## 文档导航

- 产品状态：[产品状态](../../current/product/README.md)
- 技术现状：[技术现状](../../current/technical/README.md)
- 玩家向介绍：[游戏介绍](../../current/product/overview/game-intro.md)
- 未确认事项：[open-questions.md](open-questions.md)
- 平衡问题：[balance-questions.md](../../current/product/content/balance-questions.md)
- 验收记录：[acceptance.md](acceptance.md)
- 已实现现状：[当前实现状态](../../current/README.md)
- 跨版本路线：[版本路线与风险](../../version-roadmap.md)、[内容素材库](../../content-backlog.md)

## 数据与校验

卡牌与技能实体文档是唯一数据源；卡牌字段规范（英雄卡见 §2.1）见[卡牌规范与数据](../../current/product/content/card-system.md)，技能字段规范见[技能规范与数据](../../current/product/content/skill-system.md)；每次修改后运行 `python3 scripts/validate_docs.py`。
