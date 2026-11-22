use super::validate::validate_effect;
use super::{
    Compare, Condition, Effect, Filter, FilterKey, KeywordSpec, Op, Params, Pick, Selector, Side,
    TimesIf, Trigger, Zone,
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
    ("OnKill", Trigger::OnKill),
    ("OnAllyKill", Trigger::OnAllyKill),
    ("OnDeath", Trigger::OnDeath),
    ("OnFriendlyDeath", Trigger::OnFriendlyDeath),
    ("OnEnemyDeath", Trigger::OnEnemyDeath),
    ("OnAllySummon", Trigger::OnAllySummon),
    ("OnAllyGrowth", Trigger::OnAllyGrowth),
    ("OnAllyEvolve", Trigger::OnAllyEvolve),
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
    ("GrantMinion", Op::GrantMinion),
    ("Decompose", Op::Decompose),
    ("Mimic", Op::Mimic),
    ("Devour", Op::Devour),
    ("Detonate", Op::Detonate),
    ("Convert", Op::Convert),
    ("Revive", Op::Revive),
    ("Recall", Op::Recall),
    ("Salvage", Op::Salvage),
    ("Latent", Op::Latent),
    ("SevenVoyages", Op::SevenVoyages),
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

    let mut parsed_params = None;
    if let Some(op) = op {
        match parse_params(op, parts.get(3).copied(), chunk) {
            Ok(params) => parsed_params = Some(params),
            Err(mut param_errors) => errors.append(&mut param_errors),
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let (Some(trigger), Some(op), Some(selector), Some(params)) =
        (trigger, op, selector, parsed_params)
    else {
        return Err(vec![format!("效果段无法解析: {chunk}")]);
    };
    let effect = Effect {
        trigger,
        op,
        selector,
        params,
    };
    let semantic = validate_effect(&effect);
    if semantic.is_empty() {
        Ok(effect)
    } else {
        Err(semantic
            .into_iter()
            .map(|message| format!("{message}（{chunk}）"))
            .collect())
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
        return Ok(Selector::Target {
            zone: Zone::Board,
            filters: Vec::new(),
        });
    }
    if let Some(rest) = raw.strip_prefix("target:") {
        return parse_target_selector(rest, chunk);
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
        Some("lowestAttack") => Some(Pick::LowestAttack),
        Some("adjacent") => Some(Pick::Adjacent),
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
        match parse_filter(segment, chunk) {
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

fn parse_target_selector(rest: &str, chunk: &str) -> Result<Selector, Vec<String>> {
    let mut errors = Vec::new();
    let segments: Vec<&str> = rest.split(':').collect();
    let zone = match segments[0].trim() {
        "hand" => Some(Zone::Hand),
        "shop" => Some(Zone::Shop),
        "board" => Some(Zone::Board),
        other => {
            errors.push(format!("目标区域非法 {other:?}（{chunk}）"));
            None
        }
    };
    let mut filters = Vec::new();
    for segment in segments.iter().skip(1) {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        match parse_filter(segment, chunk) {
            Ok(filter) => filters.push(filter),
            Err(message) => errors.push(message),
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(Selector::Target {
        zone: zone.expect("errors 为空时区域必然合法"),
        filters,
    })
}

fn assign_json(
    op: Op,
    key: &str,
    value: &str,
    chunk: &str,
    object: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<(), String> {
    let json_key = match key {
        "perAttack" => "per_attack",
        "perHealth" => "per_health",
        "maxCount" => "max_count",
        "canKill" => "can_kill",
        "healthOnly" => "health_only",
        "statsOnly" => "stats_only",
        "timesIf" => "times_if",
        "type" => "card_type",
        other => other,
    };
    let entry = match key {
        "times" | "count" | "amount" | "attack" | "health" | "perAttack" | "perHealth"
        | "maxCount" | "threshold" | "value" => {
            serde_json::Value::from(parse_i32(key, value, chunk)?)
        }
        "duration" => serde_json::Value::from(match value {
            "permanent" => "permanent",
            "combat" => "combat",
            _ => return Err(format!("duration 非法 {value:?}（{chunk}）")),
        }),
        "canKill" | "healthOnly" | "statsOnly" | "free" => {
            serde_json::Value::from(parse_bool(key, value, chunk)?)
        }
        "timesIf" => serde_json::to_value(parse_times_if(value, chunk)?)
            .map_err(|err| format!("timesIf 非法 {value:?}（{chunk}）：{err}"))?,
        "condition" => serde_json::to_value(parse_condition(value, chunk)?)
            .map_err(|err| format!("condition 非法 {value:?}（{chunk}）：{err}"))?,
        "when" => serde_json::to_value(parse_filter(value, chunk)?)
            .map_err(|err| format!("when 非法 {value:?}（{chunk}）：{err}"))?,
        "keyword" => serde_json::to_value(parse_keyword(value, chunk)?)
            .map_err(|err| format!("keyword 非法 {value:?}（{chunk}）：{err}"))?,
        "token" => serde_json::Value::from(non_empty(key, value, chunk)?),
        "card" => serde_json::Value::from(non_empty(key, value, chunk)?),
        "to" | "from" => serde_json::to_value(parse_zone(key, value, chunk)?)
            .map_err(|err| format!("参数 {key} 非法 {value:?}（{chunk}）：{err}"))?,
        "type" => serde_json::Value::from(match value {
            "minion" => "minion",
            "effect" => "effect",
            _ => return Err(format!("参数 {key} 非法 {value:?}（{chunk}）")),
        }),
        "pool" | "era" | "level" => serde_json::Value::from(non_empty(key, value, chunk)?),
        "choose" => serde_json::Value::from(parse_u8(key, value, chunk)?),
        "state" => serde_json::Value::from(match value {
            "base" => "base",
            "current" => "current",
            _ => return Err(format!("state 非法 {value:?}（{chunk}）")),
        }),
        "basis" => serde_json::Value::from(match value {
            "original" => "original",
            "combat_start" => "combat_start",
            "pre_kill" => "pre_kill",
            _ => return Err(format!("basis 非法 {value:?}（{chunk}）")),
        }),
        other => {
            return Err(format!("参数 {other:?} 不属于原语 {op:?}（{chunk}）"));
        }
    };
    object.insert(json_key.to_string(), entry);
    Ok(())
}

/// 把 `keyword` 的内联层数规范化：`breed 2` 与 `keyword=breed,value=2` 同义。
fn normalize_keyword_value(object: &mut serde_json::Map<String, serde_json::Value>) {
    let Some(serde_json::Value::Object(keyword)) = object.get("keyword").cloned() else {
        return;
    };
    let Some(serde_json::Value::String(base)) = keyword.get("keyword") else {
        return;
    };
    if keyword.get("value").is_some_and(|value| !value.is_null())
        || !matches!(base.as_str(), "breed" | "assemble" | "contagion")
    {
        return;
    }
    let Some(value) = object
        .get("value")
        .filter(|value| !value.is_null())
        .cloned()
    else {
        return;
    };
    let mut keyword = keyword;
    keyword.insert("value".to_string(), value);
    object.insert("keyword".to_string(), serde_json::Value::Object(keyword));
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
    let Some(key) = FilterKey::from_id(key.trim()) else {
        return Err(invalid());
    };
    let filter_value = filter_value.trim().to_string();

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
            key,
            value: filter_value,
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
    let level = match words.next() {
        None => None,
        Some(level) if is_digits(level) && words.next().is_none() => Some(
            level
                .parse()
                .map_err(|_| format!("keyword 非法 {value:?}（{chunk}）"))?,
        ),
        Some(_) => return Err(format!("keyword 非法 {value:?}（{chunk}）")),
    };

    Ok(KeywordSpec {
        keyword: base.to_string(),
        value: level,
    })
}

fn parse_zone(key: &str, value: &str, chunk: &str) -> Result<Zone, String> {
    match value {
        "hand" => Ok(Zone::Hand),
        "board" => Ok(Zone::Board),
        "shop" => Ok(Zone::Shop),
        _ => Err(format!("参数 {key} 非法 {value:?}（{chunk}）")),
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

fn parse_filter(raw: &str, chunk: &str) -> Result<Filter, String> {
    let Some((key, value)) = raw.split_once('=') else {
        return Err(format!("筛选缺少 =: {raw:?}（{chunk}）"));
    };
    let key = key.trim();
    let Some(key) = FilterKey::from_id(key) else {
        return Err(format!("未知筛选键 {key:?}（{chunk}）"));
    };
    Ok(Filter {
        key,
        value: value.trim().to_string(),
    })
}

fn parse_params(op: Op, raw: Option<&str>, chunk: &str) -> Result<Params, Vec<String>> {
    let mut object = serde_json::Map::new();
    let mut errors = Vec::new();
    let mut seen: Vec<String> = Vec::new();

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
            let value = value.trim();
            if seen.contains(&key) {
                errors.push(format!("参数重复 {key:?}（{chunk}）"));
            }
            seen.push(key.clone());
            if let Err(message) = assign_json(op, &key, value, chunk, &mut object) {
                errors.push(message);
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    normalize_keyword_value(&mut object);
    super::decode_params(op, Some(serde_json::Value::Object(object)))
        .map_err(|err| vec![format!("{err}（{chunk}）")])
}

fn parse_times_if(value: &str, chunk: &str) -> Result<TimesIf, String> {
    let invalid = || format!("timesIf 格式非法 {value:?}（{chunk}）");
    let parts: Vec<&str> = value.split(':').map(str::trim).collect();
    let (key, filter_value, count) = match parts.as_slice() {
        // `playstyle:mammal:2`：键、值、次数。
        [key, filter_value, count] => {
            let Some(key) = FilterKey::from_id(key) else {
                return Err(invalid());
            };
            (key, filter_value.to_string(), *count)
        }
        _ => return Err(invalid()),
    };
    if filter_value.is_empty() || !is_digits(count) {
        return Err(invalid());
    }
    Ok(TimesIf {
        key,
        value: filter_value,
        times: count.parse().map_err(|_| invalid())?,
    })
}
