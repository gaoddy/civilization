# recent_opponent 表

> 状态：已实现

本文记录 PostgreSQL 的 `recent_opponent` 表定义与字段含义；不描述开局与社交视图（见[房间服务与联网](../README.md)）。

## 原始定义

来源：`server/migrations/0001_baseline.sql`，`recent_opponent` 与 `recent_opponent_order_idx`。

```sql
CREATE TABLE recent_opponent (
    player_id TEXT NOT NULL REFERENCES account (account_id) ON DELETE CASCADE,
    opponent_id TEXT NOT NULL REFERENCES account (account_id) ON DELETE CASCADE,
    last_played_at TIMESTAMPTZ(3) NOT NULL,
    PRIMARY KEY (player_id, opponent_id),
    CONSTRAINT recent_opponent_not_self CHECK (player_id <> opponent_id)
);

CREATE INDEX recent_opponent_order_idx ON recent_opponent (player_id, last_played_at DESC);
```

## 字段说明

| `keypath` | 类型 | 说明 | 与产品的对应关系 |
| --- | --- | --- | --- |
| `player_id` | `text` | 记录所属玩家账号标识，外键 `account.account_id` | 不对外展示 |
| `opponent_id` | `text` | 同局对手账号标识，外键 `account.account_id` | 大厅「最近对局」列表展示的对手 |
| `last_played_at` | `timestamptz(3)` | 最近一次同局时刻，由服务端 `Clock` 写入 | 最近对局按时间倒序展示 |

## 主键、索引与约束

- 主键：`(player_id, opponent_id)`，同一玩家同一对手只保留一行。
- 外键：两列均引用 `account (account_id)`，`ON DELETE CASCADE`。
- 检查：`recent_opponent_not_self` 禁止自己与自己配对。
- 索引：`recent_opponent_order_idx (player_id, last_played_at DESC)`。
- 写入：对局开始时对每个参战座位与同局其他座位逐个写入或更新时间；每人只保留最近 20 名对手，超出部分在同一事务内删除。
