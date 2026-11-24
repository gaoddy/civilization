//! 天梯配置、积分存储、段位与排行榜。
//!
//! 积分算法本身在 `rule_engine::rating`（纯函数）；本模块负责持久化边界：
//! `after = max(0, before + delta)`、实际入账变化、历史最高积分、最近 10 局与
//! 宗师快照。未配置数据库时退化为纯内存模式，仅用于本地开发与测试。
//!
//! 日切时区：宗师快照按 UTC 00:00 切日（部署时区约定为 UTC）。段位区间不落库，
//! 每次按当前配置与积分重算；宗师额外要求位于最近一次快照的前
//! `master_rank_limit` 名。

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};

use crate::db::{Db, NewRankedResult, ms_to_utc, utc_to_ms};
use crate::queue;

/// 新账号初始积分。
pub const DEFAULT_RATING: i32 = 1000;

/// 个人最近战绩保留局数。
pub const RECENT_RESULTS: usize = 10;

/// 全服榜展示名次。
pub const LEADERBOARD_SIZE: usize = 100;

/// 默认宗师名次上限。
pub const DEFAULT_MASTER_RANK_LIMIT: u32 = 500;

/// 默认 K 系数。
pub const DEFAULT_K_FACTOR: f64 = 200.0;

/// 一天毫秒数。
const DAY_MS: u64 = 24 * 60 * 60 * 1000;

/// 段位 id；顺序即从低到高。
pub const TIERS: [&str; 8] = [
    "apprentice",
    "bronze",
    "silver",
    "gold",
    "platinum",
    "diamond",
    "master",
    "grandmaster",
];

/// 一段段位区间。
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TierBand {
    /// 段位 id。
    pub id: String,
    /// 该段位的最低积分。
    pub min_rating: i32,
}

/// 天梯运行参数；来自 `RANKED_CONFIG` JSON，未设置时用默认值。
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct RankedConfig {
    /// Elo K 系数。
    pub k: f64,
    /// 等待满该时长后不再排除最近对手。
    pub recent_opponent_relax_ms: u64,
    /// 宗师判定使用的前 N 名。
    pub master_rank_limit: u32,
    /// 段位区间，按 `min_rating` 升序。
    pub tiers: Vec<TierBand>,
}

impl Default for RankedConfig {
    fn default() -> Self {
        Self {
            k: DEFAULT_K_FACTOR,
            recent_opponent_relax_ms: queue::RECENT_OPPONENT_RELAX_MS,
            master_rank_limit: DEFAULT_MASTER_RANK_LIMIT,
            tiers: default_tiers(),
        }
    }
}

/// 产品文档 §4 的初始段位区间。
#[must_use]
pub fn default_tiers() -> Vec<TierBand> {
    vec![
        TierBand {
            id: TIERS[0].to_string(),
            min_rating: 0,
        },
        TierBand {
            id: TIERS[1].to_string(),
            min_rating: 900,
        },
        TierBand {
            id: TIERS[2].to_string(),
            min_rating: 1100,
        },
        TierBand {
            id: TIERS[3].to_string(),
            min_rating: 1250,
        },
        TierBand {
            id: TIERS[4].to_string(),
            min_rating: 1400,
        },
        TierBand {
            id: TIERS[5].to_string(),
            min_rating: 1550,
        },
        TierBand {
            id: TIERS[6].to_string(),
            min_rating: 1700,
        },
        TierBand {
            id: TIERS[7].to_string(),
            min_rating: 1900,
        },
    ]
}

impl RankedConfig {
    /// 解析并校验 `RANKED_CONFIG` JSON。
    ///
    /// # Errors
    ///
    /// JSON 非法、字段未知、K 非正、窗口为负、段位 id 缺失 / 重复 / 顺序错误时
    /// 返回错误文本；调用方应直接启动失败，不降级到默认值。
    pub fn from_json(raw: &str) -> Result<Self, String> {
        let config: Self =
            serde_json::from_str(raw).map_err(|err| format!("解析 RANKED_CONFIG 失败: {err}"))?;
        config.validate()?;
        Ok(config)
    }

    /// 校验参数自洽性。
    ///
    /// # Errors
    ///
    /// 同 [`RankedConfig::from_json`]。
    pub fn validate(&self) -> Result<(), String> {
        if !self.k.is_finite() || self.k <= 0.0 {
            return Err("RANKED_CONFIG: k 必须是正数".to_string());
        }
        if self.master_rank_limit == 0 {
            return Err("RANKED_CONFIG: masterRankLimit 必须大于 0".to_string());
        }
        if self.tiers.len() != TIERS.len() {
            return Err(format!(
                "RANKED_CONFIG: tiers 必须包含 {} 个段位",
                TIERS.len()
            ));
        }
        let mut seen = BTreeSet::new();
        let mut previous: Option<i32> = None;
        for (index, band) in self.tiers.iter().enumerate() {
            if !TIERS.contains(&band.id.as_str()) {
                return Err(format!("RANKED_CONFIG: 未知段位 id {}", band.id));
            }
            if !seen.insert(band.id.as_str()) {
                return Err(format!("RANKED_CONFIG: 段位 id 重复 {}", band.id));
            }
            if band.min_rating < 0 {
                return Err(format!("RANKED_CONFIG: 段位 {} 的积分不能为负", band.id));
            }
            if index == 0 {
                if band.min_rating != 0 {
                    return Err("RANKED_CONFIG: 最低段位的 minRating 必须为 0".to_string());
                }
            } else if previous.is_some_and(|value| band.min_rating <= value) {
                return Err("RANKED_CONFIG: 段位 minRating 必须严格递增".to_string());
            }
            previous = Some(band.min_rating);
        }
        Ok(())
    }

    /// 仅按积分算出的段位 id；宗师排名条件由 [`RatingStore::tier`] 叠加。
    #[must_use]
    pub fn rating_tier(&self, rating: i32) -> &str {
        let mut current = self.tiers[0].id.as_str();
        for band in &self.tiers {
            if rating >= band.min_rating {
                current = band.id.as_str();
            } else {
                break;
            }
        }
        current
    }

    /// 宗师最低积分。
    #[must_use]
    pub fn grandmaster_min_rating(&self) -> i32 {
        self.tiers
            .iter()
            .find(|band| band.id == TIERS[7])
            .map_or(i32::MAX, |band| band.min_rating)
    }

    /// 未进入宗师榜时，宗师积分对应的展示段位（大师）。
    #[must_use]
    pub fn master_tier(&self) -> &str {
        TIERS[6]
    }
}

/// 一条积分记录。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RatingEntry {
    pub rating: i32,
    pub highest_rating: i32,
    /// 最近一次积分变化时刻；同分排序按它先到先得。
    pub updated_at_ms: u64,
}

/// 对外展示的段位与积分。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankSummary {
    pub rating: i32,
    pub highest_rating: i32,
    pub tier: String,
    pub highest_tier: String,
}

/// 一条最近天梯战绩。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentMatch {
    pub placement: u8,
    pub rating_delta: i32,
    pub rating_after: i32,
    pub finished_at_ms: u64,
}

/// 一局天梯对局的身份与落库时间；与积分计算分开传参，避免结算入口参数过长。
#[derive(Debug, Clone, Copy)]
pub struct MatchRecord<'a> {
    pub match_id: &'a str,
    pub room_id: &'a str,
    pub seed: &'a str,
    pub finished_at_ms: u64,
}

/// 一局结算后的单人结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettleResult {
    pub player_id: String,
    pub placement: u8,
    pub rating_before: i32,
    pub rating_delta: i32,
    pub rating_after: i32,
    /// 结算后的历史最高积分；用于持久化与个人主页历史最高段位。
    pub highest_rating: i32,
    pub tier_before: String,
    pub tier_after: String,
}

/// 排行榜行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaderboardRow {
    pub player_id: String,
    pub rating: i32,
    /// `None` 表示还没有积分记录，排序时视为最后达到该分数。
    pub updated_at_ms: Option<u64>,
}

/// 积分存储：数据库写穿 + 进程内缓存；未配置数据库时只存在内存。
#[derive(Debug, Default)]
pub struct RatingStore {
    db: Option<Arc<Db>>,
    ratings: BTreeMap<String, RatingEntry>,
    /// 最近 10 局；内存模式下维护，数据库模式按需查询。
    recent: BTreeMap<String, VecDeque<RecentMatch>>,
    /// 最近一次宗师快照的 UTC 日序号。
    master_day: Option<i64>,
    /// 最近一次快照 `player_id → 名次`。
    master_ranks: BTreeMap<String, u32>,
    /// 历史上进入过快照的账号，用于历史最高段位。
    ever_top: BTreeSet<String>,
}

impl RatingStore {
    /// 纯内存模式。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 从数据库载入积分、宗师快照与历史宗师名单。
    ///
    /// # Errors
    ///
    /// 查询失败时返回错误文本。
    pub async fn load(db: Arc<Db>) -> Result<Self, String> {
        let ratings = db
            .load_ratings()
            .await
            .map_err(|err| format!("载入天梯积分失败: {err}"))?;
        let ever_top = db
            .load_master_account_ids()
            .await
            .map_err(|err| format!("载入宗师历史失败: {err}"))?
            .into_iter()
            .collect();
        let latest_day = db
            .latest_master_day()
            .await
            .map_err(|err| format!("载入宗师快照日期失败: {err}"))?;
        let (master_day, master_ranks) = match latest_day {
            Some(day) => {
                let rows = db
                    .load_master_snapshot(day)
                    .await
                    .map_err(|err| format!("载入宗师快照失败: {err}"))?;
                (
                    Some(day_to_index(day)),
                    rows.into_iter()
                        .map(|row| (row.account_id, u32::try_from(row.rank).unwrap_or(u32::MAX)))
                        .collect(),
                )
            }
            None => (None, BTreeMap::new()),
        };
        Ok(Self {
            db: Some(db),
            ratings: ratings
                .into_iter()
                .map(|row| {
                    (
                        row.account_id,
                        RatingEntry {
                            rating: row.rating,
                            highest_rating: row.highest_rating,
                            updated_at_ms: utc_to_ms(row.updated_at),
                        },
                    )
                })
                .collect(),
            recent: BTreeMap::new(),
            master_day,
            master_ranks,
            ever_top,
        })
    }

    /// 当前积分；没有记录时为初始 1000。
    #[must_use]
    pub fn rating_of(&self, player_id: &str) -> i32 {
        self.ratings
            .get(player_id)
            .map_or(DEFAULT_RATING, |entry| entry.rating)
    }

    /// 对外段位摘要。
    #[must_use]
    pub fn summary(&self, config: &RankedConfig, player_id: &str) -> RankSummary {
        let entry = self.ratings.get(player_id).copied();
        let rating = entry.map_or(DEFAULT_RATING, |value| value.rating);
        let highest_rating = entry.map_or(DEFAULT_RATING, |value| value.highest_rating);
        RankSummary {
            rating,
            highest_rating,
            tier: self.tier(config, player_id, rating),
            highest_tier: self.highest_tier(config, player_id, highest_rating),
        }
    }

    /// 当前段位：宗师需要同时满足积分与最近一次快照的名次。
    #[must_use]
    pub fn tier(&self, config: &RankedConfig, player_id: &str, rating: i32) -> String {
        let tier = config.rating_tier(rating);
        if tier == TIERS[7]
            && self
                .master_ranks
                .get(player_id)
                .is_none_or(|rank| *rank > config.master_rank_limit)
        {
            return config.master_tier().to_string();
        }
        tier.to_string()
    }

    /// 历史最高段位：按历史最高积分与当前配置计算；宗师还要求进过快照。
    #[must_use]
    pub fn highest_tier(
        &self,
        config: &RankedConfig,
        player_id: &str,
        highest_rating: i32,
    ) -> String {
        let tier = config.rating_tier(highest_rating);
        if tier == TIERS[7] && !self.ever_top.contains(player_id) {
            return config.master_tier().to_string();
        }
        tier.to_string()
    }

    /// 全服榜；数据库模式直接查库，内存模式由调用方提供账号全集。
    ///
    /// # Errors
    ///
    /// 数据库查询失败时返回错误文本。
    pub async fn leaderboard(
        &self,
        account_ids: &[String],
        limit: usize,
    ) -> Result<Vec<LeaderboardRow>, String> {
        if let Some(db) = &self.db {
            let rows = db
                .load_leaderboard(i64::try_from(limit).unwrap_or(i64::MAX))
                .await
                .map_err(|err| format!("查询排行榜失败: {err}"))?;
            return Ok(rows
                .into_iter()
                .map(|row| LeaderboardRow {
                    player_id: row.account_id,
                    rating: row.rating,
                    updated_at_ms: row.updated_at.map(utc_to_ms),
                })
                .collect());
        }

        let mut rows: Vec<LeaderboardRow> = account_ids
            .iter()
            .map(|player_id| {
                let entry = self.ratings.get(player_id);
                LeaderboardRow {
                    player_id: player_id.clone(),
                    rating: entry.map_or(DEFAULT_RATING, |value| value.rating),
                    updated_at_ms: entry.map(|value| value.updated_at_ms),
                }
            })
            .collect();
        sort_leaderboard(&mut rows);
        rows.truncate(limit);
        Ok(rows)
    }

    /// 确保最近一次宗师快照是今天；若没有则按当前排行榜写入。
    ///
    /// 数据库模式写入 `master_snapshot`；内存模式只更新进程内名单。启动后由后台
    /// 任务与首次查询共同触发，服务重启也会补齐当天快照。
    ///
    /// # Errors
    ///
    /// 查询或写入快照失败时返回错误文本。
    pub async fn ensure_master_snapshot(
        &mut self,
        config: &RankedConfig,
        now_ms: u64,
        account_ids: &[String],
    ) -> Result<(), String> {
        let today = day_index(now_ms);
        if self.master_day.is_some_and(|day| day >= today) {
            return Ok(());
        }
        let limit = usize::try_from(config.master_rank_limit).unwrap_or(usize::MAX);
        let rows = self.leaderboard(account_ids, limit).await?;
        let ranked: Vec<(String, u32)> = rows
            .iter()
            .enumerate()
            .map(|(index, row)| {
                (
                    row.player_id.clone(),
                    u32::try_from(index + 1).unwrap_or(u32::MAX),
                )
            })
            .collect();
        if let Some(db) = &self.db {
            db.save_master_snapshot(day_date(today), &ranked)
                .await
                .map_err(|err| format!("写入宗师快照失败: {err}"))?;
        }
        self.master_day = Some(today);
        self.master_ranks = ranked.iter().cloned().collect();
        self.ever_top.extend(ranked.into_iter().map(|(id, _)| id));
        Ok(())
    }

    /// 最近 10 局；数据库模式按需查询。
    ///
    /// # Errors
    ///
    /// 数据库查询失败时返回错误文本。
    pub async fn recent_for(&self, player_id: &str) -> Result<Vec<RecentMatch>, String> {
        if let Some(db) = &self.db {
            let rows = db
                .load_recent_results(player_id, RECENT_RESULTS as i64)
                .await
                .map_err(|err| format!("查询最近战绩失败: {err}"))?;
            return Ok(rows
                .into_iter()
                .map(|row| RecentMatch {
                    placement: u8::try_from(row.placement).unwrap_or(1),
                    rating_delta: row.rating_delta,
                    rating_after: row.rating_after,
                    finished_at_ms: utc_to_ms(row.finished_at),
                })
                .collect());
        }
        Ok(self
            .recent
            .get(player_id)
            .map(|matches| matches.iter().cloned().collect())
            .unwrap_or_default())
    }

    /// 结算一局天梯：写入对局、结果与账号积分，返回每人实际变化与段位前后。
    ///
    /// `placements` 是 8 个 `(player_id, 名次)`；`start_ratings` 是开局积分快照，
    /// 缺失时按 1000。数据库写入成功后才更新内存，失败时整局不计分。
    ///
    /// # Errors
    ///
    /// 参赛人数不是 8、名次非法或数据库写入失败时返回错误文本。
    pub async fn settle_match(
        &mut self,
        config: &RankedConfig,
        record: &MatchRecord<'_>,
        placements: &[(String, u8)],
        start_ratings: &BTreeMap<String, i32>,
    ) -> Result<Vec<SettleResult>, String> {
        let now_ms = record.finished_at_ms;
        if placements.len() != rule_engine::rating::PLAYERS_PER_MATCH {
            return Err(format!("天梯结算需要 8 名玩家，实际 {}", placements.len()));
        }
        let mut ratings = [0_i32; rule_engine::rating::PLAYERS_PER_MATCH];
        let mut places = [0_u8; rule_engine::rating::PLAYERS_PER_MATCH];
        for (index, (player_id, placement)) in placements.iter().enumerate() {
            if !(1..=8).contains(placement) {
                return Err(format!("天梯结算名次非法: {placement}"));
            }
            ratings[index] = start_ratings
                .get(player_id)
                .copied()
                .unwrap_or(DEFAULT_RATING)
                .max(0);
            places[index] = *placement;
        }
        let deltas = rule_engine::rating::rating_deltas(config.k, &ratings, &places);

        let mut results = Vec::with_capacity(placements.len());
        let mut new_entries = Vec::with_capacity(placements.len());
        for (index, (player_id, placement)) in placements.iter().enumerate() {
            let before = ratings[index];
            let (after, actual_delta) =
                rule_engine::rating::apply_rating_floor(before, deltas[index]);
            let previous = self.ratings.get(player_id).copied();
            let highest_rating = previous
                .map_or(DEFAULT_RATING, |entry| entry.highest_rating)
                .max(after);
            let updated_at_ms = match previous {
                Some(entry) if entry.rating == after => entry.updated_at_ms,
                _ => now_ms,
            };
            let tier_before = self.tier(config, player_id, before);
            let tier_after = self.tier(config, player_id, after);
            results.push(SettleResult {
                player_id: player_id.clone(),
                placement: *placement,
                rating_before: before,
                rating_delta: actual_delta,
                rating_after: after,
                highest_rating,
                tier_before,
                tier_after,
            });
            new_entries.push((
                player_id.clone(),
                RatingEntry {
                    rating: after,
                    highest_rating,
                    updated_at_ms,
                },
            ));
        }

        if let Some(db) = &self.db {
            let rows: Vec<NewRankedResult<'_>> = results
                .iter()
                .map(|result| NewRankedResult {
                    account_id: &result.player_id,
                    placement: i16::from(result.placement),
                    rating_before: result.rating_before,
                    rating_delta: result.rating_delta,
                    rating_after: result.rating_after,
                    highest_rating: result.highest_rating,
                })
                .collect();
            db.settle_ranked_match(
                record.match_id,
                record.room_id,
                record.seed,
                ms_to_utc(now_ms),
                &rows,
            )
            .await
            .map_err(|err| format!("写入天梯结算失败: {err}"))?;
        }

        for (player_id, entry) in new_entries {
            self.ratings.insert(player_id, entry);
        }
        for result in &results {
            let matches = self.recent.entry(result.player_id.clone()).or_default();
            matches.push_front(RecentMatch {
                placement: result.placement,
                rating_delta: result.rating_delta,
                rating_after: result.rating_after,
                finished_at_ms: now_ms,
            });
            matches.truncate(RECENT_RESULTS);
        }
        Ok(results)
    }
}

/// 最近 10 局前 4 名比例；无记录时为 0。
#[must_use]
pub fn top4_ratio(recent: &[RecentMatch]) -> f64 {
    if recent.is_empty() {
        return 0.0;
    }
    let top4 = recent.iter().filter(|entry| entry.placement <= 4).count();
    top4 as f64 / recent.len() as f64
}

/// 最近 10 局是否对请求者可见：本人或好友。
#[must_use]
pub fn recent_visible(viewer: &str, target: &str, is_friend: bool) -> bool {
    viewer == target || is_friend
}

/// 天梯结算只对自动匹配局生效。
#[must_use]
pub fn should_settle(ranked: bool) -> bool {
    ranked
}

fn sort_leaderboard(rows: &mut [LeaderboardRow]) {
    rows.sort_by(|left, right| {
        right
            .rating
            .cmp(&left.rating)
            // 没有记录的值排在同分已有记录之后；`None` 用 `MAX` 表示。
            .then_with(|| {
                left.updated_at_ms
                    .unwrap_or(u64::MAX)
                    .cmp(&right.updated_at_ms.unwrap_or(u64::MAX))
            })
            .then_with(|| left.player_id.cmp(&right.player_id))
    });
}

/// Unix 毫秒 → UTC 日序号。
#[must_use]
pub fn day_index(now_ms: u64) -> i64 {
    i64::try_from(now_ms / DAY_MS).unwrap_or(i64::MAX)
}

/// UTC 日序号 → 日期。
#[must_use]
pub fn day_date(day: i64) -> NaiveDate {
    DateTime::<Utc>::from_timestamp(day.saturating_mul(86_400), 0).map_or_else(
        || NaiveDate::from_ymd_opt(1970, 1, 1).unwrap_or_default(),
        |time| time.date_naive(),
    )
}

/// 日期 → UTC 日序号。
#[must_use]
pub fn day_to_index(day: NaiveDate) -> i64 {
    day.signed_duration_since(NaiveDate::from_ymd_opt(1970, 1, 1).unwrap_or_default())
        .num_days()
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_RATING, RankedConfig, RatingStore, RecentMatch, TIERS, recent_visible,
        should_settle, top4_ratio,
    };
    use std::collections::BTreeMap;

    fn players(count: usize) -> Vec<String> {
        (0..count).map(|index| format!("p_{index:03}")).collect()
    }

    fn placements(ids: &[String]) -> Vec<(String, u8)> {
        ids.iter()
            .enumerate()
            .map(|(index, id)| (id.clone(), u8::try_from(index + 1).unwrap()))
            .collect()
    }

    async fn settle(
        store: &mut RatingStore,
        config: &RankedConfig,
        now_ms: u64,
        ids: &[String],
        before: i32,
    ) {
        let start: BTreeMap<String, i32> = ids.iter().map(|id| (id.clone(), before)).collect();
        let record = super::MatchRecord {
            match_id: &format!("m_{now_ms}"),
            room_id: "room",
            seed: "seed",
            finished_at_ms: now_ms,
        };
        store
            .settle_match(config, &record, &placements(ids), &start)
            .await
            .expect("内存结算应成功");
    }

    #[test]
    fn default_config_matches_product_table() {
        let config = RankedConfig::default();
        assert_eq!(config.rating_tier(0), "apprentice");
        assert_eq!(config.rating_tier(899), "apprentice");
        assert_eq!(config.rating_tier(900), "bronze");
        assert_eq!(config.rating_tier(1099), "bronze");
        assert_eq!(config.rating_tier(1100), "silver");
        assert_eq!(config.rating_tier(1900), "grandmaster");
    }

    #[test]
    fn invalid_config_is_rejected() {
        assert!(RankedConfig::from_json("{ not json }").is_err());
        let invalid = RankedConfig {
            k: 0.0,
            ..RankedConfig::default()
        };
        assert!(invalid.validate().is_err());
        let mut reversed = RankedConfig::default();
        reversed.tiers.reverse();
        assert!(reversed.validate().is_err());
    }

    #[tokio::test]
    async fn settlement_applies_floor_and_keeps_actual_delta() {
        let config = RankedConfig::default();
        let mut store = RatingStore::new();
        let ids = players(8);
        let start: BTreeMap<String, i32> = ids.iter().map(|id| (id.clone(), 5)).collect();
        let placements = placements(&ids);
        let record = super::MatchRecord {
            match_id: "m1",
            room_id: "r1",
            seed: "s1",
            finished_at_ms: 1,
        };
        let results = store
            .settle_match(&config, &record, &placements, &start)
            .await
            .expect("结算应成功");
        // 第 5 名本应 −14，但 5 分被下限截断到 0，实际 −5。
        let fifth = &results[4];
        assert_eq!(fifth.rating_after, 0);
        assert_eq!(fifth.rating_delta, -5);
        assert_eq!(store.rating_of(&ids[4]), 0);
    }

    #[tokio::test]
    async fn leaderboard_orders_by_rating_then_arrival_then_id() {
        // K 取极小值让取整后变化为 0：8 人保持 1000，用于验证同分排序。
        let config = RankedConfig {
            k: 0.0001,
            ..RankedConfig::default()
        };
        let mut store = RatingStore::new();
        let first = players(8);
        settle(&mut store, &config, 1_000, &first, 1000).await;
        // 第二批玩家在更晚时刻到达同一分数。
        let second = (0..8)
            .map(|index| format!("q_{index:03}"))
            .collect::<Vec<_>>();
        settle(&mut store, &config, 2_000, &second, 1000).await;

        let all = [first.clone(), second.clone()].concat();
        let rows = store
            .leaderboard(&all, 100)
            .await
            .expect("内存排行榜应可用");
        let first_last = rows
            .iter()
            .position(|row| row.player_id == first[7])
            .expect("先到玩家应在榜上");
        let second_first = rows
            .iter()
            .position(|row| row.player_id == second[0])
            .expect("后到玩家应在榜上");
        assert!(first_last < second_first, "同分应按先达到该分数的时间排序");
        // 同批同刻按 player_id 升序。
        assert_eq!(rows[0].player_id, first[0]);
        // 没有积分记录的账号排在同分已有记录之后。
        let rows = store
            .leaderboard(&[vec!["p_new".to_string()], all].concat(), 100)
            .await
            .expect("内存排行榜应可用");
        assert_eq!(rows.last().map(|row| row.player_id.as_str()), Some("p_new"));
    }

    #[tokio::test]
    async fn master_requires_rating_and_snapshot_rank() {
        let config = RankedConfig {
            master_rank_limit: 2,
            ..RankedConfig::default()
        };
        let mut store = RatingStore::new();
        let ids = players(8);
        // 8 人同分 1900，第 1、2 名进入宗师榜。
        let start: BTreeMap<String, i32> = ids.iter().map(|id| (id.clone(), 1900)).collect();
        let record = super::MatchRecord {
            match_id: "m1",
            room_id: "r1",
            seed: "s1",
            finished_at_ms: 1,
        };
        store
            .settle_match(&config, &record, &placements(&ids), &start)
            .await
            .expect("结算应成功");
        store
            .ensure_master_snapshot(&config, 1, &ids)
            .await
            .expect("快照应成功");
        // 第 1、2 名达到 2000/1971，仍在宗师门槛。
        let top = store.summary(&config, &ids[0]);
        assert_eq!(top.tier, TIERS[7]);
        assert_eq!(top.highest_tier, TIERS[7]);
        // 第 3 名积分仍高于 1900，但排名第 3 > 上限 2。
        let third = store.summary(&config, &ids[2]);
        assert!(third.rating >= config.grandmaster_min_rating());
        assert_eq!(third.tier, TIERS[6]);
        // 500 名之外的 1900 分账号显示大师。
        let outside = store.summary(&config, "p_outside");
        assert_eq!(outside.rating, DEFAULT_RATING);
    }

    #[tokio::test]
    async fn recent_results_keep_only_ten() {
        let config = RankedConfig::default();
        let mut store = RatingStore::new();
        let ids = players(8);
        for round in 0..12_u64 {
            settle(&mut store, &config, round + 1, &ids, 1000).await;
        }
        let recent = store.recent_for(&ids[0]).await.expect("内存战绩应可用");
        assert_eq!(recent.len(), 10);
    }

    #[test]
    fn visibility_and_top4_ratio() {
        assert!(recent_visible("a", "a", false));
        assert!(recent_visible("a", "b", true));
        assert!(!recent_visible("a", "b", false));
        let recent = vec![
            RecentMatch {
                placement: 1,
                rating_delta: 0,
                rating_after: 0,
                finished_at_ms: 0,
            },
            RecentMatch {
                placement: 5,
                rating_delta: 0,
                rating_after: 0,
                finished_at_ms: 0,
            },
        ];
        assert!((top4_ratio(&recent) - 0.5).abs() < f64::EPSILON);
        assert_eq!(top4_ratio(&[]), 0.0);
    }

    #[test]
    fn only_ranked_matches_settle() {
        assert!(should_settle(true));
        assert!(!should_settle(false));
    }
}
