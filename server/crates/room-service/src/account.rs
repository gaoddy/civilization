//! 账号身份：手机号 / SteamID → 稳定 `playerId` / 玩家编号 / 展示名。
//!
//! 数据库为唯一真相源；内存缓存保存全部已登录账号，启动时从库载入。同一手机号或
//! 同一 SteamID 重新登录恢复同一账号与玩家编号，展示名不在重新登录时被覆盖。
//! 一个账号可以有手机号与 SteamID 两个身份，至少有一个；绑定只允许 Steam 账号补绑
//! 手机号。

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use protocol::AccountView;
use protocol::server::ErrorMessage;
use uuid::Uuid;

use crate::auth::{PLAYER_CODE_END, PLAYER_CODE_START};
use crate::db::{BindPhoneOutcome, Db, StoredAccount, ms_to_utc};
use crate::error::error_message;

/// 展示名最多保留的字符数；超出的 Steam 个人资料名截断。
pub const MAX_DISPLAY_NAME_CHARS: usize = 20;

/// 账号对外身份。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub player_id: String,
    pub player_code: String,
    pub display_name: String,
    /// 已绑定的手机号；未绑定为 `None`。
    pub phone: Option<String>,
    /// 已绑定的 SteamID；非 Steam 账号为 `None`。
    pub steam_id: Option<String>,
}

/// 账号存储：数据库写穿 + 进程内缓存。
#[derive(Debug, Default)]
pub struct AccountStore {
    db: Option<Arc<Db>>,
    by_phone: BTreeMap<String, Account>,
    by_steam: BTreeMap<String, Account>,
    by_id: BTreeMap<String, Account>,
    /// 已分配的玩家编号；内存模式下保证不重复，数据库侧由唯一约束保证。
    codes: BTreeSet<String>,
    /// 已创建的账号数，只用于生成 `玩家N`；内存模式下不因清理而回退。
    ordinal: u64,
    next_code: u64,
    /// 账号级成就解锁：每个账号第一次达成的成就 id。
    achievements: BTreeMap<String, BTreeSet<String>>,
}

impl AccountStore {
    /// 内存模式，用于未配置数据库的开发与测试。
    pub fn new() -> Self {
        Self {
            db: None,
            by_phone: BTreeMap::new(),
            by_steam: BTreeMap::new(),
            by_id: BTreeMap::new(),
            codes: BTreeSet::new(),
            ordinal: 0,
            next_code: PLAYER_CODE_START,
            achievements: BTreeMap::new(),
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
            db: Some(db.clone()),
            ..Self::default()
        };
        for row in rows {
            store.cache_from_row(row);
        }
        let achievements = db
            .load_achievements()
            .await
            .map_err(|err| format!("载入账号成就失败: {err}"))?;
        for row in achievements {
            store
                .achievements
                .entry(row.account_id)
                .or_default()
                .insert(row.achievement_id);
        }
        Ok(store)
    }

    /// 手机号登录：按手机号取已有账号，不存在则创建。
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
            return Ok(self.create_in_memory(Some(phone), None, display_name));
        };
        let row = db
            .create_account(phone, display_name, ms_to_utc(now_ms))
            .await
            .map_err(|err| storage_error(&err))?;
        Ok(self.cache_from_row(row))
    }

    /// Steam 登录：按 SteamID 取已有账号，不存在则创建。
    ///
    /// `display_name` 来自 Steam 个人资料名，去空白并截断为 20 字符；为空时按持久化
    /// 序号生成 `玩家N`。已有账号保留原展示名。
    ///
    /// # Errors
    ///
    /// 数据库写入失败时返回 `INTERNAL_ERROR`。
    pub async fn login_steam(
        &mut self,
        steam_id: &str,
        display_name: Option<&str>,
        now_ms: u64,
    ) -> Result<Account, ErrorMessage> {
        if let Some(account) = self.by_steam.get(steam_id) {
            return Ok(account.clone());
        }
        let display_name = normalize_display_name(display_name);
        let Some(db) = self.db.clone() else {
            return Ok(self.create_in_memory(None, Some(steam_id), display_name.as_deref()));
        };
        let row = db
            .create_steam_account(steam_id, display_name.as_deref(), ms_to_utc(now_ms))
            .await
            .map_err(|err| storage_error(&err))?;
        Ok(self.cache_from_row(row))
    }

    /// 账号页面视图：绑定状态与掩码手机号，不暴露原始身份。
    /// 天梯字段由调用方按积分存储补全。
    pub fn view(&self, player_id: &str) -> Option<AccountView> {
        self.by_id.get(player_id).map(account_view)
    }

    /// 全部账号 id，按持久化序号（内存模式为创建顺序的 `BTreeMap` 顺序）。
    pub fn player_ids(&self) -> Vec<String> {
        self.by_id.keys().cloned().collect()
    }

    /// 账号展示名；账号不存在返回 `None`。
    pub fn display_name(&self, player_id: &str) -> Option<&str> {
        self.by_id
            .get(player_id)
            .map(|account| account.display_name.as_str())
    }

    /// 账号已解锁的成就 id，按 id 升序。
    pub fn unlocked_achievements(&self, player_id: &str) -> Vec<String> {
        self.achievements
            .get(player_id)
            .map(|ids| ids.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// 记录账号首次解锁一个成就；本账号已解锁时返回 `false`。
    ///
    /// 内存模式下只写缓存；数据库模式下先落库再更新缓存。落库失败时不更新缓存，
    /// 由调用方决定是否继续下发。
    ///
    /// # Errors
    ///
    /// 账号不存在或数据库写入失败时返回 `INTERNAL_ERROR`。
    pub async fn unlock_achievement(
        &mut self,
        player_id: &str,
        achievement_id: &str,
        now_ms: u64,
    ) -> Result<bool, ErrorMessage> {
        if !self.by_id.contains_key(player_id) {
            return Err(error_message("INTERNAL_ERROR", "账号不存在", false));
        }
        if self
            .achievements
            .get(player_id)
            .is_some_and(|ids| ids.contains(achievement_id))
        {
            return Ok(false);
        }
        let inserted = match self.db.clone() {
            Some(db) => db
                .insert_achievement(player_id, achievement_id, ms_to_utc(now_ms))
                .await
                .map_err(|err| storage_error(&err))?,
            None => true,
        };
        self.achievements
            .entry(player_id.to_string())
            .or_default()
            .insert(achievement_id.to_string());
        Ok(inserted)
    }

    /// 给 Steam 账号绑定手机号。
    ///
    /// 已绑定同一手机号幂等返回；已绑定其他手机号或手机号被其他账号占用返回
    /// `PHONE_ALREADY_BOUND`；非 Steam 账号不允许绑定。
    ///
    /// # Errors
    ///
    /// 账号不存在或数据库写入失败时为 `INTERNAL_ERROR`；其余见错误码说明。
    pub async fn bind_phone(
        &mut self,
        player_id: &str,
        phone: &str,
        now_ms: u64,
    ) -> Result<AccountView, ErrorMessage> {
        let Some(account) = self.by_id.get(player_id).cloned() else {
            return Err(error_message("INTERNAL_ERROR", "账号不存在", false));
        };
        if account.steam_id.is_none() {
            return Err(error_message(
                "INVALID_STAGE",
                "只有 Steam 账号可以绑定手机号",
                false,
            ));
        }
        if account.phone.as_deref() == Some(phone) {
            return Ok(account_view(&account));
        }
        if account.phone.is_some() || self.by_phone.contains_key(phone) {
            return Err(phone_already_bound_error());
        }

        let Some(db) = self.db.clone() else {
            let mut updated = account;
            updated.phone = Some(phone.to_string());
            self.cache_insert(updated.clone());
            return Ok(account_view(&updated));
        };

        let outcome = db
            .bind_phone(player_id, phone, ms_to_utc(now_ms))
            .await
            .map_err(|err| storage_error(&err))?;
        match outcome {
            BindPhoneOutcome::Bound(row) => Ok(account_view(&self.cache_from_row(row))),
            BindPhoneOutcome::AlreadyBound => Err(phone_already_bound_error()),
            BindPhoneOutcome::NotFound => Err(error_message("INTERNAL_ERROR", "账号不存在", false)),
        }
    }

    /// 内存模式下的账号创建，编号与序号语义与持久化模式一致。
    fn create_in_memory(
        &mut self,
        phone: Option<&str>,
        steam_id: Option<&str>,
        display_name: Option<&str>,
    ) -> Account {
        self.ordinal = self.ordinal.saturating_add(1);
        let display_name = match display_name {
            Some(name) => name.to_string(),
            None => format!("玩家{}", self.ordinal),
        };
        let account = Account {
            player_id: format!("p_{}", Uuid::new_v4().simple()),
            player_code: self.allocate_code(),
            display_name,
            phone: phone.map(str::to_string),
            steam_id: steam_id.map(str::to_string),
        };
        self.cache_insert(account.clone());
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
            phone: row.phone,
            steam_id: row.steam_id,
        };
        self.cache_insert(account.clone());
        account
    }

    fn cache_insert(&mut self, account: Account) {
        self.codes.insert(account.player_code.clone());
        if let Some(phone) = &account.phone {
            self.by_phone.insert(phone.clone(), account.clone());
        }
        if let Some(steam_id) = &account.steam_id {
            self.by_steam.insert(steam_id.clone(), account.clone());
        }
        self.by_id.insert(account.player_id.clone(), account);
    }
}

/// 去空白并截断展示名；结果为空时返回 `None`，交由默认命名。
fn normalize_display_name(name: Option<&str>) -> Option<String> {
    let name = name?.trim();
    if name.is_empty() {
        return None;
    }
    Some(name.chars().take(MAX_DISPLAY_NAME_CHARS).collect())
}

fn account_view(account: &Account) -> AccountView {
    AccountView {
        player_id: account.player_id.clone(),
        player_code: account.player_code.clone(),
        display_name: account.display_name.clone(),
        steam_bound: account.steam_id.is_some(),
        phone_bound: account.phone.is_some(),
        masked_phone: account.phone.as_deref().map(mask_phone),
        // 未结算账号按初始分与青铜展示；下发前由房间服务按积分存储补全。
        rating: u64::try_from(crate::rating::DEFAULT_RATING).unwrap_or(0),
        tier: crate::rating::TIERS[1].to_string(),
        highest_tier: crate::rating::TIERS[1].to_string(),
    }
}

/// 保留前 3 位与后 4 位，其余用 `*`；过短的号码整体掩码。
fn mask_phone(phone: &str) -> String {
    let chars: Vec<char> = phone.chars().collect();
    if chars.len() <= 7 {
        return "*".repeat(chars.len());
    }
    let head: String = chars[..3].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}{}{tail}", "*".repeat(chars.len() - 7))
}

fn storage_error(error: &sqlx::Error) -> ErrorMessage {
    error_message("INTERNAL_ERROR", &format!("账号存储失败: {error}"), true)
}

fn phone_already_bound_error() -> ErrorMessage {
    error_message(
        "PHONE_ALREADY_BOUND",
        "手机号已绑定其他账号或当前账号已绑定其他手机号",
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::{AccountStore, mask_phone, normalize_display_name};

    #[tokio::test]
    async fn achievement_unlock_is_account_level_and_idempotent() {
        let mut store = AccountStore::new();
        let account = store
            .login("13800000001", None, 0)
            .await
            .expect("内存账号应创建成功");
        assert!(store.unlocked_achievements(&account.player_id).is_empty());
        assert!(
            store
                .unlock_achievement(&account.player_id, "genetics", 1)
                .await
                .expect("首次解锁应成功")
        );
        assert!(
            !store
                .unlock_achievement(&account.player_id, "genetics", 2)
                .await
                .expect("重复解锁应返回 false")
        );
        assert_eq!(
            store.unlocked_achievements(&account.player_id),
            vec!["genetics".to_string()]
        );
        assert!(
            store
                .unlock_achievement("p_missing", "genetics", 3)
                .await
                .is_err()
        );
    }

    #[test]
    fn mask_phone_keeps_prefix_and_suffix() {
        assert_eq!(mask_phone("13800000000"), "138****0000");
        assert_eq!(mask_phone("1234567"), "*******");
    }

    #[test]
    fn display_name_is_trimmed_and_truncated() {
        assert_eq!(
            normalize_display_name(Some("  Ada  ")).as_deref(),
            Some("Ada")
        );
        assert_eq!(normalize_display_name(Some("   ")), None);
        let long = "一".repeat(30);
        assert_eq!(
            normalize_display_name(Some(&long))
                .expect("截断后应保留内容")
                .chars()
                .count(),
            20
        );
    }
}
