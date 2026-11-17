use crate::data::Tribe;

use super::{Effect, Filter, Op, Params, Selector, Trigger, Zone};

/// 光环触发允许的原语。
pub(crate) const AURA_OPS: &[Op] = &[Op::ModifyStats, Op::GrantKeyword];
/// 选择器筛选允许的键。
pub(crate) const FILTER_KEYS: &[&str] = &["tribe", "subtype", "id", "type", "other"];
/// 部落取值。
pub(crate) const TRIBES: &[&str] = &["human", "machine", "biology", "plant", "virus", "scholar"];
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
];
/// `Params` 中所有可出现的 DSL 参数名。
const PARAM_KEYS: &[&str] = &[
    "times",
    "count",
    "amount",
    "attack",
    "health",
    "value",
    "extra",
    "duration",
    "canKill",
    "healthOnly",
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
        Op::Copy => (&["to"], &["from", "type"], &[]),
        Op::Discover => (&["pool", "choose"], &[], &[]),
        Op::RefreshShop => (&[], &["free", "extra"], &[]),
        Op::AddShopSlot => (&["era", "count"], &[], &[]),
        Op::GainMinion => (&["level", "to"], &["from"], &[]),
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
            "value" => self.value.is_some(),
            "extra" => self.extra.is_some(),
            "duration" => self.duration.is_some(),
            "canKill" => self.can_kill.is_some(),
            "healthOnly" => self.health_only.is_some(),
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
            _ => false,
        }
    }
}

/// 校验一段结构化效果，返回全部违规消息。
pub fn validate_effect(effect: &Effect) -> Vec<String> {
    let mut errors = Vec::new();
    let params = &effect.params;

    if matches!(effect.trigger, Trigger::Aura | Trigger::ConditionalAura)
        && !AURA_OPS.contains(&effect.op)
    {
        errors.push(format!("光环触发只能用 {AURA_OPS:?}，实为 {:?}", effect.op));
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
        if times_if.tribe == Tribe::General {
            errors.push("timesIf 的部落不能是通用".to_string());
        }
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
        if matches!(keyword.keyword.as_str(), "breed" | "assemble" | "contagion") {
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
    if check_value {
        match filter.key.as_str() {
            "tribe" if !TRIBES.contains(&filter.value.as_str()) => {
                errors.push(format!("tribe 取值非法 {:?}", filter.value));
            }
            "type" if !CARD_TYPES.contains(&filter.value.as_str()) => {
                errors.push(format!("type 取值非法 {:?}", filter.value));
            }
            "other" if !matches!(filter.value.as_str(), "true" | "false") => {
                errors.push(format!("other 取值非法 {:?}", filter.value));
            }
            _ => {}
        }
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
