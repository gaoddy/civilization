use crate::data::CardType;

use super::{
    Compare, Condition, Duration, Effect, Filter, KeywordSpec, Op, Params, Pick, Selector, Side,
    Trigger, Zone,
};

const TRIGGERS: &[(&str, Trigger)] = &[
    ("OnPlay", Trigger::OnPlay),
    ("OnPlayEffect", Trigger::OnPlayEffect),
    ("OnUse", Trigger::OnUse),
    ("OnActivate", Trigger::OnActivate),
    ("OnTurnStart", Trigger::OnTurnStart),
    ("OnFirstRefresh", Trigger::OnFirstRefresh),
    ("OnCombatStart", Trigger::OnCombatStart),
    ("OnDealDamage", Trigger::OnDealDamage),
    ("OnDeath", Trigger::OnDeath),
    ("OnFriendlyDeath", Trigger::OnFriendlyDeath),
    ("OnEnemyDeath", Trigger::OnEnemyDeath),
    ("OnAllySummon", Trigger::OnAllySummon),
    ("OnAllyGrowth", Trigger::OnAllyGrowth),
    ("OnAllyEvolve", Trigger::OnAllyEvolve),
    ("OnCombatEnd", Trigger::OnCombatEnd),
    ("Aura", Trigger::Aura),
    ("ConditionalAura", Trigger::ConditionalAura),
];

const OPS: &[(&str, Op)] = &[
    ("Growth", Op::Growth),
    ("ModifyStats", Op::ModifyStats),
    ("GrantKeyword", Op::GrantKeyword),
    ("RemoveGrowth", Op::RemoveGrowth),
    ("CannotAttack", Op::CannotAttack),
    ("DealDamage", Op::DealDamage),
    ("Summon", Op::Summon),
    ("Copy", Op::Copy),
    ("Discover", Op::Discover),
    ("RefreshShop", Op::RefreshShop),
    ("AddShopSlot", Op::AddShopSlot),
    ("GainMinion", Op::GainMinion),
];

const AURA_OPS: &[Op] = &[Op::ModifyStats, Op::GrantKeyword];
const FILTER_KEYS: &[&str] = &["tribe", "subtype", "name", "type", "other"];
const TRIBES: &[&str] = &["人类", "机械", "生物", "植物", "病毒", "学者"];
const CARD_TYPES: &[&str] = &["随从", "效果"];
const KEYWORD_BASES: &[&str] = &[
    "战吼",
    "亡语",
    "复生",
    "嘲讽",
    "护盾",
    "连击",
    "传染",
    "最优攻击",
    "播种",
    "进化",
    "繁衍",
    "装配",
];

/// 把单个技能的 `effects` 字符串解析为类型化 AST。
///
/// `"[]"` 表示没有效果，返回空 `Vec`。语法错误会全部累积后返回，
/// 每条消息都带上原始效果段，便于定位。
pub fn parse_effects(raw: &str) -> Result<Vec<Effect>, Vec<String>> {
    let raw = raw.trim();
    if raw == "[]" {
        return Ok(Vec::new());
    }

    let mut effects = Vec::new();
    let mut errors = Vec::new();
    for chunk in raw.split(';') {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            errors.push("effects 含空效果段".to_string());
            continue;
        }
        match parse_chunk(chunk) {
            Ok(effect) => effects.push(effect),
            Err(mut chunk_errors) => errors.append(&mut chunk_errors),
        }
    }

    if errors.is_empty() {
        Ok(effects)
    } else {
        Err(errors)
    }
}

fn parse_chunk(chunk: &str) -> Result<Effect, Vec<String>> {
    let parts: Vec<&str> = chunk.split('|').collect();
    if parts.len() != 3 && parts.len() != 4 {
        return Err(vec![format!("effects 段数非法（需 3–4 段）: {chunk}")]);
    }

    let mut errors = Vec::new();

    let trigger = match lookup(TRIGGERS, parts[0].trim()) {
        Some(trigger) => Some(trigger),
        None => {
            errors.push(format!("未知触发 {:?}（{chunk}）", parts[0].trim()));
            None
        }
    };
    let op = match lookup(OPS, parts[1].trim()) {
        Some(op) => Some(op),
        None => {
            errors.push(format!("未知原语 {:?}（{chunk}）", parts[1].trim()));
            None
        }
    };

    let selector = match parse_selector(parts[2].trim(), chunk) {
        Ok(selector) => Some(selector),
        Err(mut selector_errors) => {
            errors.append(&mut selector_errors);
            None
        }
    };

    let mut params = Params::default();
    if let Some(op) = op {
        let (parsed, mut param_errors) = parse_params(op, parts.get(3).copied(), chunk);
        params = parsed;
        errors.append(&mut param_errors);
    }

    if let (Some(trigger), Some(op)) = (trigger, op)
        && matches!(trigger, Trigger::Aura | Trigger::ConditionalAura)
        && !AURA_OPS.contains(&op)
    {
        errors.push(format!(
            "光环触发只能用 {AURA_OPS:?}，实为 {op:?}（{chunk}）"
        ));
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    match (trigger, op, selector) {
        (Some(trigger), Some(op), Some(selector)) => Ok(Effect {
            trigger,
            op,
            selector,
            params,
        }),
        _ => Err(vec![format!("效果段无法解析: {chunk}")]),
    }
}

fn lookup<T: Copy>(table: &[(&str, T)], name: &str) -> Option<T> {
    table
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, value)| *value)
}

fn parse_selector(raw: &str, chunk: &str) -> Result<Selector, Vec<String>> {
    if raw == "self" {
        return Ok(Selector::SelfCard);
    }
    if raw == "target" {
        return Ok(Selector::Target);
    }

    let mut errors = Vec::new();
    let segments: Vec<&str> = raw.split(':').collect();

    let side = match segments[0].trim() {
        "friendly" => Some(Side::Friendly),
        "enemy" => Some(Side::Enemy),
        "both" => Some(Side::Both),
        other => {
            errors.push(format!("选择器阵营非法 {other:?}（{chunk}）"));
            None
        }
    };

    let pick = match segments.get(1).map(|segment| segment.trim()) {
        Some("single") => Some(Pick::Single),
        Some("all") => Some(Pick::All),
        Some("random") => Some(Pick::Random),
        Some("leftmost") => Some(Pick::Leftmost),
        Some("leftmost+adjacent") => Some(Pick::LeftmostAdjacent),
        Some("topAttack") => Some(Pick::TopAttack),
        Some(other) if !other.is_empty() => {
            errors.push(format!("选择器选取非法 {other:?}（{chunk}）"));
            None
        }
        _ => {
            errors.push(format!("选择器缺少选取 {raw:?}（{chunk}）"));
            None
        }
    };

    let mut filters = Vec::new();
    for segment in segments.iter().skip(2) {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        match parse_filter(segment, chunk, true) {
            Ok(filter) => filters.push(filter),
            Err(message) => errors.push(message),
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    match (side, pick) {
        (Some(side), Some(pick)) => Ok(Selector::Side {
            side,
            pick,
            filters,
        }),
        _ => Err(vec![format!("选择器无法解析 {raw:?}（{chunk}）")]),
    }
}

fn parse_filter(raw: &str, chunk: &str, check_value: bool) -> Result<Filter, String> {
    let Some((key, value)) = raw.split_once('=') else {
        return Err(format!("筛选缺少 =: {raw:?}（{chunk}）"));
    };
    let key = key.trim();
    let value = value.trim();

    if !FILTER_KEYS.contains(&key) {
        return Err(format!("未知筛选键 {key:?}（{chunk}）"));
    }
    if value.is_empty() {
        return Err(format!("筛选 {key:?} 缺少取值（{chunk}）"));
    }
    if check_value {
        match key {
            "tribe" if !TRIBES.contains(&value) => {
                return Err(format!("tribe 取值非法 {value:?}（{chunk}）"));
            }
            "type" if !CARD_TYPES.contains(&value) => {
                return Err(format!("type 取值非法 {value:?}（{chunk}）"));
            }
            "other" if !matches!(value, "true" | "false") => {
                return Err(format!("other 取值非法 {value:?}（{chunk}）"));
            }
            _ => {}
        }
    }

    Ok(Filter {
        key: key.to_string(),
        value: value.to_string(),
    })
}

fn parse_params(op: Op, raw: Option<&str>, chunk: &str) -> (Params, Vec<String>) {
    let mut params = Params::default();
    let mut errors = Vec::new();
    let mut pairs: Vec<(String, String)> = Vec::new();

    if let Some(raw) = raw {
        for pair in raw.split(',') {
            let pair = pair.trim();
            if pair.is_empty() {
                continue;
            }
            let Some((key, value)) = pair.split_once('=') else {
                errors.push(format!("参数缺少 =: {pair:?}（{chunk}）"));
                continue;
            };
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            if pairs.iter().any(|(existing, _)| *existing == key) {
                errors.push(format!("参数重复 {key:?}（{chunk}）"));
            }
            pairs.push((key, value));
        }
    }

    let (required, optional, one_of) = op_spec(op);

    for (key, value) in &pairs {
        if !required.contains(&key.as_str()) && !optional.contains(&key.as_str()) {
            errors.push(format!("参数 {key:?} 不属于原语 {op:?}（{chunk}）"));
            continue;
        }
        if let Err(message) = assign(&mut params, op, key, value, chunk) {
            errors.push(message);
        }
    }

    for key in required {
        if !pairs.iter().any(|(existing, _)| existing == key) {
            errors.push(format!("原语 {op:?} 缺少必填参数 {key:?}（{chunk}）"));
        }
    }

    for group in one_of {
        let satisfied = group
            .iter()
            .any(|key| pairs.iter().any(|(existing, _)| existing == key));
        if !satisfied {
            errors.push(format!("原语 {op:?} 缺少 {group:?} 之一（{chunk}）"));
        }
    }

    // 繁衍 / 装配 的层数可以直接写在 keyword 上，也可以由 value 参数承载；
    // 后者统一规范化为 `Keyword(name, Some(n))`，供执行器只读 keyword。
    if let Some(KeywordSpec::Keyword(base, level)) = &mut params.keyword
        && level.is_none()
        && matches!(base.as_str(), "繁衍" | "装配")
        && let Some(value) = params.value
    {
        *level = Some(value);
    }

    (params, errors)
}

type OpSpec = (
    &'static [&'static str],
    &'static [&'static str],
    &'static [&'static [&'static str]],
);

fn op_spec(op: Op) -> OpSpec {
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
        Op::DealDamage => (&["amount"], &["when"], &[]),
        Op::Summon => (
            &["count"],
            &["token", "card", "when"],
            &[&["token", "card"]],
        ),
        Op::Copy => (&["to"], &["from", "type"], &[]),
        Op::Discover => (&["pool"], &["choose"], &[]),
        Op::RefreshShop => (&[], &["free", "extra"], &[]),
        Op::AddShopSlot => (&["era", "count"], &[], &[]),
        Op::GainMinion => (&["level", "to"], &["from"], &[]),
    }
}

fn assign(params: &mut Params, op: Op, key: &str, value: &str, chunk: &str) -> Result<(), String> {
    match key {
        "times" => params.times = Some(parse_i32(key, value, chunk)?),
        "count" => params.count = Some(parse_i32(key, value, chunk)?),
        "amount" => params.amount = Some(parse_i32(key, value, chunk)?),
        "attack" => params.attack = Some(parse_i32(key, value, chunk)?),
        "health" => params.health = Some(parse_i32(key, value, chunk)?),
        "value" => params.value = Some(parse_i32(key, value, chunk)?),
        "extra" => params.extra = Some(parse_i32(key, value, chunk)?),
        "duration" => {
            params.duration = match value {
                "permanent" => Duration::Permanent,
                "combat" => Duration::Combat,
                _ => return Err(format!("duration 非法 {value:?}（{chunk}）")),
            }
        }
        "canKill" => params.can_kill = Some(parse_bool(key, value, chunk)?),
        "healthOnly" => params.health_only = Some(parse_bool(key, value, chunk)?),
        "free" => params.free = Some(parse_bool(key, value, chunk)?),
        "timesIf" => params.times_if = Some(parse_times_if(value, chunk)?),
        "condition" => params.condition = Some(parse_condition(value, chunk)?),
        "when" => params.when = Some(parse_filter(value, chunk, false)?),
        "keyword" => params.keyword = Some(parse_keyword(value, chunk)?),
        "token" => params.token = Some(non_empty(key, value, chunk)?),
        "card" => params.card = Some(non_empty(key, value, chunk)?),
        "to" => params.to = Some(parse_zone(key, value, chunk)?),
        "from" => params.from = Some(parse_zone(key, value, chunk)?),
        "type" => params.card_type = Some(parse_card_type(value, chunk)?),
        "pool" => params.pool = Some(non_empty(key, value, chunk)?),
        "choose" => {
            let choose = parse_u8(key, value, chunk)?;
            if choose != 1 {
                return Err(format!("choose 必须为 1，实为 {choose}（{chunk}）"));
            }
            params.choose = Some(choose);
        }
        "era" => params.era = Some(non_empty(key, value, chunk)?),
        "level" => {
            if value != "current" && !is_digits(value) {
                return Err(format!("参数 level 非法 {value:?}（{chunk}）"));
            }
            params.level = Some(value.to_string());
        }
        other => {
            return Err(format!("参数 {other:?} 不属于原语 {op:?}（{chunk}）"));
        }
    }
    Ok(())
}

fn parse_times_if(value: &str, chunk: &str) -> Result<(String, i32), String> {
    let Some((tribe, count)) = value.split_once(':') else {
        return Err(format!("timesIf 格式非法 {value:?}（{chunk}）"));
    };
    let tribe = tribe.trim();
    let count = count.trim();
    if !TRIBES.contains(&tribe) || !is_digits(count) {
        return Err(format!("timesIf 格式非法 {value:?}（{chunk}）"));
    }
    Ok((
        tribe.to_string(),
        count
            .parse()
            .map_err(|_| format!("timesIf 格式非法 {value:?}（{chunk}）"))?,
    ))
}

fn parse_condition(value: &str, chunk: &str) -> Result<Condition, String> {
    let invalid = || format!("condition 格式非法 {value:?}（{chunk}）");
    let Some(rest) = value.strip_prefix("count:") else {
        return Err(invalid());
    };
    let segments: Vec<&str> = rest.split(':').collect();
    if segments.len() != 3 {
        return Err(invalid());
    }

    let side = match segments[0] {
        "friendly" => Side::Friendly,
        "enemy" => Side::Enemy,
        "both" => Side::Both,
        _ => return Err(invalid()),
    };

    let Some((key, filter_value)) = segments[1].split_once('=') else {
        return Err(invalid());
    };
    let key = key.trim();
    let filter_value = filter_value.trim();
    if key.is_empty() || !key.chars().all(|c| c.is_ascii_lowercase()) || filter_value.is_empty() {
        return Err(invalid());
    }

    let (compare, digits) = if let Some(digits) = segments[2].strip_prefix(">=") {
        (Compare::Ge, digits)
    } else if let Some(digits) = segments[2].strip_prefix("<=") {
        (Compare::Le, digits)
    } else if let Some(digits) = segments[2].strip_prefix("==") {
        (Compare::Eq, digits)
    } else if let Some(digits) = segments[2].strip_prefix('>') {
        (Compare::Gt, digits)
    } else if let Some(digits) = segments[2].strip_prefix('<') {
        (Compare::Lt, digits)
    } else {
        return Err(invalid());
    };
    if !is_digits(digits) {
        return Err(invalid());
    }

    Ok(Condition {
        side,
        filter: Filter {
            key: key.to_string(),
            value: filter_value.to_string(),
        },
        compare,
        count: digits.parse().map_err(|_| invalid())?,
    })
}

fn parse_keyword(value: &str, chunk: &str) -> Result<KeywordSpec, String> {
    let mut words = value.split_whitespace();
    let Some(base) = words.next() else {
        return Err(format!("keyword 非法 {value:?}（{chunk}）"));
    };
    if !KEYWORD_BASES.contains(&base) {
        return Err(format!("keyword 非法 {value:?}（{chunk}）"));
    }

    let level = match words.next() {
        None => None,
        Some(level) if is_digits(level) && words.next().is_none() => Some(
            level
                .parse()
                .map_err(|_| format!("keyword 非法 {value:?}（{chunk}）"))?,
        ),
        Some(_) => return Err(format!("keyword 非法 {value:?}（{chunk}）")),
    };

    Ok(KeywordSpec::Keyword(base.to_string(), level))
}

fn parse_zone(key: &str, value: &str, chunk: &str) -> Result<Zone, String> {
    match value {
        "hand" => Ok(Zone::Hand),
        "board" => Ok(Zone::Board),
        "shop" => Ok(Zone::Shop),
        _ => Err(format!("参数 {key} 非法 {value:?}（{chunk}）")),
    }
}

fn parse_card_type(value: &str, chunk: &str) -> Result<CardType, String> {
    match value {
        "随从" => Ok(CardType::Minion),
        "效果" => Ok(CardType::Effect),
        _ => Err(format!("参数 type 非法 {value:?}（{chunk}）")),
    }
}

fn parse_i32(key: &str, value: &str, chunk: &str) -> Result<i32, String> {
    if !is_integer(value) {
        return Err(format!("参数 {key} 必须为整数，实为 {value:?}（{chunk}）"));
    }
    value
        .parse()
        .map_err(|_| format!("参数 {key} 必须为整数，实为 {value:?}（{chunk}）"))
}

fn parse_u8(key: &str, value: &str, chunk: &str) -> Result<u8, String> {
    if !is_digits(value) {
        return Err(format!("参数 {key} 必须为整数，实为 {value:?}（{chunk}）"));
    }
    value
        .parse()
        .map_err(|_| format!("参数 {key} 必须为整数，实为 {value:?}（{chunk}）"))
}

fn parse_bool(key: &str, value: &str, chunk: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!(
            "参数 {key} 必须为布尔值，实为 {value:?}（{chunk}）"
        )),
    }
}

fn non_empty(key: &str, value: &str, chunk: &str) -> Result<String, String> {
    if value.is_empty() {
        return Err(format!("参数 {key} 缺少取值（{chunk}）"));
    }
    Ok(value.to_string())
}

fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_digit())
}

fn is_integer(text: &str) -> bool {
    let body = text.strip_prefix('-').unwrap_or(text);
    !body.is_empty() && body.chars().all(|c| c.is_ascii_digit())
}
