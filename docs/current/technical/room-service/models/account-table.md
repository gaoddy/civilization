# account 表

> 状态：已实现

本文记录 PostgreSQL 的 `account` 表定义与字段含义；不描述登录流程（见[房间服务与联网](../README.md)）。

## 原始定义

来源：`server/migrations/0001_baseline.sql`，`player_code_seq`、`account` 与 `account_display_name_idx`。

```sql
CREATE SEQUENCE player_code_seq AS BIGINT START WITH 100000 MAXVALUE 999999 CYCLE;

CREATE TABLE account (
    account_id TEXT PRIMARY KEY,
    phone TEXT NOT NULL UNIQUE,
    player_code TEXT NOT NULL UNIQUE CHECK (player_code ~ '^[0-9]{6}$'),
    display_name TEXT NOT NULL CHECK (display_name <> ''),
    ordinal BIGINT GENERATED ALWAYS AS IDENTITY UNIQUE,
    created_at TIMESTAMPTZ(3) NOT NULL,
    updated_at TIMESTAMPTZ(3) NOT NULL
);

CREATE INDEX account_display_name_idx ON account (display_name, account_id);
```

## 字段说明

| `keypath` | 类型 | 说明 | 与产品的对应关系 |
| --- | --- | --- | --- |
| `account_id` | `text` | 账号标识，主键；服务内即 `playerId` | 不对外展示 |
| `phone` | `text` | 登录手机号，唯一；同一手机号恢复同一账号 | 登录凭据，不对外展示 |
| `player_code` | `text` | 6 位数字玩家编号，唯一，`100000`–`999999` 循环分配 | 好友搜索、邀请与好友列表展示的玩家编号，见[好友](../../../../versions/v0.1/product/systems/friends.md) |
| `display_name` | `text` | 展示名，非空；首次创建为 `玩家N`，重复登录不覆盖 | 大厅、房间座位与好友列表展示的名字 |
| `ordinal` | `bigint` | 持久化自增序号，唯一 | 默认展示名 `玩家N` 的 `N` |
| `created_at` | `timestamptz(3)` | 创建时刻，由服务端 `Clock` 写入 | 不对外展示 |
| `updated_at` | `timestamptz(3)` | 最近更新时刻，由服务端 `Clock` 写入 | 不对外展示 |

## 主键、索引与约束

- 主键：`account_id`。
- 唯一：`phone`、`player_code`、`ordinal`。
- 检查：`player_code` 必须匹配 `^[0-9]{6}$`；`display_name` 不得为空串。
- 索引：`account_display_name_idx (display_name, account_id)`，用于好友列表按展示名排序。
- 编号来源：`player_code_seq` 从 `100000` 起、`999999` 封顶并循环；编号用尽时写入失败并返回 `INTERNAL_ERROR`。
- 外键：其它五张表的玩家列均引用本表 `account_id`，删除账号时级联删除。
