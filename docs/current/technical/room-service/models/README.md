# 房间服务数据结构

本文索引 `room-service` 模块的数据库表文档；领域模型、值对象与传输结构尚未单独成文，字段含义见[房间服务与联网](../README.md)。

| 文档 | 内容 |
| --- | --- |
| [account 表](account-table.md) | 手机号 → 持久化账号、6 位玩家编号与展示名 |
| [friendship 表](friendship-table.md) | 双向好友关系 |
| [friend_request 表](friend-request-table.md) | 待处理好友申请与发送时刻 |
| [friend_request_attempt 表](friend-request-attempt-table.md) | 24 小时内好友申请发送次数记录 |
| [recent_opponent 表](recent-opponent-table.md) | 最近对局对手与时间 |
| [room_invite 表](room-invite-table.md) | 房间邀请、房间码与过期时刻 |

全部表定义在基线迁移 `server/migrations/0001_baseline.sql`。
