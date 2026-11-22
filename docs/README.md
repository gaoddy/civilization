# 文档索引

本文是仓库文档总索引，列出目录职责、版本入口与跨版本文档；不重复正文内容。

## 目录职责

| 路径 | 职责 |
| --- | --- |
| [versions/](versions/README.md) | 未实现版本的规划与目标形态 |
| [current/](current/README.md) | 当前代码已实现的状态；只写现状，不复制版本规划 |
| `releases/` | 已发布版本的更新内容；当前没有发版记录，首次发版时再建 |
| `plan/` | 实施计划，本地保留、不提交 |

## 版本

| 版本 | 状态 | 文档 |
| --- | --- | --- |
| v0.1 | 已完成 | [v0.1](versions/v0.1/README.md) |
| v0.2 | 规划中 | [v0.2](versions/v0.2/README.md) |

版本状态与后续版本的内容增量见[版本总览](versions/README.md)。

## 跨版本文档

| 文档 | 内容 |
| --- | --- |
| [版本路线与风险](version-roadmap.md) | 开发顺序、里程碑、各版本内容增量、风险与止损线 |
| [内容素材库](content-backlog.md) | 未进入首版卡池、保留给后续版本的素材与再评估方向 |

## 数据与校验

卡牌实体文档是唯一数据源，索引于[卡牌索引](current/product/content/cards/README.md)；32 张英雄卡在 `content/heroes/` 单独索引于[英雄卡索引](current/product/content/heroes/README.md)；一致性由 `python3 scripts/validate_docs.py` 校验。
