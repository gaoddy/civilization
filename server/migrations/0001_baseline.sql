-- v0.1 基线：账号身份、好友关系、好友申请、最近对局与房间邀请。
-- 开发阶段 schema 改动直接修改本文件；已上线后冻结，改走按序 migration。
-- 所有时间列带时区、精度毫秒；时间值由服务端 Clock 写入，不使用 now()。

CREATE SEQUENCE player_code_seq AS BIGINT START WITH 100000 MAXVALUE 999999 CYCLE;

CREATE TABLE account (
    account_id TEXT PRIMARY KEY,
    phone TEXT UNIQUE,
    steam_id TEXT UNIQUE,
    player_code TEXT NOT NULL UNIQUE CHECK (player_code ~ '^[0-9]{6}$'),
    display_name TEXT NOT NULL CHECK (display_name <> ''),
    ordinal BIGINT GENERATED ALWAYS AS IDENTITY UNIQUE,
    created_at TIMESTAMPTZ(3) NOT NULL,
    updated_at TIMESTAMPTZ(3) NOT NULL,
    CHECK (phone IS NOT NULL OR steam_id IS NOT NULL)
);

CREATE INDEX account_display_name_idx ON account (display_name, account_id);

CREATE TABLE friendship (
    player_id TEXT NOT NULL REFERENCES account (account_id) ON DELETE CASCADE,
    friend_id TEXT NOT NULL REFERENCES account (account_id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ(3) NOT NULL,
    PRIMARY KEY (player_id, friend_id),
    CONSTRAINT friendship_not_self CHECK (player_id <> friend_id)
);

CREATE INDEX friendship_friend_idx ON friendship (friend_id);

CREATE TABLE friend_request (
    from_account_id TEXT NOT NULL REFERENCES account (account_id) ON DELETE CASCADE,
    to_account_id TEXT NOT NULL REFERENCES account (account_id) ON DELETE CASCADE,
    sent_at TIMESTAMPTZ(3) NOT NULL,
    PRIMARY KEY (from_account_id, to_account_id),
    CONSTRAINT friend_request_not_self CHECK (from_account_id <> to_account_id)
);

CREATE INDEX friend_request_to_idx ON friend_request (to_account_id, sent_at DESC);

CREATE TABLE friend_request_attempt (
    friend_request_attempt_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    from_account_id TEXT NOT NULL REFERENCES account (account_id) ON DELETE CASCADE,
    to_account_id TEXT NOT NULL REFERENCES account (account_id) ON DELETE CASCADE,
    sent_at TIMESTAMPTZ(3) NOT NULL,
    CONSTRAINT friend_request_attempt_not_self CHECK (from_account_id <> to_account_id)
);

CREATE INDEX friend_request_attempt_pair_idx
    ON friend_request_attempt (from_account_id, to_account_id, sent_at DESC);

CREATE TABLE recent_opponent (
    player_id TEXT NOT NULL REFERENCES account (account_id) ON DELETE CASCADE,
    opponent_id TEXT NOT NULL REFERENCES account (account_id) ON DELETE CASCADE,
    last_played_at TIMESTAMPTZ(3) NOT NULL,
    PRIMARY KEY (player_id, opponent_id),
    CONSTRAINT recent_opponent_not_self CHECK (player_id <> opponent_id)
);

CREATE INDEX recent_opponent_order_idx ON recent_opponent (player_id, last_played_at DESC);

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

-- 账号级成就解锁：只记录每个账号第一次达成，用于认证时下发与 Steam 侧只同步一次。
CREATE TABLE account_achievement (
    account_id TEXT NOT NULL REFERENCES account (account_id) ON DELETE CASCADE,
    achievement_id TEXT NOT NULL CHECK (achievement_id <> ''),
    unlocked_at TIMESTAMPTZ(3) NOT NULL,
    PRIMARY KEY (account_id, achievement_id)
);

-- 天梯积分：新账号首次结算时插入，初始 1000；未插入时按 1000 处理。
-- updated_at 记录最近一次积分变化时刻，同分按它先到先得。
CREATE TABLE account_rating (
    account_id TEXT PRIMARY KEY REFERENCES account (account_id) ON DELETE CASCADE,
    rating INTEGER NOT NULL DEFAULT 1000 CHECK (rating >= 0),
    highest_rating INTEGER NOT NULL DEFAULT 1000 CHECK (highest_rating >= 0),
    updated_at TIMESTAMPTZ(3) NOT NULL
);

CREATE INDEX account_rating_leaderboard_idx
    ON account_rating (rating DESC, updated_at ASC, account_id ASC);

-- 一局天梯对局；房间局不写此表。
CREATE TABLE ranked_match (
    match_id TEXT PRIMARY KEY,
    room_id TEXT NOT NULL,
    finished_at TIMESTAMPTZ(3) NOT NULL,
    seed TEXT NOT NULL
);

CREATE INDEX ranked_match_finished_idx ON ranked_match (finished_at DESC);

-- 每人结算结果：最近 10 局与历史最高段位的数据来源。
CREATE TABLE ranked_result (
    match_id TEXT NOT NULL REFERENCES ranked_match (match_id) ON DELETE CASCADE,
    account_id TEXT NOT NULL REFERENCES account (account_id) ON DELETE CASCADE,
    placement SMALLINT NOT NULL CHECK (placement BETWEEN 1 AND 8),
    rating_before INTEGER NOT NULL CHECK (rating_before >= 0),
    rating_delta INTEGER NOT NULL,
    rating_after INTEGER NOT NULL CHECK (rating_after >= 0),
    PRIMARY KEY (match_id, account_id)
);

CREATE INDEX ranked_result_account_idx ON ranked_result (account_id);

-- 宗师榜每日快照：只保留前 master_rank_limit 名，作为次日宗师判定依据。
CREATE TABLE master_snapshot (
    snapshot_day DATE NOT NULL,
    account_id TEXT NOT NULL REFERENCES account (account_id) ON DELETE CASCADE,
    rank INTEGER NOT NULL CHECK (rank >= 1),
    PRIMARY KEY (snapshot_day, account_id)
);

CREATE INDEX master_snapshot_rank_idx ON master_snapshot (snapshot_day, rank);
