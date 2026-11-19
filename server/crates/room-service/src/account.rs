//! 账号身份：手机号 → 稳定 `playerId` / 玩家编号 / 展示名。
//!
//! 数据库为唯一真相源；内存缓存保存全部已登录账号，启动时从库载入。同一手机号
//! 重新登录恢复同一账号与玩家编号，展示名不在重新登录时被覆盖。

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use protocol::server::ErrorMessage;
use uuid::Uuid;

use crate::auth::{PLAYER_CODE_END, PLAYER_CODE_START};
use crate::db::{Db, StoredAccount, ms_to_utc};
use crate::error::error_message;

/// 账号对外身份。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub player_id: String,
    pub player_code: String,
    pub display_name: String,
}

/// 账号存储：数据库写穿 + 进程内缓存。
#[derive(Debug, Default)]
pub struct AccountStore {
    db: Option<Arc<Db>>,
    by_phone: BTreeMap<String, Account>,
    /// 已分配的玩家编号；内存模式下保证不重复，数据库侧由唯一约束保证。
    codes: BTreeSet<String>,
    /// 已创建的账号数，只用于生成 `玩家N`；内存模式下不因清理而回退。
    ordinal: u64,
    next_code: u64,
}

impl AccountStore {
    /// 内存模式，用于未配置数据库的开发与测试。
    pub fn new() -> Self {
        Self {
            db: None,
            by_phone: BTreeMap::new(),
            codes: BTreeSet::new(),
            ordinal: 0,
            next_code: PLAYER_CODE_START,
        }
    }

    /// 从数据库载入全部账号到缓存。
    ///
    /// # Errors
    ///
    /// 查询失败时返回错误文本。
    pub async fn load(db: Arc<Db>) -> Result<Self, String> {
        let rows = db
            .load_accounts()
            .await
            .map_err(|err| format!("载入账号失败: {err}"))?;
        let mut store = Self {
            db: Some(db),
            ..Self::default()
        };
        for row in rows {
            store.cache_insert(
                row.phone,
                Account {
                    player_id: row.account_id,
                    player_code: row.player_code,
                    display_name: row.display_name,
                },
            );
        }
        Ok(store)
    }

    /// 登录：按手机号取已有账号，不存在则创建。
    ///
    /// `display_name` 只在创建时使用；已有账号保留原展示名。
    ///
    /// # Errors
    ///
    /// 数据库写入失败时返回 `INTERNAL_ERROR`。
    pub async fn login(
        &mut self,
        phone: &str,
        display_name: Option<&str>,
        now_ms: u64,
    ) -> Result<Account, ErrorMessage> {
        if let Some(account) = self.by_phone.get(phone) {
            return Ok(account.clone());
        }
        let Some(db) = self.db.clone() else {
            return Ok(self.create_in_memory(phone, display_name));
        };
        let row = db
            .create_account(phone, display_name, ms_to_utc(now_ms))
            .await
            .map_err(|err| storage_error(&err))?;
        Ok(self.cache_from_row(row))
    }

    /// 内存模式下的账号创建，编号与序号语义与持久化模式一致。
    fn create_in_memory(&mut self, phone: &str, display_name: Option<&str>) -> Account {
        self.ordinal = self.ordinal.saturating_add(1);
        let display_name = match display_name {
            Some(name) => name.to_string(),
            None => format!("玩家{}", self.ordinal),
        };
        let account = Account {
            player_id: format!("p_{}", Uuid::new_v4().simple()),
            player_code: self.allocate_code(),
            display_name,
        };
        self.cache_insert(phone.to_string(), account.clone());
        account
    }

    fn allocate_code(&mut self) -> String {
        for _ in 0..(PLAYER_CODE_END - PLAYER_CODE_START) {
            let code = format!("{:06}", self.next_code);
            self.next_code = if self.next_code + 1 >= PLAYER_CODE_END {
                PLAYER_CODE_START
            } else {
                self.next_code + 1
            };
            if self.codes.insert(code.clone()) {
                return code;
            }
        }
        // 90 万个编号耗尽属于不可恢复状态；保留最后一个编号而不是崩溃。
        format!("{:06}", self.next_code)
    }

    fn cache_from_row(&mut self, row: StoredAccount) -> Account {
        let account = Account {
            player_id: row.account_id,
            player_code: row.player_code,
            display_name: row.display_name,
        };
        self.cache_insert(row.phone, account.clone());
        account
    }

    fn cache_insert(&mut self, phone: String, account: Account) {
        self.codes.insert(account.player_code.clone());
        self.by_phone.insert(phone, account);
    }
}

fn storage_error(error: &sqlx::Error) -> ErrorMessage {
    error_message("INTERNAL_ERROR", &format!("账号存储失败: {error}"), true)
}
