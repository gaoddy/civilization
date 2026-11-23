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
