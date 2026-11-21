use crate::data::Playstyle;

use super::{Duration, Effect, Filter, FilterKey, Op, Params, Pick, Selector, Trigger, Zone};

/// 光环触发允许的原语。
pub(crate) const AURA_OPS: &[Op] = &[Op::ModifyStats, Op::GrantKeyword];
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
    "ignore_taunt",
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
    Trigger::OnUse,
    Trigger::OnDeath,
    Trigger::OnKill,
    Trigger::OnAllyKill,
];

/// 校验一段结构化效果，返回全部违规消息。
///
/// 参数的必填、互斥与枚举取值已在解析或加载期按原语类型化校验；这里只保留
/// 类型层面无法表达、且需要读触发器或选择器的规则。
pub fn validate_effect(effect: &Effect) -> Vec<String> {
    let mut errors = Vec::new();
    let is_aura = matches!(effect.trigger, Trigger::Aura | Trigger::ConditionalAura);

    if is_aura && !AURA_OPS.contains(&effect.op) {
        errors.push(format!("光环触发只能用 {AURA_OPS:?}，实为 {:?}", effect.op));
    }

    // 群聚参数只属于光环的 ModifyStats。
    if let Params::ModifyStats(params) = &effect.params {
        let clustered = params.per_attack.is_some()
            || params.per_health.is_some()
            || params.max_count.is_some();
        if clustered && !is_aura {
            errors.push("perAttack/perHealth/maxCount 只能用于光环的 ModifyStats".to_string());
        }
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

    if let Params::Growth(params) = &effect.params
        && let Some(times_if) = &params.times_if
    {
        if times_if.times < 0 {
            errors.push("timesIf 的次数不能为负".to_string());
        }
        validate_times_if_filter(times_if, &mut errors);
    }

    if let Some(when) = effect.params.when() {
        if !WHEN_TRIGGERS.contains(&effect.trigger) {
            errors.push(format!(
                "when 只能用于事件类触发，不能用于 {:?}",
                effect.trigger
            ));
        }
        validate_filter(when, false, &mut errors);
    }

    if let Selector::Side { filters, .. } | Selector::Target { filters, .. } = &effect.selector {
        for filter in filters {
            validate_filter(filter, true, &mut errors);
        }
    }

    if let Some(condition) = effect.params.condition() {
        validate_condition_filter(&condition.filter, &mut errors);
        if condition.count < 0 {
            errors.push("condition 的 count 不能为负".to_string());
        }
    }

    match &effect.params {
        Params::Summon(params) => {
            validate_token_or_card(
                Op::Summon,
                params.token.as_ref(),
                params.card.as_ref(),
                &mut errors,
            );
        }
        Params::Convert(params) => {
            validate_token_or_card(
                Op::Convert,
                params.token.as_ref(),
                params.card.as_ref(),
                &mut errors,
            );
            if params.count.is_some_and(|count| count < 0) {
                errors.push("Convert 的 count 不能为负".to_string());
            }
        }
        Params::ModifyStats(params) => {
            if params.attack.is_none() && params.health.is_none() {
                errors.push("原语 ModifyStats 缺少 [\"attack\", \"health\"] 之一".to_string());
            }
        }
        Params::Discover(params) => {
            if params.pool.is_empty() {
                errors.push("参数 pool 不能为空".to_string());
            }
            if params.choose != 1 {
                errors.push(format!(
                    "Discover 的 choose 必须为 1，实为 {}",
                    params.choose
                ));
            }
        }
        Params::RefreshShop(_) => {}
        Params::AddShopSlot(params) => {
            if params.era.is_empty() {
                errors.push("参数 era 不能为空".to_string());
            }
            if params.count < 0 {
                errors.push("AddShopSlot 的 count 不能为负".to_string());
            }
        }
        Params::GainMinion(params) => {
            validate_level(&params.level, false, &mut errors);
            if params.from.is_some_and(|from| from != Zone::Shop) {
                errors.push(format!(
                    "GainMinion 的 from 只接受 shop，实为 {:?}",
                    params.from
                ));
            }
        }
        Params::GrantMinion(params) => {
            validate_level(&params.level, true, &mut errors);
            if params.to != Zone::Hand {
                errors.push(format!(
                    "GrantMinion 的 to 只接受 hand，实为 {:?}",
                    params.to
                ));
            }
        }
        Params::Copy(params) => {
            // 从手牌复制的目标必须由玩家指定：禁止再出现「静默取手牌第一张」的数据。
            if params.from == Some(Zone::Hand)
                && !matches!(
                    effect.selector,
                    Selector::Target {
                        zone: Zone::Hand,
                        ..
                    }
                )
            {
                errors.push("Copy 从手牌复制必须使用 target:hand 选择器".to_string());
            }
        }
        // 分解的生命值增量必须有明确时长：`permanent` 落盘，`combat` 本回合。
        Params::Decompose(params) if params.duration == Duration::Instant => {
            errors.push("Decompose 的 duration 只接受 permanent / combat".to_string());
        }
        // 拟态同样需要可回滚的时长：`instant` 会让关键词在记录时被丢弃。
        Params::Mimic(params) if params.duration == Duration::Instant => {
            errors.push("Mimic 的 duration 只接受 permanent / combat".to_string());
        }
        Params::Devour(params) => {
            if params.value < 0 {
                errors.push("Devour 的 value 不能为负".to_string());
            }
        }
        Params::Salvage(params) => {
            if params.pool.is_empty() {
                errors.push("参数 pool 不能为空".to_string());
            }
            if params.count < 0 {
                errors.push("Salvage 的 count 不能为负".to_string());
            }
        }
        Params::GrantKeyword(params) => {
            validate_keyword(
                &params.keyword.keyword,
                params.keyword.value.or(params.value),
                &mut errors,
            );
        }
        _ => {}
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

fn validate_token_or_card(
    op: Op,
    token: Option<&String>,
    card: Option<&String>,
    errors: &mut Vec<String>,
) {
    if token.is_none() && card.is_none() {
        errors.push(format!("原语 {op:?} 缺少 [\"token\", \"card\"] 之一"));
    }
    if token.is_some() && card.is_some() {
        errors.push(format!("{op:?} 的 token 与 card 只能二选一"));
    }
    for (name, value) in [("token", token), ("card", card)] {
        if value.is_some_and(|text| text.is_empty()) {
            errors.push(format!("参数 {name} 不能为空"));
        }
    }
}

fn validate_level(level: &str, allow_next: bool, errors: &mut Vec<String>) {
    if level == "current" || is_digits(level) || (allow_next && level == "next") {
        return;
    }
    errors.push(format!("level 非法 {level:?}"));
}

fn validate_keyword(base: &str, layer: Option<i32>, errors: &mut Vec<String>) {
    if !KEYWORD_BASES.contains(&base) {
        errors.push(format!("keyword 非法 {base:?}"));
    }
    if matches!(base, "breed" | "assemble" | "contagion" | "armor") {
        match layer {
            None => errors.push(format!("keyword {base:?} 必须带层数")),
            Some(value) if value < 0 => errors.push(format!("keyword {base:?} 的层数不能为负")),
            Some(_) => {}
        }
    } else if layer.is_some() {
        errors.push(format!("keyword {base:?} 禁止层数与 value"));
    }
}

fn validate_filter(filter: &Filter, check_value: bool, errors: &mut Vec<String>) {
    if filter.value.is_empty() {
        errors.push(format!("筛选 {:?} 缺少取值", filter.key.id()));
    }
    if check_value {
        validate_filter_value(filter.key, &filter.value, errors);
    }
}

/// `timesIf` 必须带值；键已在解析期枚举化。
fn validate_times_if_filter(times_if: &super::TimesIf, errors: &mut Vec<String>) {
    validate_filter_value(times_if.key, &times_if.value, errors);
}

/// 按筛选键校验取值；只覆盖有固定取值集合的键。
///
/// `playstyle` 接受 13 个玩法与 `neutral`：中立卡是可被筛中的实体玩法取值
/// （`filter_matches` 按 id 比较），只有 `from_id` 把 `neutral` 排除在可启用集合之外。
fn validate_filter_value(key: FilterKey, value: &str, errors: &mut Vec<String>) {
    match key {
        FilterKey::Playstyle
            if value != Playstyle::Neutral.id() && Playstyle::from_id(value).is_none() =>
        {
            errors.push(format!("playstyle 取值非法 {value:?}"));
        }
        FilterKey::Type if !CARD_TYPES.contains(&value) => {
            errors.push(format!("type 取值非法 {value:?}"));
        }
        FilterKey::Other if !matches!(value, "true" | "false") => {
            errors.push(format!("other 取值非法 {value:?}"));
        }
        _ => {}
    }
}

fn validate_condition_filter(filter: &Filter, errors: &mut Vec<String>) {
    if filter.value.is_empty() {
        errors.push("condition 的筛选值不能为空".to_string());
    }
}

fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_digit())
}
