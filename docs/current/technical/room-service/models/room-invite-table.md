# room_invite 表

> 状态：已实现

本文记录 PostgreSQL 的 `room_invite` 表定义与字段含义；不描述邀请校验与接受后的入房流程（见[房间服务与联网](../README.md)）。

## 原始定义

来源：`server/migrations/0001_baseline.sql`，`room_invite`、`room_invite_to_idx` 与 `room_invite_expiry_idx`。

```sql
CREATE TABLE room_invite (
    invite_id TEXT PRIMARY KEY,
    from_account_id TEXT NOT NULL REFERENCES account (account_id) ON DELETE CASCADE,
    to_account_id TEXT NOT NULL REFERENCES account (account_id) ON DELETE CASCADE,
    room_id TEXT NOT NULL,
    room_code TEXT NOT NULL,
    expires_at TIMESTAMPTZ(3) NOT NULL,
    created_at TIMESTAMPTZ(3) NOT NULL,
    CONSTRAINT room_invite_not_self CHECK (from_account_id <> to_account_id)
);

CREATE INDEX room_invite_to_idx ON room_invite (to_account_id);
CREATE INDEX room_invite_expiry_idx ON room_invite (expires_at);
```

## 字段说明

| `keypath` | 类型 | 说明 | 与产品的对应关系 |
| --- | --- | --- | --- |
| `invite_id` | `text` | 邀请标识，主键，形如 `inv_<uuid>` | 邀请弹窗回执用的邀请编号 |
| `from_account_id` | `text` | 邀请人账号标识，外键 `account.account_id` | 邀请弹窗展示的邀请人，见[房间与邀请](../../../product/systems/rooms-and-invites.md) |
| `to_account_id` | `text` | 被邀请人账号标识，外键 `account.account_id` | 邀请弹窗的接收者 |
| `room_id` | `text` | 目标房间标识，仅进程内有效 | 不对外展示 |
| `room_code` | `text` | 目标房间码，6 位大写字母数字 | 邀请弹窗展示、接受后 `JoinRoom` 使用的房间码 |
| `expires_at` | `timestamptz(3)` | 过期时刻，创建时刻加 60 秒 | 邀请弹窗的有效期与倒计时 |
| `created_at` | `timestamptz(3)` | 创建时刻，由服务端 `Clock` 写入 | 不对外展示 |

## 主键、索引与约束

- 主键：`invite_id`。
- 外键：两个账号列均引用 `account (account_id)`，`ON DELETE CASCADE`。
- 检查：`room_invite_not_self` 禁止邀请自己。
- 索引：`room_invite_to_idx (to_account_id)` 用于被邀请人读取；`room_invite_expiry_idx (expires_at)` 用于清理过期邀请。
- 生命周期：创建时先删除已过期行再写入；响应（接受或拒绝）后删除；读取时清理过期行。房间本身不落盘，重启后已有邀请指向的房间不存在，接受时按 `ROOM_NOT_FOUND` 失败。
