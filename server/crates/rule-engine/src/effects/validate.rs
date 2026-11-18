use crate::data::Playstyle;

use super::{Duration, Effect, Filter, Op, Params, Pick, Selector, TimesIf, Trigger, Zone};

/// 光环触发允许的原语。
pub(crate) const AURA_OPS: &[Op] = &[Op::ModifyStats, Op::GrantKeyword];
/// 选择器筛选允许的键；`condition` 与 `when` 复用同一套键。
pub(crate) const FILTER_KEYS: &[&str] = &[
    "taxon",
    "playstyle",
    "play",
    "prey",
    "host",
    "subtype",
    "id",
    "type",
    "scholar",
    "keyword",
    "other",
];
/// 卡牌类型取值。
pub(crate) const CARD_TYPES: &[&str] = &["minion", "effect"];
/// 关键词基名。
pub(crate) const KEYWORD_BASES: &[&str] = &[
    "battlecry",
    "deathrattle",
    "reborn",
    "taunt",
    "divine_shield",
    "windfury",
    "contagion",
    "optimal_attack",
    "sow",
    "evolve",
    "breed",
    "assemble",
    "poison",
    "armor",
    "stealth",
];
/// `when` 允许的事件类触发，与 `scripts/audit_effects.py` 的 `WHEN_TRIGGERS` 一致。
pub(crate) const WHEN_TRIGGERS: &[Trigger] = &[
    Trigger::OnEnemyDeath,
    Trigger::OnFriendlyDeath,
    Trigger::OnAllySummon,
    Trigger::OnAllyGrowth,
    Trigger::OnAllyEvolve,
    Trigger::OnDealDamage,
    Trigger::OnCombatEnd,
    Trigger::OnUse,
    Trigger::OnDeath,
    Trigger::OnKill,
    Trigger::OnAllyKill,
];
/// `Params` 中所有可出现的 DSL 参数名。
const PARAM_KEYS: &[&str] = &[
    "times",
    "count",
    "amount",
    "attack",
    "health",
    "perAttack",
    "perHealth",
    "maxCount",
    "threshold",
    "value",
    "extra",
    "duration",
    "canKill",
    "healthOnly",
    "statsOnly",
    "timesIf",
    "condition",
    "when",
    "keyword",
    "token",
    "card",
    "to",
    "from",
    "type",
    "pool",
    "choose",
    "free",
    "era",
    "level",
    "state",
    "basis",
];

pub(crate) type OpSpec = (
    &'static [&'static str],
    &'static [&'static str],
    &'static [&'static [&'static str]],
);

pub(crate) fn op_spec(op: Op) -> OpSpec {
    match op {
        Op::Growth => (
            &["times"],
            &["healthOnly", "timesIf", "condition", "when"],
            &[&["times"]],
        ),
        Op::ModifyStats => (
            &[],
            &[
                "attack",
                "health",
                "perAttack",
                "perHealth",
                "maxCount",
                "duration",
                "canKill",
                "condition",
                "when",
            ],
            &[&["attack", "health"]],
        ),
        Op::GrantKeyword => (
            &["keyword"],
            &["value", "duration", "condition", "when"],
            &[],
        ),
        Op::RemoveGrowth => (&[], &["condition", "when"], &[]),
        Op::CannotAttack => (&[], &["duration"], &[]),
        Op::DealDamage => (&["amount"], &["when", "canKill"], &[]),
        Op::Summon => (
            &["count"],
            &["token", "card", "when"],
            &[&["token", "card"]],
        ),
        Op::Copy => (&["to"], &["from", "type", "state"], &[]),
        Op::Discover => (&["pool", "choose"], &[], &[]),
        Op::RefreshShop => (&[], &["free", "extra"], &[]),
        Op::AddShopSlot => (&["era", "count"], &[], &[]),
        Op::GainMinion => (&["level", "to"], &["from"], &[]),
        Op::Decompose => (
            &["basis", "duration"],
            &["timesIf", "condition", "when"],
            &[],
        ),
        Op::Mimic => (&["duration"], &["statsOnly"], &[]),
        Op::Devour => (&["value"], &["duration", "when"], &[]),
        Op::Detonate => (&["amount"], &["threshold", "when"], &[]),
        Op::Convert => (
            &["threshold"],
            &["token", "card", "count", "when"],
            &[&["token", "card"]],
        ),
        Op::Revive => (&["attack", "health"], &[], &[]),
        Op::Recall => (&["to"], &[], &[]),
        Op::Salvage => (&["pool", "count"], &[], &[]),
        Op::Latent => (&["amount", "count"], &["threshold", "when"], &[]),
    }
}

impl Params {
    /// 该 DSL 参数在结构化字段中是否存在。
    pub(crate) fn has(&self, key: &str) -> bool {
        match key {
            "times" => self.times.is_some(),
            "count" => self.count.is_some(),
            "amount" => self.amount.is_some(),
            "attack" => self.attack.is_some(),
            "health" => self.health.is_some(),
            "perAttack" => self.per_attack.is_some(),
            "perHealth" => self.per_health.is_some(),
            "maxCount" => self.max_count.is_some(),
            "threshold" => self.threshold.is_some(),
            "value" => self.value.is_some(),
            "extra" => self.extra.is_some(),
            "duration" => self.duration.is_some(),
            "canKill" => self.can_kill.is_some(),
            "healthOnly" => self.health_only.is_some(),
            "statsOnly" => self.stats_only.is_some(),
            "timesIf" => self.times_if.is_some(),
            "condition" => self.condition.is_some(),
            "when" => self.when.is_some(),
            "keyword" => self.keyword.is_some(),
            "token" => self.token.is_some(),
            "card" => self.card.is_some(),
            "to" => self.to.is_some(),
            "from" => self.from.is_some(),
            "type" => self.card_type.is_some(),
            "pool" => self.pool.is_some(),
            "choose" => self.choose.is_some(),
            "free" => self.free.is_some(),
            "era" => self.era.is_some(),
            "level" => self.level.is_some(),
            "state" => self.state.is_some(),
            "basis" => self.basis.is_some(),
            _ => false,
        }
    }
}

/// 校验一段结构化效果，返回全部违规消息。
pub fn validate_effect(effect: &Effect) -> Vec<String> {
    let mut errors = Vec::new();
    let params = &effect.params;

    let is_aura = matches!(effect.trigger, Trigger::Aura | Trigger::ConditionalAura);

    if is_aura && !AURA_OPS.contains(&effect.op) {
        errors.push(format!("光环触发只能用 {AURA_OPS:?}，实为 {:?}", effect.op));
    }

    // 群聚参数只属于光环的 ModifyStats；非光环同名原语不得携带。
    if (params.per_attack.is_some() || params.per_health.is_some() || params.max_count.is_some())
        && !(is_aura && effect.op == Op::ModifyStats)
    {
        errors.push("perAttack/perHealth/maxCount 只能用于光环的 ModifyStats".to_string());
    }

    // 相邻选取只服务光环；其他触发器（战斗、招募）没有稳定的来源棋盘位置。
    if !is_aura
        && matches!(
            effect.selector,
            Selector::Side {
                pick: Pick::Adjacent,
                ..
            }
        )
    {
        errors.push("Adjacent 只能用于光环".to_string());
    }

    let (required, allowed, one_of) = op_spec(effect.op);
    for key in required {
        if !params.has(key) {
            errors.push(format!("原语 {:?} 缺少参数 {key:?}", effect.op));
        }
    }
    let allowed_all: Vec<&str> = required.iter().chain(allowed.iter()).copied().collect();
    for key in PARAM_KEYS {
        if params.has(key) && !allowed_all.contains(key) {
            errors.push(format!("参数 {key:?} 不属于原语 {:?}", effect.op));
        }
    }
    for group in one_of {
        if !group.iter().any(|key| params.has(key)) {
            errors.push(format!("原语 {:?} 缺少 {group:?} 之一", effect.op));
        }
    }

    if let Some(times_if) = &params.times_if {
        if effect.op != Op::Growth || params.times.is_none() {
            errors.push("timesIf 只能与 Growth 的 times 同时出现".to_string());
        }
        if times_if.times < 0 {
            errors.push("timesIf 的次数不能为负".to_string());
        }
        validate_times_if_filter(times_if, &mut errors);
    }

    if let Some(when) = &params.when {
        if !WHEN_TRIGGERS.contains(&effect.trigger) {
            errors.push(format!(
                "when 只能用于事件类触发，不能用于 {:?}",
                effect.trigger
            ));
        }
        validate_filter(when, false, &mut errors);
    }

    if let Selector::Side { filters, .. } = &effect.selector {
        for filter in filters {
            validate_filter(filter, true, &mut errors);
        }
    }

    if let Some(condition) = &params.condition {
        validate_condition_filter(&condition.filter, &mut errors);
        if condition.count < 0 {
            errors.push("condition 的 count 不能为负".to_string());
        }
    }

    if effect.op == Op::Summon {
        let present = usize::from(params.token.is_some()) + usize::from(params.card.is_some());
        if present > 1 {
            errors.push("Summon 的 token 与 card 只能二选一".to_string());
        }
    }

    if effect.op == Op::Discover
        && let Some(choose) = params.choose
        && choose != 1
    {
        errors.push(format!("Discover 的 choose 必须为 1，实为 {choose}"));
    }

    if effect.op == Op::GainMinion
        && let Some(from) = params.from
        && from != Zone::Shop
    {
        errors.push(format!("GainMinion 的 from 只接受 shop，实为 {from:?}"));
    }

    // 分解的生命值增量必须有明确时长：`permanent` 落盘，`combat` 本回合。
    if effect.op == Op::Decompose
        && params
            .duration
            .is_some_and(|duration| duration == Duration::Instant)
    {
        errors.push("Decompose 的 duration 只接受 permanent / combat".to_string());
    }

    // 拟态同样需要可回滚的时长：`instant` 会让关键词在记录时被丢弃。
    if effect.op == Op::Mimic
        && params
            .duration
            .is_some_and(|duration| duration == Duration::Instant)
    {
        errors.push("Mimic 的 duration 只接受 permanent / combat".to_string());
    }

    // 吞噬的攻击力上限不能为负。
    if effect.op == Op::Devour && params.value.is_some_and(|value| value < 0) {
        errors.push("Devour 的 value 不能为负".to_string());
    }

    if let Some(level) = &params.level
        && level != "current"
        && !is_digits(level)
    {
        errors.push(format!("level 非法 {level:?}"));
    }

    for (name, value) in [
        ("token", &params.token),
        ("card", &params.card),
        ("pool", &params.pool),
        ("era", &params.era),
    ] {
        if value.as_ref().is_some_and(|text| text.is_empty()) {
            errors.push(format!("参数 {name} 不能为空"));
        }
    }

    if let Some(keyword) = &params.keyword {
        if !KEYWORD_BASES.contains(&keyword.keyword.as_str()) {
            errors.push(format!("keyword 非法 {:?}", keyword.keyword));
        }
        let layer = keyword.value.or(params.value);
        if matches!(
            keyword.keyword.as_str(),
            "breed" | "assemble" | "contagion" | "armor"
        ) {
            if layer.is_none() {
                errors.push(format!("keyword {:?} 必须带层数", keyword.keyword));
            } else if layer.is_some_and(|value| value < 0) {
                errors.push(format!("keyword {:?} 的层数不能为负", keyword.keyword));
            }
        } else if layer.is_some() {
            errors.push(format!("keyword {:?} 禁止层数与 value", keyword.keyword));
        }
    }

    errors
}

/// 校验一组效果；全部通过时返回 `Ok(())`。
pub fn validate_effects(effects: &[Effect]) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    for effect in effects {
        errors.extend(validate_effect(effect));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_filter(filter: &Filter, check_value: bool, errors: &mut Vec<String>) {
    if !FILTER_KEYS.contains(&filter.key.as_str()) {
        errors.push(format!("未知筛选键 {:?}", filter.key));
    }
    if filter.value.is_empty() {
        errors.push(format!("筛选 {:?} 缺少取值", filter.key));
    }
    if filter.key == "scholar" && !matches!(filter.value.as_str(), "true" | "false") {
        errors.push(format!("scholar 取值非法 {:?}", filter.value));
    }
    if check_value {
        validate_filter_value(&filter.key, &filter.value, errors);
    }
}

/// `timesIf` 的键必须在筛选键集合内；除 `scholar` 外必须带值。
fn validate_times_if_filter(times_if: &TimesIf, errors: &mut Vec<String>) {
    let key = times_if.key.as_str();
    if !FILTER_KEYS.contains(&key) {
        errors.push(format!("timesIf 的筛选键非法 {:?}", times_if.key));
        return;
    }
    match times_if.value.as_deref() {
        None if key == "scholar" => {}
        None => errors.push(format!("timesIf 的 {:?} 缺少取值", key)),
        Some(value) if key == "scholar" => {
            errors.push(format!("timesIf 的 scholar 不能带取值 {value:?}"));
        }
        Some(value) => validate_filter_value(key, value, errors),
    }
}

/// 按筛选键校验取值；只覆盖有固定取值集合的键。
///
/// `playstyle` 接受 13 个玩法与 `neutral`：中立卡是可被筛中的实体玩法取值
/// （`filter_matches` 按 id 比较），只有 `from_id` 把 `neutral` 排除在可启用集合之外。
fn validate_filter_value(key: &str, value: &str, errors: &mut Vec<String>) {
    match key {
        "playstyle" if value != Playstyle::Neutral.id() && Playstyle::from_id(value).is_none() => {
            errors.push(format!("playstyle 取值非法 {value:?}"));
        }
        "type" if !CARD_TYPES.contains(&value) => {
            errors.push(format!("type 取值非法 {value:?}"));
        }
        "other" if !matches!(value, "true" | "false") => {
            errors.push(format!("other 取值非法 {value:?}"));
        }
        "scholar" if !matches!(value, "true" | "false") => {
            errors.push(format!("scholar 取值非法 {value:?}"));
        }
        _ => {}
    }
}

fn validate_condition_filter(filter: &Filter, errors: &mut Vec<String>) {
    if filter.key.is_empty() || !filter.key.chars().all(|c| c.is_ascii_lowercase()) {
        errors.push(format!("condition 的筛选键非法 {:?}", filter.key));
    }
    if filter.value.is_empty() {
        errors.push("condition 的筛选值不能为空".to_string());
    }
}

fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_digit())
}
