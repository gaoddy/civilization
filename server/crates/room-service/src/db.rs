//! PostgreSQL 基础设施：连接池、基线迁移与全部 SQL 落点。
//!
//! 本模块是服务端唯一的 SQL 所在处：`account` 与 `social` 只调用这里的方法，不拼
//! SQL。连接由入口装配，业务模块不读取环境变量。

use std::str::FromStr;
use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use sqlx::{Executor, FromRow};
use uuid::Uuid;

/// 数据库操作结果。
pub type DbResult<T> = Result<T, sqlx::Error>;

/// Unix epoch 毫秒转 UTC；超出可表示范围时退化为 epoch 起点。
pub fn ms_to_utc(ms: u64) -> DateTime<Utc> {
    i64::try_from(ms)
        .ok()
        .and_then(DateTime::<Utc>::from_timestamp_millis)
        .unwrap_or(DateTime::UNIX_EPOCH)
}

/// UTC 转 Unix epoch 毫秒；早于 epoch 的取值按 0 处理（本服务不产生此类时间）。
pub fn utc_to_ms(at: DateTime<Utc>) -> u64 {
    u64::try_from(at.timestamp_millis()).unwrap_or(0)
}

/// 账号行。
#[derive(Debug, Clone, FromRow)]
pub struct StoredAccount {
    pub account_id: String,
    pub phone: String,
    pub player_code: String,
    pub display_name: String,
    pub ordinal: i64,
}

/// 好友关系行。
#[derive(Debug, Clone, FromRow)]
pub struct StoredFriendship {
    pub player_id: String,
    pub friend_id: String,
}

/// 好友申请或发送记录行。
#[derive(Debug, Clone, FromRow)]
pub struct StoredRequest {
    pub from_account_id: String,
    pub to_account_id: String,
    pub sent_at: DateTime<Utc>,
}

/// 最近对局行。
#[derive(Debug, Clone, FromRow)]
pub struct StoredRecent {
    pub player_id: String,
    pub opponent_id: String,
    pub last_played_at: DateTime<Utc>,
}

/// 房间邀请行。
#[derive(Debug, Clone, FromRow)]
pub struct StoredInvite {
    pub invite_id: String,
    pub from_account_id: String,
    pub to_account_id: String,
    pub room_id: String,
    pub room_code: String,
    pub expires_at: DateTime<Utc>,
}

/// 待写入的房间邀请。
#[derive(Debug, Clone)]
pub struct NewInvite<'a> {
    pub invite_id: &'a str,
    pub from: &'a str,
    pub to: &'a str,
    pub room_id: &'a str,
    pub room_code: &'a str,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// 启动时一次性载入的社交数据。
#[derive(Debug, Default)]
pub struct StoredSocial {
    pub friendships: Vec<StoredFriendship>,
    pub requests: Vec<StoredRequest>,
    pub attempts: Vec<StoredRequest>,
    pub recent: Vec<StoredRecent>,
    pub invites: Vec<StoredInvite>,
}

/// PostgreSQL 连接池封装。
#[derive(Clone)]
pub struct Db {
    pool: PgPool,
}

impl std::fmt::Debug for Db {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Db").finish_non_exhaustive()
    }
}

impl Db {
    /// 连接数据库并执行基线迁移。
    ///
    /// # Errors
    ///
    /// DSN 非法、连接失败或迁移失败时返回错误文本。
    pub async fn connect(database_url: &str) -> Result<Self, String> {
        let pool = Self::pool(database_url, None).await?;
        let db = Self { pool };
        db.migrate().await?;
        Ok(db)
    }

    /// 连接数据库并为每个测试分配独立 schema，执行基线迁移。
    ///
    /// # Errors
    ///
    /// DSN 非法、建 schema 失败、连接失败或迁移失败时返回错误文本。
    pub async fn connect_in_schema(database_url: &str, schema: &str) -> Result<Self, String> {
        let bootstrap = PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(5))
            .connect(database_url)
            .await
            .map_err(|err| format!("连接数据库失败: {err}"))?;
        bootstrap
            .execute(
                format!(
                    "CREATE SCHEMA IF NOT EXISTS \"{}\"",
                    schema.replace('"', "")
                )
                .as_str(),
            )
            .await
            .map_err(|err| format!("创建 schema {schema} 失败: {err}"))?;
        bootstrap.close().await;

        let pool = Self::pool(database_url, Some(schema)).await?;
        let db = Self { pool };
        db.migrate().await?;
        Ok(db)
    }

    async fn pool(database_url: &str, schema: Option<&str>) -> Result<PgPool, String> {
        let mut options = PgConnectOptions::from_str(database_url)
            .map_err(|err| format!("DATABASE_URL 非法: {err}"))?;
        if let Some(schema) = schema {
            options = options.options([("search_path", schema.to_string())]);
        }
        PgPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(5))
            .connect_with(options)
            .await
            .map_err(|err| format!("连接数据库失败: {err}"))
    }

    /// 执行 `server/migrations` 下的基线迁移。
    ///
    /// # Errors
    ///
    /// 迁移失败时返回错误文本。
    pub async fn migrate(&self) -> Result<(), String> {
        sqlx::migrate!("../../migrations")
            .run(&self.pool)
            .await
            .map_err(|err| format!("执行数据库迁移失败: {err}"))
    }

    /// 执行 `SELECT 1`，供就绪探针使用。
    ///
    /// # Errors
    ///
    /// 查询失败时返回错误。
    pub async fn ping(&self) -> DbResult<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map(|_| ())
    }

    /// 载入全部账号。
    ///
    /// # Errors
    ///
    /// 查询失败时返回错误。
    pub async fn load_accounts(&self) -> DbResult<Vec<StoredAccount>> {
        sqlx::query_as::<_, StoredAccount>(
            "SELECT account_id, phone, player_code, display_name, ordinal
             FROM account ORDER BY ordinal",
        )
        .fetch_all(&self.pool)
        .await
    }

    /// 按手机号查账号。
    ///
    /// # Errors
    ///
    /// 查询失败时返回错误。
    pub async fn find_account(&self, phone: &str) -> DbResult<Option<StoredAccount>> {
        sqlx::query_as::<_, StoredAccount>(
            "SELECT account_id, phone, player_code, display_name, ordinal
             FROM account WHERE phone = $1",
        )
        .bind(phone)
        .fetch_optional(&self.pool)
        .await
    }

    /// 按手机号取账号，不存在则创建；默认展示名由持久化序号生成。
    ///
    /// # Errors
    ///
    /// 事务或写入失败时返回错误。同号并发创建视为已存在，回查后返回。
    pub async fn create_account(
        &self,
        phone: &str,
        display_name: Option<&str>,
        at: DateTime<Utc>,
    ) -> DbResult<StoredAccount> {
        let mut tx = self.pool.begin().await?;
        if let Some(account) = fetch_account(&mut *tx, phone).await? {
            tx.commit().await?;
            return Ok(account);
        }

        let account_id = format!("p_{}", Uuid::new_v4().simple());
        let inserted = sqlx::query_as::<_, StoredAccount>(
            "INSERT INTO account
                 (account_id, phone, player_code, display_name, created_at, updated_at)
             VALUES ($1, $2, nextval('player_code_seq')::text, $3, $4, $4)
             RETURNING account_id, phone, player_code, display_name, ordinal",
        )
        .bind(&account_id)
        .bind(phone)
        .bind(display_name.unwrap_or("玩家"))
        .bind(at)
        .fetch_one(&mut *tx)
        .await;

        let mut account = match inserted {
            Ok(account) => account,
            Err(error) => {
                tx.rollback().await?;
                if let Some(account) = self.find_account(phone).await? {
                    return Ok(account);
                }
                return Err(error);
            }
        };

        if display_name.is_none() {
            account.display_name = format!("玩家{}", account.ordinal);
            sqlx::query(
                "UPDATE account SET display_name = $2, updated_at = $3 WHERE account_id = $1",
            )
            .bind(&account.account_id)
            .bind(&account.display_name)
            .bind(at)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(account)
    }

    /// 载入未过期的社交数据，并顺手清理过期申请、发送记录与邀请。
    ///
    /// # Errors
    ///
    /// 清理或查询失败时返回错误。
    pub async fn load_social(
        &self,
        request_cutoff: DateTime<Utc>,
        attempt_cutoff: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> DbResult<StoredSocial> {
        sqlx::query("DELETE FROM friend_request WHERE sent_at <= $1")
            .bind(request_cutoff)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM friend_request_attempt WHERE sent_at <= $1")
            .bind(attempt_cutoff)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM room_invite WHERE expires_at <= $1")
            .bind(now)
            .execute(&self.pool)
            .await?;

        Ok(StoredSocial {
            friendships: sqlx::query_as::<_, StoredFriendship>(
                "SELECT player_id, friend_id FROM friendship",
            )
            .fetch_all(&self.pool)
            .await?,
            requests: sqlx::query_as::<_, StoredRequest>(
                "SELECT from_account_id, to_account_id, sent_at
                 FROM friend_request WHERE sent_at > $1",
            )
            .bind(request_cutoff)
            .fetch_all(&self.pool)
            .await?,
            attempts: sqlx::query_as::<_, StoredRequest>(
                "SELECT from_account_id, to_account_id, sent_at
                 FROM friend_request_attempt WHERE sent_at > $1",
            )
            .bind(attempt_cutoff)
            .fetch_all(&self.pool)
            .await?,
            recent: sqlx::query_as::<_, StoredRecent>(
                "SELECT player_id, opponent_id, last_played_at
                 FROM recent_opponent ORDER BY player_id, last_played_at DESC, opponent_id",
            )
            .fetch_all(&self.pool)
            .await?,
            invites: sqlx::query_as::<_, StoredInvite>(
                "SELECT invite_id, from_account_id, to_account_id, room_id, room_code, expires_at
                 FROM room_invite WHERE expires_at > $1",
            )
            .bind(now)
            .fetch_all(&self.pool)
            .await?,
        })
    }

    /// 记录一次好友申请：申请行与发送记录在同一事务内写入。
    ///
    /// # Errors
    ///
    /// 事务或写入失败时返回错误。
    pub async fn record_request(&self, from: &str, to: &str, at: DateTime<Utc>) -> DbResult<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO friend_request (from_account_id, to_account_id, sent_at)
             VALUES ($1, $2, $3)
             ON CONFLICT (from_account_id, to_account_id)
             DO UPDATE SET sent_at = EXCLUDED.sent_at",
        )
        .bind(from)
        .bind(to)
        .bind(at)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO friend_request_attempt (from_account_id, to_account_id, sent_at)
             VALUES ($1, $2, $3)",
        )
        .bind(from)
        .bind(to)
        .bind(at)
        .execute(&mut *tx)
        .await?;
        tx.commit().await
    }

    /// 处理好友申请：同意时删除申请、建立双向关系并清空双方发送记录；拒绝只删申请。
    ///
    /// # Errors
    ///
    /// 事务或写入失败时返回错误。
    pub async fn respond_request(
        &self,
        to: &str,
        from: &str,
        accept: bool,
        at: DateTime<Utc>,
    ) -> DbResult<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM friend_request WHERE from_account_id = $1 AND to_account_id = $2")
            .bind(from)
            .bind(to)
            .execute(&mut *tx)
            .await?;
        if accept {
            sqlx::query(
                "INSERT INTO friendship (player_id, friend_id, created_at)
                 VALUES ($1, $2, $3), ($2, $1, $3)
                 ON CONFLICT DO NOTHING",
            )
            .bind(to)
            .bind(from)
            .bind(at)
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                "DELETE FROM friend_request_attempt
                 WHERE (from_account_id = $1 AND to_account_id = $2)
                    OR (from_account_id = $2 AND to_account_id = $1)",
            )
            .bind(to)
            .bind(from)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await
    }

    /// 删除好友：双向关系、两个方向的申请与发送记录一并清除。
    ///
    /// # Errors
    ///
    /// 事务或写入失败时返回错误。
    pub async fn remove_friendship(&self, a: &str, b: &str) -> DbResult<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "DELETE FROM friendship
             WHERE (player_id = $1 AND friend_id = $2)
                OR (player_id = $2 AND friend_id = $1)",
        )
        .bind(a)
        .bind(b)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "DELETE FROM friend_request
             WHERE (from_account_id = $1 AND to_account_id = $2)
                OR (from_account_id = $2 AND to_account_id = $1)",
        )
        .bind(a)
        .bind(b)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "DELETE FROM friend_request_attempt
             WHERE (from_account_id = $1 AND to_account_id = $2)
                OR (from_account_id = $2 AND to_account_id = $1)",
        )
        .bind(a)
        .bind(b)
        .execute(&mut *tx)
        .await?;
        tx.commit().await
    }

    /// 记录一局参与者，并只保留每人的最近 20 名对手。
    ///
    /// # Errors
    ///
    /// 事务或写入失败时返回错误。
    pub async fn record_match(&self, participants: &[String], at: DateTime<Utc>) -> DbResult<()> {
        let mut tx = self.pool.begin().await?;
        for player in participants {
            for opponent in participants {
                if player == opponent {
                    continue;
                }
                sqlx::query(
                    "INSERT INTO recent_opponent (player_id, opponent_id, last_played_at)
                     VALUES ($1, $2, $3)
                     ON CONFLICT (player_id, opponent_id)
                     DO UPDATE SET last_played_at = EXCLUDED.last_played_at",
                )
                .bind(player)
                .bind(opponent)
                .bind(at)
                .execute(&mut *tx)
                .await?;
            }
        }
        for player in participants {
            sqlx::query(
                "DELETE FROM recent_opponent
                 WHERE player_id = $1 AND opponent_id NOT IN (
                     SELECT opponent_id FROM recent_opponent
                     WHERE player_id = $1
                     ORDER BY last_played_at DESC, opponent_id
                     LIMIT 20
                 )",
            )
            .bind(player)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await
    }

    /// 写入房间邀请，并清理已过期邀请。
    ///
    /// # Errors
    ///
    /// 事务或写入失败时返回错误。
    pub async fn save_invite(&self, invite: &NewInvite<'_>) -> DbResult<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM room_invite WHERE expires_at <= $1")
            .bind(invite.created_at)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO room_invite
                 (invite_id, from_account_id, to_account_id, room_id, room_code, expires_at, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(invite.invite_id)
        .bind(invite.from)
        .bind(invite.to)
        .bind(invite.room_id)
        .bind(invite.room_code)
        .bind(invite.expires_at)
        .bind(invite.created_at)
        .execute(&mut *tx)
        .await?;
        tx.commit().await
    }

    /// 删除一条邀请（响应或撤回）。
    ///
    /// # Errors
    ///
    /// 删除失败时返回错误。
    pub async fn delete_invite(&self, invite_id: &str) -> DbResult<()> {
        sqlx::query("DELETE FROM room_invite WHERE invite_id = $1")
            .bind(invite_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

async fn fetch_account<'e, E>(executor: E, phone: &str) -> DbResult<Option<StoredAccount>>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query_as::<_, StoredAccount>(
        "SELECT account_id, phone, player_code, display_name, ordinal
         FROM account WHERE phone = $1",
    )
    .bind(phone)
    .fetch_optional(executor)
    .await
}

/// 删除测试 schema；仅供测试隔离使用。
///
/// # Errors
///
/// 连接或删除失败时返回错误文本。
pub async fn drop_schema(database_url: &str, schema: &str) -> Result<(), String> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
        .map_err(|err| format!("连接数据库失败: {err}"))?;
    let result = sqlx::query(&format!(
        "DROP SCHEMA IF EXISTS \"{}\" CASCADE",
        schema.replace('"', "")
    ))
    .execute(&pool)
    .await
    .map_err(|err| format!("删除 schema {schema} 失败: {err}"));
    pool.close().await;
    result.map(|_| ())
}
