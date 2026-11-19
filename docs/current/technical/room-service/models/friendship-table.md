# friendship 表

> 状态：已实现

本文记录 PostgreSQL 的 `friendship` 表定义与字段含义；不描述好友申请与邀请流程（见[房间服务与联网](../README.md)）。

## 原始定义

来源：`server/migrations/0001_baseline.sql`，`friendship` 与 `friendship_friend_idx`。

```sql
CREATE TABLE friendship (
    player_id TEXT NOT NULL REFERENCES account (account_id) ON DELETE CASCADE,
    friend_id TEXT NOT NULL REFERENCES account (account_id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ(3) NOT NULL,
    PRIMARY KEY (player_id, friend_id),
    CONSTRAINT friendship_not_self CHECK (player_id <> friend_id)
);

CREATE INDEX friendship_friend_idx ON friendship (friend_id);
```

## 字段说明

| `keypath` | 类型 | 说明 | 与产品的对应关系 |
| --- | --- | --- | --- |
| `player_id` | `text` | 关系一方的账号标识，外键 `account.account_id` | 不对外展示 |
| `friend_id` | `text` | 关系另一方的账号标识，外键 `account.account_id` | 不对外展示 |
| `created_at` | `timestamptz(3)` | 建立好友关系的时刻，由服务端 `Clock` 写入 | 不对外展示 |

## 主键、索引与约束

- 主键：`(player_id, friend_id)`。
- 外键：两列均引用 `account (account_id)`，`ON DELETE CASCADE`。
- 检查：`friendship_not_self` 禁止自己与自己建立关系。
- 索引：`friendship_friend_idx (friend_id)`，用于反查某人的好友。
- 关系成对写入：同意申请或建立关系时同时写入两个方向，删除时同时删除两个方向；读取到此表即双方互为好友，好友列表与邀请前提均以此为准，见[好友](../../../../versions/v0.1/product/systems/friends.md)。
