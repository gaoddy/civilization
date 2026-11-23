# account_achievement 表

> 状态：已实现

本文记录 PostgreSQL 的 `account_achievement` 表定义与字段含义；不描述成就触发与奖励（见[成就运行时](../../../../versions/v0.2/technical/achievements-runtime.md)）。

## 原始定义

来源：`server/migrations/0001_baseline.sql`，`account_achievement`。

```sql
CREATE TABLE account_achievement (
    account_id TEXT NOT NULL REFERENCES account (account_id) ON DELETE CASCADE,
    achievement_id TEXT NOT NULL CHECK (achievement_id <> ''),
    unlocked_at TIMESTAMPTZ(3) NOT NULL,
    PRIMARY KEY (account_id, achievement_id)
);
```

## 字段说明

| `keypath` | 类型 | 说明 | 与产品的对应关系 |
| --- | --- | --- | --- |
| `account_id` | `text` | 账号标识，外键 `account.account_id` | 不对外展示 |
| `achievement_id` | `text` | 成就稳定 id，与成就实体文档一致 | 认证后 `AchievementState` 下发的成就列表 |
| `unlocked_at` | `timestamptz(3)` | 首次解锁时刻，由服务端 `Clock` 写入 | 不对外展示 |

## 主键、索引与约束

- 主键：`(account_id, achievement_id)`，同一账号同一成就只保留首次解锁。
- 外键：`account_id` 引用 `account (account_id)`，`ON DELETE CASCADE`。
- 检查：`achievement_id` 不得为空串。
- 写入：首次达成时 `INSERT ... ON CONFLICT DO NOTHING`；写入结果决定是否向客户端下发 `AchievementUnlocked`。
- 载入：账号加载时一并读回，按成就 id 升序组装认证后的 `AchievementState`。
