# friend_request_attempt 表

> 状态：已实现

本文记录 PostgreSQL 的 `friend_request_attempt` 表定义与字段含义；不描述申请规则本身（见[房间服务与联网](../README.md)）。

## 原始定义

来源：`server/migrations/0001_baseline.sql`，`friend_request_attempt` 与 `friend_request_attempt_pair_idx`。

```sql
CREATE TABLE friend_request_attempt (
    friend_request_attempt_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    from_account_id TEXT NOT NULL REFERENCES account (account_id) ON DELETE CASCADE,
    to_account_id TEXT NOT NULL REFERENCES account (account_id) ON DELETE CASCADE,
    sent_at TIMESTAMPTZ(3) NOT NULL,
    CONSTRAINT friend_request_attempt_not_self CHECK (from_account_id <> to_account_id)
);

CREATE INDEX friend_request_attempt_pair_idx
    ON friend_request_attempt (from_account_id, to_account_id, sent_at DESC);
```

## 字段说明

| `keypath` | 类型 | 说明 | 与产品的对应关系 |
| --- | --- | --- | --- |
| `friend_request_attempt_id` | `bigint` | 自增主键，仅用于唯一标识一次发送 | 不对外展示 |
| `from_account_id` | `text` | 申请人账号标识，外键 `account.account_id` | 不对外展示 |
| `to_account_id` | `text` | 收件人账号标识，外键 `account.account_id` | 不对外展示 |
| `sent_at` | `timestamptz(3)` | 发送时刻，由服务端 `Clock` 写入；超过 24 小时不再计入 | 被拒绝后的重发限制，见[好友](../../../product/systems/friends.md) |

## 主键、索引与约束

- 主键：`friend_request_attempt_id`（`GENERATED ALWAYS AS IDENTITY`）。
- 外键：两个账号列均引用 `account (account_id)`，`ON DELETE CASCADE`。
- 检查：`friend_request_attempt_not_self` 禁止给自己发送。
- 索引：`friend_request_attempt_pair_idx (from_account_id, to_account_id, sent_at DESC)`。
- 用途：只追加，不因申请被处理而删除；同意后清空双方的发送记录，拒绝保留发送记录，24 小时内同一对申请人最多再发 3 次（超出回 `FRIEND_REQUEST_LIMIT`）。读取时清理超过 24 小时的行。
