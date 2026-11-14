#!/usr/bin/env python3
"""校验 docs/versions/v0.1/product/content 下的技能与卡牌实体文档。

只使用标准库；front matter 仅支持本项目生成的三种形式：
    key: 标量
    skills: [技能 id, ...]
    skill: 技能 id
卡牌分两类：card_type=随从 必须有攻击力/生命值；card_type=效果 必须没有。
随从卡与英雄卡通过 skills / hero_skill 引用技能实体，效果文本与关键词由被引用技能派生。
"""

import collections
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent / "docs" / "versions" / "v0.1" / "product" / "content"

ID_RE = re.compile(r"[a-z][a-z0-9-]*")
KEYWORD_RE = re.compile(
    r"^(无|战吼|亡语|复生|嘲讽|护盾|连击|传染|最优攻击|繁衍 \d+|装配 \d+|播种|进化)$"
)
# 技能上的 keyword 是携带者自身登记的常驻关键词，不接受卡牌层的「无」哨兵值。
SKILL_KEYWORD_RE = re.compile(
    r"^(战吼|亡语|复生|嘲讽|护盾|连击|传染|最优攻击|繁衍 \d+|装配 \d+|播种|进化)$"
)
AURA_RE = re.compile(r"^你的(其他)?(人类|机械|生物|植物|病毒|学者)?随从获得 ?(.+)$")
AURA_VALUE_RE = re.compile(
    r"^(?:\+\d+/\+\d+|\+\d+ 攻击力|\+\d+ 生命值|最优攻击|嘲讽|护盾|复生|连击)$"
)
TRIBE_KEYWORD = {"装配": "机械", "播种": "植物", "进化": "生物"}
TRIBES = {"人类", "机械", "生物", "植物", "病毒", "学者"}
EFFECT_TRIBES = TRIBES | {"通用"}
ERAS = ["史前", "原始", "古典", "封建", "工业", "现代", "未来"]
CARD_TYPES = {"随从", "效果"}
SKILL_TRIGGERS = {"主动使用", "首次刷新时", "战斗开始时"}
ACHIEVEMENT_EVENTS = {"时代升级", "关键词组合", "成长触发", "卡牌使用", "三连合成"}
EFFECT_TRIGGERS = {
    "OnPlay", "OnPlayEffect", "OnUse", "OnActivate", "OnTurnStart",
    "OnFirstRefresh", "OnCombatStart", "OnDealDamage", "OnDeath",
    "OnFriendlyDeath", "OnEnemyDeath", "OnAllySummon", "OnAllyGrowth",
    "OnAllyEvolve", "OnCombatEnd", "Aura", "ConditionalAura",
}
EFFECT_OPS = {
    "Growth", "ModifyStats", "GrantKeyword", "RemoveGrowth", "CannotAttack",
    "DealDamage", "Summon", "Copy", "Discover", "RefreshShop", "AddShopSlot",
    "GainMinion",
}
SELECTOR_SIDES = {"friendly", "enemy", "both", "self", "target"}
SELECTOR_PICKS = {"single", "all", "random", "leftmost", "leftmost+adjacent", "topAttack"}
FILTER_KEYS = {"tribe", "subtype", "name", "type", "other"}
DURATIONS = {"permanent", "combat"}
KEYWORD_BASES = {"战吼", "亡语", "复生", "嘲讽", "护盾", "连击", "传染",
                 "最优攻击", "播种", "进化", "繁衍", "装配"}
NUMERIC_KEYS = {"times", "count", "amount", "attack", "health", "value", "extra"}
AURA_OPS = {"ModifyStats", "GrantKeyword"}
OP_SPEC = {
    "Growth": ({"times"}, {"healthOnly", "timesIf", "condition", "when"}, {("times",)}),
    "ModifyStats": (set(), {"attack", "health", "duration", "canKill", "condition", "when"},
                    {("attack", "health")}),
    "GrantKeyword": ({"keyword"}, {"value", "duration", "condition", "when"}, set()),
    "RemoveGrowth": (set(), {"condition", "when"}, set()),
    "CannotAttack": (set(), {"duration"}, set()),
    "DealDamage": ({"amount"}, {"source", "when"}, set()),
    "Summon": ({"count"}, {"token", "card", "when"}, {("token", "card")}),
    "Copy": ({"to"}, {"from", "type"}, set()),
    "Discover": ({"pool"}, {"choose"}, set()),
    "RefreshShop": (set(), {"free", "extra"}, set()),
    "AddShopSlot": ({"era", "count"}, set(), set()),
    "GainMinion": ({"level", "to"}, {"from"}, set()),
}
CONDITION_RE = re.compile(
    r"^count:(friendly|enemy|both):[a-z]+=[^:]+:(>=|<=|==|>|<)\d+$"
)
TIMES_IF_RE = re.compile(r"^(人类|机械|生物|植物|病毒|学者):\d+$")
EVENT_TRIGGERS = {
    "OnEnemyDeath", "OnFriendlyDeath", "OnAllySummon", "OnAllyGrowth",
    "OnAllyEvolve", "OnDealDamage", "OnCombatEnd", "OnUse",
}
ACHIEVEMENT_FIELDS = ["id", "name", "event", "condition", "count", "reward", "affects_power"]
RANGE = {1: (1, 2, 2, 4), 2: (2, 3, 3, 5), 3: (3, 5, 4, 7),
         4: (4, 7, 6, 9), 5: (6, 9, 8, 12), 6: (8, 12, 10, 16),
         7: (10, 15, 14, 20)}
CAP = {1: 8, 2: 10, 3: 13, 4: 17, 5: 22, 6: 26, 7: 33}
KW_UNLOCK = {"繁衍": 1, "嘲讽": 2, "连击": 3, "最优攻击": 3, "传染": 3, "复生": 4, "护盾": 5}
EXPECT_ERA = {"史前": 27, "原始": 26, "古典": 14, "封建": 29, "工业": 18, "现代": 38, "未来": 32}
EXPECT_TRIBE = {"学者": 41, "生物": 30, "人类": 37, "机械": 34, "植物": 21, "病毒": 21}
EXPECT_LEVEL = {1: 22, 2: 37, 3: 37, 4: 30, 5: 30, 6: 16, 7: 12}
SKILL_FIELDS = ["id", "text", "effects"]
MINION_FIELDS = ["id", "name", "card_type", "tribe", "subtype", "level", "era",
                 "attack", "health", "is_scholar"]
EFFECT_FIELDS = ["id", "name", "card_type", "level", "tribe", "cost", "match_text"]
HERO_CARD_FIELDS = ["hero_skill", "hero_skill_cost", "hero_skill_type",
                    "hero_skill_uses_per_turn", "hero_skill_trigger"]

errors = []


def fail(msg):
    errors.append(msg)


def parse_front_matter(path):
    text = path.read_text(encoding="utf-8")
    if not text.startswith("---\n"):
        raise ValueError(f"{path.name}: 缺少 front matter")
    block = text.split("---\n", 2)[1]
    data = {}
    for line in block.splitlines():
        if not line.strip():
            continue
        if ":" not in line:
            raise ValueError(f"{path.name}: 无法解析行 {line!r}")
        key, raw = line.split(":", 1)
        raw = raw.strip()
        if raw.startswith("[") and raw.endswith("]"):
            inner = raw[1:-1].strip()
            data[key.strip()] = [x.strip() for x in inner.split(",") if x.strip()]
        elif raw == '""':
            data[key.strip()] = ""
        elif raw in ("true", "false"):
            data[key.strip()] = raw == "true"
        elif re.fullmatch(r"-?\d+", raw):
            data[key.strip()] = int(raw)
        else:
            data[key.strip()] = raw
    return data


def check_filters(segments, chunk):
    """筛选写作 key=value；返回错误列表。"""
    errors = []
    for segment in segments:
        if not segment:
            continue
        if "=" not in segment:
            errors.append(f"筛选缺少 =: {segment!r}（{chunk}）")
            continue
        key, val = segment.split("=", 1)
        key, val = key.strip(), val.strip()
        if key not in FILTER_KEYS:
            errors.append(f"未知筛选键 {key!r}（{chunk}）")
        elif key == "tribe" and val not in TRIBES:
            errors.append(f"tribe 取值非法 {val!r}（{chunk}）")
        elif key == "type" and val not in CARD_TYPES:
            errors.append(f"type 取值非法 {val!r}（{chunk}）")
        elif key == "other" and val not in ("true", "false"):
            errors.append(f"other 取值非法 {val!r}（{chunk}）")
        elif not val:
            errors.append(f"筛选 {key!r} 缺少取值（{chunk}）")
    return errors


def check_selector(selector, chunk):
    """选择器写作 self / target / <阵营>:<选取>[:<筛选>...]。"""
    if selector in ("self", "target"):
        return []
    segments = selector.split(":")
    errors = []
    if segments[0] not in SELECTOR_SIDES:
        errors.append(f"选择器阵营非法 {selector!r}（{chunk}）")
    if len(segments) < 2 or not segments[1]:
        errors.append(f"选择器缺少选取 {selector!r}（{chunk}）")
    elif segments[1] not in SELECTOR_PICKS:
        errors.append(f"选择器选取非法 {segments[1]!r}（{chunk}）")
    errors.extend(check_filters(segments[2:], chunk))
    return errors


def check_params(op, params, chunk):
    """按原语规格校验参数键、必填项与取值格式。"""
    spec = OP_SPEC.get(op)
    if spec is None:
        return []
    required, optional, one_of = spec
    errors = []
    unknown = set(params) - required - optional
    if unknown:
        errors.append(f"参数 {sorted(unknown)} 不属于原语 {op}（{chunk}）")
    missing = required - set(params)
    if missing:
        errors.append(f"原语 {op} 缺少必填参数 {sorted(missing)}（{chunk}）")
    for group in one_of:
        if set(group) & set(params):
            break
    else:
        if one_of:
            errors.append(f"原语 {op} 缺少 {[sorted(g) for g in one_of]} 之一（{chunk}）")
    for key, val in params.items():
        if key in NUMERIC_KEYS and not re.fullmatch(r"-?\d+", val):
            errors.append(f"参数 {key} 必须为整数，实为 {val!r}（{chunk}）")
        elif key == "level" and val not in ("current",) and not re.fullmatch(r"\d+", val):
            errors.append(f"参数 level 非法 {val!r}（{chunk}）")
        elif key == "duration" and val not in DURATIONS:
            errors.append(f"duration 非法 {val!r}（{chunk}）")
        elif key in ("to", "from") and val not in ("hand", "board", "shop"):
            errors.append(f"参数 {key} 非法 {val!r}（{chunk}）")
        elif key == "type" and val not in CARD_TYPES:
            errors.append(f"参数 type 非法 {val!r}（{chunk}）")
        elif key == "keyword" and val.split()[0] not in KEYWORD_BASES:
            errors.append(f"keyword 非法 {val!r}（{chunk}）")
        elif key == "condition" and not CONDITION_RE.fullmatch(val):
            errors.append(f"condition 格式非法 {val!r}（{chunk}）")
        elif key == "timesIf" and not TIMES_IF_RE.fullmatch(val):
            errors.append(f"timesIf 格式非法 {val!r}（{chunk}）")
        elif key == "when" and (
            "=" not in val or val.split("=", 1)[0] not in FILTER_KEYS
        ):
            errors.append(f"when 格式非法 {val!r}（{chunk}）")
    return errors


def parse_effects(value):
    """解析单个技能的 effects 字段，返回 (effects, errors)。"""
    if isinstance(value, list):
        if not value:
            return [], []
        return [], [f"effects 不能写成非空列表 {value!r}"]
    raw = str(value).strip()
    if raw == "[]":
        return [], []
    effects, errors = [], []
    for chunk in raw.split(";"):
        chunk = chunk.strip()
        if not chunk:
            errors.append("effects 含空效果段")
            continue
        parts = chunk.split("|")
        if len(parts) not in (3, 4):
            errors.append(f"effects 段数非法（需 3–4 段）: {chunk!r}")
            continue
        trigger, op, selector = (p.strip() for p in parts[:3])
        params = {}
        if len(parts) == 4:
            for pair in parts[3].split(","):
                pair = pair.strip()
                if not pair:
                    continue
                if "=" not in pair:
                    errors.append(f"参数缺少 =: {pair!r}（{chunk}）")
                    continue
                key, val = pair.split("=", 1)
                key = key.strip()
                if key in params:
                    errors.append(f"参数重复 {key!r}（{chunk}）")
                params[key] = val.strip()
        if trigger not in EFFECT_TRIGGERS:
            errors.append(f"未知触发 {trigger!r}（{chunk}）")
        if op not in EFFECT_OPS:
            errors.append(f"未知原语 {op!r}（{chunk}）")
        errors.extend(check_selector(selector, chunk))
        errors.extend(check_params(op, params, chunk))
        if trigger in ("Aura", "ConditionalAura") and op not in AURA_OPS:
            errors.append(f"光环触发只能用 {sorted(AURA_OPS)}，实为 {op}（{chunk}）")
        effects.append({"trigger": trigger, "op": op,
                        "selector": selector, "params": params})
    return ([], errors) if errors else (effects, errors)


def allowed_triggers(text):
    """按技能文本的固定前缀推出允许的触发时机；无法判定时返回 None。"""
    text = text.strip()
    if text.startswith("战吼"):
        return {"OnPlay", "OnCombatStart"}
    if text.startswith("亡语"):
        return {"OnDeath"}
    if text.startswith("战斗开始时"):
        return {"OnCombatStart"}
    if text.startswith("回合开始时"):
        return {"OnTurnStart"}
    if text.startswith("每回合首次刷新"):
        return {"OnFirstRefresh"}
    if text.startswith("每当"):
        return EVENT_TRIGGERS
    if text.startswith("你的") and "随从获得" in text and "直到回合结束" not in text:
        return {"Aura"}
    if text.startswith("若你控制"):
        return {"ConditionalAura"}
    return None


def check_text_trigger(text, effects_raw):
    """卡面文本前缀必须与 effects 的触发时机一致。"""
    allowed = allowed_triggers(text)
    if allowed is None:
        return []
    effects, errors = parse_effects(effects_raw)
    if errors:
        return []
    if not effects:
        return [f"文本含触发语义，但 effects 为 []: {text!r}"]
    bad = [e["trigger"] for e in effects if e["trigger"] not in allowed]
    if bad:
        return [f"触发时机 {bad} 与文本 {text!r} 不一致（允许 {sorted(allowed)}）"]
    return []


def load(kind, required):
    docs = []
    for path in sorted((ROOT / kind).glob("*.md")):
        if path.name == "README.md":
            continue
        try:
            doc = parse_front_matter(path)
        except ValueError as exc:
            fail(str(exc))
            continue
        doc["_file"] = path.name
        missing = [k for k in required if k not in doc]
        if missing:
            fail(f"{path.name}: 缺字段 {missing}")
            continue
        if path.stem != doc["id"]:
            fail(f"{path.name}: 文件名与 id {doc['id']!r} 不一致")
        if not ID_RE.fullmatch(str(doc["id"])):
            fail(f"{path.name}: id 格式非法")
        docs.append(doc)
    return docs


def body_text(path):
    text = path.read_text(encoding="utf-8")
    marker = "## 效果"
    if marker not in text:
        return "无"
    rest = text.split(marker, 1)[1]
    next_heading = re.search(r"(?m)^## ", rest)
    if next_heading:
        rest = rest[:next_heading.start()]
    return rest.strip()


def section_text(text, heading):
    """返回指定「## <heading>」小节正文；无小节返回 None，空小节返回空串。"""
    match = re.search(rf"(?m)^## {re.escape(heading)}\s*$", text)
    if not match:
        return None
    rest = text[match.end():]
    next_heading = re.search(r"(?m)^## ", rest)
    if next_heading:
        rest = rest[:next_heading.start()]
    return rest.strip()


def source_section(text):
    """返回「## 出处」小节正文；无小节返回 None，空小节返回空串。"""
    return section_text(text, "出处")


def parse_attr_table(path):
    text = path.read_text(encoding="utf-8")
    match = re.search(r"## 属性\n\n((?:\|.*\n)+)", text)
    if not match:
        return None
    rows = {}
    for line in match.group(1).strip().splitlines():
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) != 2 or cells[0] == "属性" or set(cells[0]) <= {"-"}:
            continue
        rows[cells[0]] = cells[1]
    return rows


def card_skills(doc):
    value = doc.get("skills")
    return value if isinstance(value, list) else []


def rendered_text(doc, skill_map):
    """按 skills 顺序拼接技能 text；引用缺失时返回 None 交由引用校验报告。"""
    parts = []
    for sid in card_skills(doc):
        skill = skill_map.get(sid)
        if skill is None:
            return None
        parts.append(skill["text"])
    return "；".join(parts) if parts else "无"


def derived_keywords(doc, skill_map):
    keywords = []
    seen = set()
    for sid in card_skills(doc):
        skill = skill_map.get(sid)
        if skill is None:
            continue
        keyword = skill.get("keyword")
        if keyword and keyword not in seen:
            seen.add(keyword)
            keywords.append(keyword)
    return keywords


def hero_skill_text(skill):
    return f"{skill['name']}：{skill['text']}"


def expected_card_attrs(doc, skill_map):
    skills = card_skills(doc)
    skill_value = "；".join(skills) if skills else "无"
    keyword_list = derived_keywords(doc, skill_map)
    keyword_value = "；".join(keyword_list) if keyword_list else "无"
    if doc["card_type"] == "随从":
        attrs = {
            "id": doc["id"], "名称": doc["name"], "类型": doc["card_type"],
            "流派": doc["tribe"], "子类": doc["subtype"], "星级": str(doc["level"]),
            "时代": doc["era"], "攻击力": str(doc["attack"]),
            "生命值": str(doc["health"]), "关键词": keyword_value,
            "技能": skill_value, "学者": "是" if doc["is_scholar"] else "否",
            "英雄资格": "是" if doc.get("is_hero") else "否",
        }
        if doc.get("is_hero"):
            attrs.update({
                "英雄技能": doc["hero_skill"],
                "英雄技能费用": str(doc["hero_skill_cost"]),
                "英雄技能类型": doc["hero_skill_type"],
                "每回合次数": str(doc["hero_skill_uses_per_turn"]),
                "触发时机": doc["hero_skill_trigger"],
            })
        return attrs
    return {
        "id": doc["id"], "名称": doc["name"], "类型": doc["card_type"],
        "星级": str(doc["level"]), "流派": doc["tribe"],
        "技能": skill_value, "费用": str(doc["cost"]),
        "比赛模式替换": doc["match_text"],
    }


def expected_skill_attrs(skill):
    return {
        "id": skill["id"],
        "关键词": skill.get("keyword") or "无",
        "比赛模式替换": skill.get("match_text") or "不适用",
    }


def check_skills():
    skills = load("skills", SKILL_FIELDS)
    if len(skills) != 118:
        fail(f"技能数量 {len(skills)}，期望 118")
    dup = [v for v, n in collections.Counter(s["id"] for s in skills).items() if n > 1]
    if dup:
        fail(f"技能 id 重复: {dup}")
    for skill in skills:
        name = skill["_file"]
        if not str(skill["text"]).strip():
            fail(f"{name}: text 为空")
        keyword = skill.get("keyword")
        if keyword and not SKILL_KEYWORD_RE.fullmatch(str(keyword)):
            fail(f"{name}: 关键词非法 {keyword!r}")
        raw_effects = skill.get("effects")
        parsed_effects, effect_errors = parse_effects(raw_effects)
        for message in effect_errors:
            fail(f"{name}: {message}")
        for message in check_text_trigger(skill["text"], raw_effects):
            fail(f"{name}: {message}")
        path = ROOT / "skills" / name
        table = parse_attr_table(path)
        if table is None:
            fail(f"{name}: 缺少「## 属性」表")
            continue
        expected = expected_skill_attrs(skill)
        for key, value in expected.items():
            if table.get(key) != value:
                fail(f"{name}: 属性表 {key}={table.get(key)!r}，front matter 为 {value!r}")
        extra = set(table) - set(expected)
        if extra:
            fail(f"{name}: 属性表多出字段 {sorted(extra)}")
        if body_text(path) != skill["text"]:
            fail(f"{name}: 「## 效果」与 text 不一致")
    return skills


def check_skill_context(skill_map, cards, effects):
    hero_refs = {c["hero_skill"] for c in cards
                 if c.get("is_hero") is True and c.get("hero_skill") in skill_map}
    effect_refs = {sid for c in effects for sid in card_skills(c)}
    for sid in sorted(hero_refs):
        skill = skill_map[sid]
        if not skill.get("name"):
            fail(f"{skill['_file']}: 被 hero_skill 引用的技能缺少 name")
        if not skill.get("match_text"):
            fail(f"{skill['_file']}: 被 hero_skill 引用的技能缺少 match_text")
    for sid in sorted(effect_refs):
        skill = skill_map.get(sid)
        if skill is not None and not skill.get("match_text"):
            fail(f"{skill['_file']}: 被效果牌引用的技能缺少 match_text")


def check_attr_tables(raw, skill_map):
    for doc in raw:
        path = ROOT / "cards" / doc["_file"]
        table = parse_attr_table(path)
        if table is None:
            fail(f"{doc['_file']}: 缺少「## 属性」表")
            continue
        expected = expected_card_attrs(doc, skill_map)
        for key, value in expected.items():
            if table.get(key) != value:
                fail(f"{doc['_file']}: 属性表 {key}={table.get(key)!r}，front matter 为 {value!r}")
        extra = set(table) - set(expected)
        if extra:
            fail(f"{doc['_file']}: 属性表多出字段 {sorted(extra)}")


def check_card_rendering(raw, skill_map):
    for doc in raw:
        expected = rendered_text(doc, skill_map)
        if expected is None:
            continue
        actual = body_text(ROOT / "cards" / doc["_file"])
        if actual != expected:
            fail(f"{doc['_file']}: 「## 效果」与技能文本拼接不一致：{actual!r} != {expected!r}")


def check_card_sources(raw):
    for doc in raw:
        text = (ROOT / "cards" / doc["_file"]).read_text(encoding="utf-8")
        section = source_section(text)
        if section is None or not section:
            fail(f"{doc['_file']}: 缺少「## 出处」或内容为空")


def keyword_cost(card, skill_map):
    total = 0
    summon_like = any(
        marker in sid
        for sid in card_skills(card)
        for marker in ("summon", "copy", "discover")
    )
    for k in derived_keywords(card, skill_map):
        if k == "嘲讽":
            total += .5
        elif k == "护盾":
            total += 2
        elif k in ("连击", "复生", "进化"):
            total += 3
        elif k in ("传染", "最优攻击", "播种"):
            total += 2
        elif k.startswith("繁衍"):
            total += 2
        elif k.startswith("装配"):
            total += 2 + 2 * (int(k.split()[1]) - 1)
        elif k in ("战吼", "亡语"):
            total += 4 if summon_like else 2
    return total


def check_keyword_unlock(minions, skill_map):
    actual = {}
    for c in minions:
        for k in derived_keywords(c, skill_map):
            base = k.split()[0]
            if base in KW_UNLOCK:
                actual[base] = min(actual.get(base, 99), c["level"])
    for keyword, era_level in KW_UNLOCK.items():
        if actual.get(keyword) != era_level:
            fail(f"关键词 {keyword} 的最小星级为 {actual.get(keyword)}，期望 {era_level}")


def check_cards(skill_map):
    raw = load("cards", ["id", "name", "card_type", "level", "tribe"])
    for doc in raw:
        if doc["card_type"] not in CARD_TYPES:
            fail(f"{doc['_file']}: card_type 非法 {doc['card_type']!r}")
        raw_skills = doc.get("skills")
        if raw_skills is not None and not isinstance(raw_skills, list):
            fail(f"{doc['_file']}: skills 必须是列表，实为 {raw_skills!r}")
        if "keywords" in doc:
            fail(f"{doc['_file']}: 残留旧字段 keywords，应改用 skills")
        for sid in card_skills(doc):
            if sid not in skill_map:
                fail(f"{doc['_file']}: 引用的技能不存在 {sid!r}")
    minions = [c for c in raw if c["card_type"] == "随从"]
    effects = [c for c in raw if c["card_type"] == "效果"]

    skillless = [c for c in minions if not card_skills(c)]
    if len(skillless) != 15:
        fail(f"无技能随从 {len(skillless)} 张，期望 15 张（skills 缺失或为空）")
    for c in skillless:
        if body_text(ROOT / "cards" / c["_file"]) != "无":
            fail(f"{c['_file']}: 无技能随从的「## 效果」必须为「无」")

    for c in minions:
        missing = [k for k in MINION_FIELDS if k not in c]
        if missing:
            fail(f"{c['_file']}: 随从缺字段 {missing}")
            continue
        name = c["name"]
        if c["tribe"] not in TRIBES:
            fail(f"{name}: 流派非法 {c['tribe']!r}")
        if c["era"] not in ERAS:
            fail(f"{name}: 时代非法 {c['era']!r}")
        if c["level"] not in RANGE:
            fail(f"{name}: 星级非法 {c['level']!r}")
            continue
        if (c["tribe"] == "学者") != c["is_scholar"]:
            fail(f"{name}: 流派与 is_scholar 不一致")
        if c["is_scholar"] and c["attack"] != 0:
            fail(f"{name}: 学者攻击力必须为 0")
        if c["is_scholar"]:
            expected_health = RANGE[c["level"]][3] * 1.3
            if abs(c["health"] - expected_health) > 1:
                fail(
                    f"{name}: 学者生命值 {c['health']} 偏离 {c['level']} 星生命上限×1.3"
                    f"（{expected_health:g}）超过 1 点"
                )
            continue
        lo, hi, lh, hh = RANGE[c["level"]]
        if not (lo <= c["attack"] <= hi and lh <= c["health"] <= hh):
            fail(f"{name}: 数值 {c['attack']}/{c['health']} 越出 {c['level']} 星区间")
        if c["attack"] + c["health"] + keyword_cost(c, skill_map) > CAP[c["level"]]:
            fail(f"{name}: 超模")

    for c in minions:
        for k in derived_keywords(c, skill_map):
            if not KEYWORD_RE.fullmatch(k):
                fail(f"{c['name']}: 派生关键词非法 {k!r}")
            base = k.split()[0]
            if base in TRIBE_KEYWORD and c["tribe"] != TRIBE_KEYWORD[base]:
                fail(f"{c['name']}: 专属词 {k!r} 与流派 {c['tribe']} 越界")

    for c in effects:
        missing = [k for k in EFFECT_FIELDS if k not in c]
        if missing:
            fail(f"{c['_file']}: 效果牌缺字段 {missing}")
            continue
        name = c["name"]
        if not card_skills(c):
            fail(f"{c['_file']}: 效果牌缺技能，skills 不能为空")
        if "attack" in c or "health" in c:
            fail(f"{name}: 效果牌不得有 attack/health")
        if c["tribe"] not in EFFECT_TRIBES:
            fail(f"{name}: 效果牌流派非法 {c['tribe']!r}")
        if c["level"] not in RANGE:
            fail(f"{name}: 星级非法 {c['level']!r}")
        if not 0 <= c["cost"] <= 4:
            fail(f"{name}: 费用越界 {c['cost']}")

    counts = collections.Counter(c["card_type"] for c in raw)
    if counts != {"随从": 184, "效果": 36}:
        fail(f"卡牌类型分布不符: {dict(counts)}")
    ids = [c["id"] for c in raw]
    dup = [v for v, n in collections.Counter(ids).items() if n > 1]
    if dup:
        fail(f"卡牌 id 重复: {dup}")
    for label, group in (("随从", minions), ("效果", effects)):
        dup = [v for v, n in collections.Counter(c["name"] for c in group).items() if n > 1]
        if dup:
            fail(f"{label}名称重复: {dup}")

    for label, expected, field in (("时代", EXPECT_ERA, "era"),
                                   ("流派", EXPECT_TRIBE, "tribe"),
                                   ("星级", EXPECT_LEVEL, "level")):
        actual = collections.Counter(c[field] for c in minions)
        if dict(actual) != expected:
            fail(f"随从{label}分布不符: {dict(actual)}")
    return raw, minions, effects


def check_hero_cards(minions, effects, skill_map):
    hero_cards = [c for c in minions if c.get("is_hero") is True]
    if len(hero_cards) != 13:
        fail(f"英雄卡 {len(hero_cards)} 张，期望 13")
    refs = []
    for c in hero_cards:
        missing = [k for k in HERO_CARD_FIELDS if k not in c]
        if missing:
            fail(f"{c['_file']}: 英雄卡缺字段 {missing}")
            continue
        name = c["name"]
        skill = skill_map.get(c["hero_skill"])
        if skill is None:
            fail(f"{name}: hero_skill 指向不存在的技能 {c['hero_skill']!r}")
        else:
            if not skill.get("name") or not skill.get("match_text"):
                fail(f"{name}: 英雄技能 {c['hero_skill']!r} 缺少 name 或 match_text")
        if c["hero_skill_type"] not in ("主动", "被动"):
            fail(f"{name}: 英雄技能类型非法 {c['hero_skill_type']!r}")
        if not 0 <= c["hero_skill_cost"] <= 2:
            fail(f"{name}: 英雄技能费用越界 {c['hero_skill_cost']}")
        if c["hero_skill_type"] == "被动" and c["hero_skill_cost"] != 0:
            fail(f"{name}: 被动英雄技能费用必须为 0")
        if c["hero_skill_uses_per_turn"] not in (1, 2):
            fail(f"{name}: 每回合次数非法 {c['hero_skill_uses_per_turn']!r}")
        if c["hero_skill_trigger"] not in SKILL_TRIGGERS:
            fail(f"{name}: 英雄技能触发时机非法 {c['hero_skill_trigger']!r}")
        if c["hero_skill_type"] == "主动" and c["hero_skill_trigger"] != "主动使用":
            fail(f"{name}: 主动英雄技能的触发时机必须为「主动使用」")
        if c["hero_skill_type"] == "被动" and c["hero_skill_trigger"] == "主动使用":
            fail(f"{name}: 被动英雄技能不能使用「主动使用」触发")
        refs.append(c["hero_skill"])
    dup = [v for v, n in collections.Counter(refs).items() if n > 1]
    if dup:
        fail(f"hero_skill 重复: {dup}")
    for c in minions:
        if c.get("is_hero") is True:
            continue
        extra = [k for k in HERO_CARD_FIELDS + ["is_hero"] if k in c]
        if extra:
            fail(f"{c['_file']}: 非英雄随从不得有英雄字段 {extra}")
    for c in effects:
        extra = [k for k in HERO_CARD_FIELDS + ["is_hero"] if k in c]
        if extra:
            fail(f"{c['_file']}: 效果牌不得有英雄字段 {extra}")
    return hero_cards


def check_hero_skill_rendering(cards, skill_map):
    for doc in cards:
        section = section_text(
            (ROOT / "cards" / doc["_file"]).read_text(encoding="utf-8"), "英雄技能"
        )
        if doc.get("is_hero") is True:
            skill = skill_map.get(doc.get("hero_skill"))
            if skill is None:
                continue
            expected = hero_skill_text(skill)
            if section != expected:
                fail(f"{doc['_file']}: 「## 英雄技能」与 hero_skill 渲染不一致："
                     f"{section!r} != {expected!r}")
        elif section is not None:
            fail(f"{doc['_file']}: 非英雄卡不得有「## 英雄技能」小节")


def check_achievements():
    achievements = load("achievements", ACHIEVEMENT_FIELDS)
    if len(achievements) != 6:
        fail(f"成就数量 {len(achievements)}，期望 6")
    for key in ("id", "name"):
        values = [a[key] for a in achievements]
        dup = [v for v, n in collections.Counter(values).items() if n > 1]
        if dup:
            fail(f"成就 {key} 重复: {dup}")
    for a in achievements:
        if a["event"] not in ACHIEVEMENT_EVENTS:
            fail(f"{a['name']}: 成就事件非法 {a['event']!r}")
        if not isinstance(a["count"], int) or a["count"] < 1:
            fail(f"{a['name']}: 成就次数必须为正整数")
        if not isinstance(a["affects_power"], bool):
            fail(f"{a['name']}: affects_power 必须为布尔")
        if not a["condition"] or not a["reward"]:
            fail(f"{a['name']}: condition 或 reward 为空")
    return achievements


def check_auras(minions):
    for c in minions:
        text = body_text(ROOT / "cards" / c["_file"])
        for part in re.split(r"[；;]", text):
            part = part.strip()
            if not (part.startswith("你的") and "随从获得" in part):
                continue
            match = AURA_RE.fullmatch(part)
            if not match or not AURA_VALUE_RE.fullmatch(match.group(3)):
                fail(f"{c['_file']}: 光环句式不合法 {part!r}")


def check_enemy_selectors(raw):
    picks = ("随机", "最左侧", "攻击力最高", "全体")
    for doc in raw:
        text = body_text(ROOT / "cards" / doc["_file"])
        for phrase in ("对一个敌方随从", "使一个敌方随从"):
            if phrase in text and not any(pick in text for pick in picks):
                fail(f"{doc['_file']}: 单体敌方目标缺少选择器（随机/最左侧/攻击力最高/全体）")


def main():
    skills = check_skills()
    skill_map = {s["id"]: s for s in skills}
    raw, minions, effects = check_cards(skill_map)
    check_keyword_unlock(minions, skill_map)
    hero_cards = check_hero_cards(minions, effects, skill_map)
    check_skill_context(skill_map, raw, effects)
    achievements = check_achievements()
    check_attr_tables(raw, skill_map)
    check_card_rendering(raw, skill_map)
    check_hero_skill_rendering(raw, skill_map)
    check_card_sources(raw)
    check_enemy_selectors(raw)
    check_auras(minions)
    if errors:
        print(f"违规 {len(errors)}")
        for item in errors:
            print(" -", item)
        return 1
    print("时代", dict(collections.Counter(c["era"] for c in minions)))
    print("流派", dict(collections.Counter(c["tribe"] for c in minions)))
    print("星级", dict(collections.Counter(c["level"] for c in minions)))
    print("关键词", dict(sorted(collections.Counter(
        k.split()[0] for c in minions for k in derived_keywords(c, skill_map)).items())))
    print("技能引用", dict(sorted(collections.Counter(
        sid for c in raw for sid in card_skills(c)).items())))
    print(f"随从 {len(minions)} 张、效果牌 {len(effects)} 张、技能 {len(skills)} 个、"
          f"英雄卡 {len(hero_cards)} 张、成就 {len(achievements)} 个，校验通过")
    return 0


if __name__ == "__main__":
    sys.exit(main())
