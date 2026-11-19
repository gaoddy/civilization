# friend_request 表

> 状态：已实现

本文记录 PostgreSQL 的 `friend_request` 表定义与字段含义；不描述申请次数限制与拒绝回执（见[房间服务与联网](../README.md)）。

## 原始定义

来源：`server/migrations/0001_baseline.sql`，`friend_request` 与 `friend_request_to_idx`。

```sql
CREATE TABLE friend_request (
    from_account_id TEXT NOT NULL REFERENCES account (account_id) ON DELETE CASCADE,
    to_account_id TEXT NOT NULL REFERENCES account (account_id) ON DELETE CASCADE,
    sent_at TIMESTAMPTZ(3) NOT NULL,
    PRIMARY KEY (from_account_id, to_account_id),
    CONSTRAINT friend_request_not_self CHECK (from_account_id <> to_account_id)
);

CREATE INDEX friend_request_to_idx ON friend_request (to_account_id, sent_at DESC);
```

## 字段说明

| `keypath` | 类型 | 说明 | 与产品的对应关系 |
| --- | --- | --- | --- |
| `from_account_id` | `text` | 申请人账号标识，外键 `account.account_id` | 申请卡片展示的申请人，见[好友](../../../../versions/v0.1/product/systems/friends.md) |
| `to_account_id` | `text` | 收件人账号标识，外键 `account.account_id` | 不对外展示 |
| `sent_at` | `timestamptz(3)` | 申请提交时刻，由服务端 `Clock` 写入；超过 7 天视为过期 | 申请卡片展示的发送时间 |

## 主键、索引与约束

- 主键：`(from_account_id, to_account_id)`，同一对玩家最多一条待处理申请。
- 外键：两列均引用 `account (account_id)`，`ON DELETE CASCADE`。
- 检查：`friend_request_not_self` 禁止给自己发申请。
- 索引：`friend_request_to_idx (to_account_id, sent_at DESC)`，用于收件人读取待处理申请。
- 过期：读取时清理 `sent_at` 超过 7 天的行；处理成功（同意或拒绝）后删除该行。
