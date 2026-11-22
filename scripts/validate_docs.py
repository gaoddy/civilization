#!/usr/bin/env python3
"""校验 docs/versions/v0.1/product/content 下的技能与卡牌实体文档，
以及 docs/versions/v0.2/product/content 下的成就实体文档。

只使用标准库；front matter 仅支持本项目生成的三种形式：
    key: 标量
    skills: [技能 id, ...]
    skill: 技能 id
卡牌分两类：card_type=随从 必须有攻击力/生命值；card_type=效果 必须没有。
随从与效果牌都通过 skills 引用技能实体；效果文本与关键词由被引用技能派生。
"""

import collections
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import i18n_terms
import skill_templates

ROOT = pathlib.Path(__file__).resolve().parent.parent / "docs" / "versions" / "v0.1" / "product" / "content"
V02_ROOT = pathlib.Path(__file__).resolve().parent.parent / "docs" / "versions" / "v0.2" / "product" / "content"
RULES_FILE = ROOT.parent / "rules" / "keywords-and-mechanics.md"

ID_RE = re.compile(r"[a-z][a-z0-9-]*")
_KEYWORD_NAMES = (
    "battlecry|deathrattle|reborn|taunt|divine_shield|windfury|contagion|"
    "ignore_taunt|sow|evolve|poison|stealth|decompose"
)
KEYWORD_RE = re.compile(rf"^({_KEYWORD_NAMES}|breed \d+|assemble \d+|armor \d+)$")
# 技能上的 keyword 是携带者自身登记的常驻关键词，不接受卡牌层的「无」哨兵值。
SKILL_KEYWORD_RE = re.compile(rf"^({_KEYWORD_NAMES}|breed \d+|assemble \d+|armor \d+)$")
AURA_RE = re.compile(r"^你的(其他)?(人类|人科|机械|生物|动物|植物|真菌|细菌|原生生物|病毒|鱼纲|鸟纲|爬行纲|昆虫纲|头足纲|水螅纲|真菌界|原生生物界)?随从获得 ?(.+)$")
AURA_VALUE_RE = re.compile(
    r"^(?:\+\d+/\+\d+|\+\d+ 攻击力|\+\d+ 生命值|无视嘲讽|嘲讽|护盾|复生|连击)$"
)
TAXONOMY_FILE = ROOT / "taxonomy.md"
# 分类树层级：中文编辑值 → 生成物 rank。
TAXONOMY_RANKS = {"界": "kingdom", "门": "phylum", "纲": "class", "目": "order",
                  "科": "family", "技术分类": "technique", "学科": "discipline"}
# rank 的合法出现位置：根只允许界。
RANK_PARENT_ALLOWED = {
    "kingdom": set(),
    "phylum": {"kingdom"},
    "class": {"phylum"},
    "order": {"phylum", "class"},
    "family": {"kingdom", "phylum", "class", "order", "family"},
    "technique": {"kingdom"},
    "discipline": {"kingdom"},
}
ROOT_RANKS = {"界"}
PLAYSTYLE_IDS = ("mammal", "bird", "reptile", "fish", "insect", "cephalopod", "cnidarian",
                 "plant", "fungus", "bacteria", "protozoa", "virus", "machine")
PLAYSTYLE_VALUES = set(PLAYSTYLE_IDS) | {"neutral"}
# 无技能随从只允许造物六类工具卡。
CRAFT_TOOL_TAXA = {"power-energy", "transport", "weapon-siege",
                   "production-construction", "information-computing", "life-medical"}
CATEGORY_KINDS = ("play", "prey", "host")
CATEGORY_RANK = {"play": 0, "prey": 1, "host": 2}
# 专属关键词及其允许的玩法。evolve 另要求生物卡，在关键词校验里单独处理。
KEYWORD_PLAYSTYLE = {
    "decompose": {"fungus", "bacteria"},
    "assemble": {"machine"},
    "sow": {"plant"},
    "poison": {"reptile", "cephalopod", "cnidarian"},
    "armor": {"reptile", "bacteria", "cnidarian"},
    "stealth": {"cephalopod"},
}
# 新机制原语的玩法归属；原语只能出现在对应玩法的卡牌技能里。
MECHANISM_PLAYSTYLE = {
    "Mimic": {"cephalopod"},
    "Devour": {"protozoa", "cephalopod"},
    "Detonate": {"bacteria", "virus"},
    "Convert": {"bacteria"},
    "Revive": {"cnidarian"},
    "Recall": {"cnidarian"},
    "Salvage": {"machine"},
    "Latent": {"virus"},
}
# 选择器选取的玩法归属。
PICK_PLAYSTYLE = {"lowestAttack": {"bird"}}
BIOLOGICAL_KINGDOMS = {"animalia", "plantae", "fungi", "bacteria", "protozoa", "virus"}
CRAFT_ID = "craft"
DISCIPLINE_ROOTS = {"science", "humanities", "society"}
ERAS = list(i18n_terms.ERAS)
CARD_TYPES = set(i18n_terms.CARD_TYPES)
KEYWORD_BASES = set(i18n_terms.KEYWORD_BASES)
# 卡牌/技能关键词允许 decompose；GrantKeyword 参数仍沿用 i18n_terms 的既有集合。
ALL_KEYWORD_BASES = KEYWORD_BASES | {"decompose"}
CARD_IDS = {path.stem for path in (ROOT / "cards").glob("*.md") if path.name != "README.md"}
ACHIEVEMENT_EVENTS = {"时代升级", "关键词组合", "成长触发", "卡牌使用", "三连合成"}
EFFECT_TRIGGERS = {
    "OnPlay", "OnPlayEffect", "OnUse", "OnActivate", "OnTurnStart",
    "OnFirstRefresh", "OnCombatStart", "OnDealDamage", "OnKill", "OnAllyKill",
    "OnDeath", "OnFriendlyDeath", "OnEnemyDeath", "OnAllySummon", "OnAllyGrowth",
    "OnAllyEvolve", "Aura", "ConditionalAura",
}
EFFECT_OPS = {
    "Growth", "ModifyStats", "GrantKeyword", "RemoveGrowth", "CannotAttack",
    "DealDamage", "Summon", "Copy", "Discover", "RefreshShop", "AddShopSlot",
    "GainMinion", "Decompose", "Mimic", "Devour", "Detonate", "Convert", "Revive",
    "Recall", "Salvage", "Latent", "SevenVoyages", "GrantMinion",
}
SELECTOR_SIDES = {"friendly", "enemy", "both", "self", "target"}
SELECTOR_TARGET_ZONES = {"hand", "shop", "board"}
SELECTOR_PICKS = {"single", "all", "random", "leftmost", "leftmost+adjacent", "topAttack",
                  "lowestAttack", "adjacent"}
FILTER_KEYS = {"taxon", "playstyle", "play", "prey", "host", "subtype", "id", "type",
               "keyword", "other"}
DURATIONS = {"permanent", "combat"}
DECOMPOSE_BASES = {"original", "combat_start", "pre_kill"}
NUMERIC_KEYS = {"times", "count", "amount", "attack", "health", "value",
                "perAttack", "perHealth", "maxCount", "threshold", "turns_max"}
AURA_OPS = {"ModifyStats", "GrantKeyword"}
OP_SPEC = {
    "Growth": ({"times"}, {"healthOnly", "timesIf", "condition", "when"}, {("times",)}),
    "ModifyStats": (set(), {"attack", "health", "duration", "canKill", "condition", "when",
                            "perAttack", "perHealth", "maxCount"},
                    {("attack", "health")}),
    "GrantKeyword": ({"keyword"}, {"value", "duration", "condition", "when"}, set()),
    "RemoveGrowth": (set(), {"condition", "when"}, set()),
    "CannotAttack": (set(), {"duration"}, set()),
    "DealDamage": ({"amount"}, {"when", "canKill"}, set()),
    "Summon": ({"count"}, {"token", "card", "when"}, {("token", "card")}),
    "Copy": ({"to"}, {"from", "type", "state"}, set()),
    "Discover": ({"pool"}, {"choose"}, set()),
    "RefreshShop": (set(), {"free"}, set()),
    "AddShopSlot": ({"era", "count"}, set(), set()),
    "GainMinion": ({"level", "to"}, {"from"}, set()),
    "GrantMinion": ({"level", "to"}, set(), set()),
    "Decompose": ({"basis", "duration"}, {"timesIf", "condition", "when"}, set()),
    "Mimic": ({"duration"}, {"statsOnly"}, set()),
    "Devour": ({"value"}, {"duration", "when"}, set()),
    "Detonate": ({"amount"}, {"threshold", "when"}, set()),
    "Convert": ({"threshold"}, {"token", "card", "count", "when"}, {("token", "card")}),
    "Revive": ({"attack", "health"}, set(), set()),
    "Recall": ({"to"}, set(), set()),
    "Salvage": ({"pool", "count"}, set(), set()),
    "Latent": ({"amount", "count"}, {"threshold", "when"}, set()),
    "SevenVoyages": ({"turns_max"}, set(), set()),
}
# 群聚参数只允许出现在光环的 ModifyStats 上。
CROWD_PARAMS = {"perAttack", "perHealth", "maxCount"}
CONDITION_RE = re.compile(
    r"^count:(friendly|enemy|both):([a-z_]+)=([a-z0-9_-]+):(>=|<=|==|>|<)\d+$"
)
EVENT_TRIGGERS = {
    "OnEnemyDeath", "OnFriendlyDeath", "OnAllySummon", "OnAllyGrowth",
    "OnAllyEvolve", "OnDealDamage", "OnUse", "OnKill", "OnAllyKill",
}
ACHIEVEMENT_FIELDS = ["id", "name", "event", "condition", "count", "reward",
                      "steam_api_name", "affects_power"]
# Steamworks 成就 API 名：ASCII 大写字母开头，后接大写字母、数字或下划线。
STEAM_API_NAME_RE = re.compile(r"^[A-Z][A-Z0-9_]{2,63}$")
RANGE = {1: (1, 2, 2, 4), 2: (2, 3, 3, 5), 3: (3, 5, 4, 7),
         4: (4, 7, 6, 9), 5: (6, 9, 8, 12), 6: (8, 12, 10, 16),
         7: (10, 15, 14, 20)}
CAP = {1: 8, 2: 10, 3: 13, 4: 17, 5: 22, 6: 26, 7: 33}
KW_UNLOCK = {"breed": 1, "taunt": 2, "windfury": 3, "ignore_taunt": 3,
             "contagion": 3, "reborn": 4, "divine_shield": 5,
             "poison": 3, "armor": 2, "stealth": 3}
EXPECT_ERA = {"prehistoric": 86, "primitive": 60, "classical": 45, "feudal": 57,
              "industrial": 41, "modern": 96, "future": 47}
EXPECT_LEVEL = {1: 50, 2: 58, 3: 61, 4: 60, 5: 55, 6: 28, 7: 20}
SKILL_FIELDS = ["id"]
SKILL_OPTIONAL_FIELDS = {"text", "text_golden", "template", "keyword", "cost",
                         "uses_per_turn", "name"}
MINION_FIELDS = ["id", "name", "card_type", "taxon", "subtype", "level", "era",
                 "attack", "health"]
EFFECT_FIELDS = ["id", "name", "card_type", "level", "era", "taxon",
                 "cost"]
HERO_REQUIRED_FIELDS = ["id", "name", "categories", "introduction"]
HERO_OPTIONAL_FIELDS = {"skills"}
HERO_COUNT = 32
LEGACY_HERO_FIELDS = ["hero_skill", "hero_skill_cost", "hero_skill_type",
                      "hero_skill_uses_per_turn", "hero_skill_trigger"]

# 分类上下文，main 在 check_taxonomy 之后写入；单测可直接赋值。
TAXONOMY_NODES = {}
TAXONOMY_CHILDREN = {}
TAXONOMY_ORDER = {}
PLAYSTYLE_NAMES = {}

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
        key = key.strip()
        if key in data:
            # dict 会静默覆盖重复键，掩盖「值写进了上一个键」这类损坏（例如
            # `taxon: skills: []`），必须当作解析错误。
            raise ValueError(f"{path.name}: front matter 键重复 {key!r}")
        raw = raw.strip()
        if raw.startswith("[") and raw.endswith("]"):
            inner = raw[1:-1].strip()
            data[key.strip()] = [x.strip().strip("'\"") for x in inner.split(",") if x.strip()]
        elif raw == '""':
            data[key.strip()] = ""
        elif raw in ("true", "false"):
            data[key.strip()] = raw == "true"
        elif re.fullmatch(r"-?\d+", raw):
            data[key.strip()] = int(raw)
        else:
            data[key.strip()] = raw
    return data


def check_filter_value(key, val, chunk):
    """校验单个筛选键的取值；taxonomy 未加载时只做格式校验。"""
    errors = []
    if key == "taxon":
        if not ID_RE.fullmatch(val):
            errors.append(f"taxon 取值格式非法 {val!r}（{chunk}）")
        elif TAXONOMY_NODES and val not in TAXONOMY_NODES:
            errors.append(f"taxon 取值不存在 {val!r}（{chunk}）")
    elif key == "playstyle":
        if val not in PLAYSTYLE_VALUES:
            errors.append(f"playstyle 取值非法 {val!r}（{chunk}）")
    elif key == "play":
        if val not in PLAYSTYLE_IDS:
            errors.append(f"play 取值非法 {val!r}（{chunk}）")
    elif key in ("prey", "host"):
        if not ID_RE.fullmatch(val):
            errors.append(f"{key} 取值格式非法 {val!r}（{chunk}）")
        elif TAXONOMY_NODES and val not in TAXONOMY_NODES:
            errors.append(f"{key} 取值不存在 {val!r}（{chunk}）")
    elif key == "type":
        if val not in CARD_TYPES:
            errors.append(f"type 取值非法 {val!r}（{chunk}）")
    elif key == "id":
        if val not in CARD_IDS:
            errors.append(f"id 取值非法 {val!r}（{chunk}）")
    elif key == "other":
        if val not in ("true", "false"):
            errors.append(f"{key} 取值非法 {val!r}（{chunk}）")
    elif key == "subtype":
        if not ID_RE.fullmatch(val):
            errors.append(f"subtype 取值格式非法 {val!r}（{chunk}）")
    elif key == "keyword":
        if val.split()[0] not in ALL_KEYWORD_BASES:
            errors.append(f"keyword 取值非法 {val!r}（{chunk}）")
    return errors


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
        elif not val:
            errors.append(f"筛选 {key!r} 缺少取值（{chunk}）")
        else:
            errors.extend(check_filter_value(key, val, chunk))
    return errors


def times_if_errors(value, chunk=""):
    """timesIf 格式：<键>:<值>:<次数>。"""
    parts = str(value).split(":")
    if len(parts) == 3 and parts[0] in FILTER_KEYS and parts[2].isdigit():
        return check_filter_value(parts[0], parts[1], chunk)
    return [f"timesIf 格式非法 {value!r}（{chunk}）"]


def check_selector(selector, chunk):
    """选择器写作 self / target[:<区域>[:<筛选>...]] / <阵营>:<选取>[:<筛选>...]。"""
    if selector in ("self", "target"):
        return []
    if selector.startswith("target:"):
        segments = selector.split(":")
        errors = []
        if len(segments) < 2 or segments[1] not in SELECTOR_TARGET_ZONES:
            errors.append(f"目标区域非法 {selector!r}（{chunk}）")
        errors.extend(check_filters(segments[2:], chunk))
        return errors
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
        elif key == "level" and val not in ("current", "next") and not re.fullmatch(r"\d+", val):
            errors.append(f"参数 level 非法 {val!r}（{chunk}）")
        elif key == "duration" and val not in DURATIONS:
            errors.append(f"duration 非法 {val!r}（{chunk}）")
        elif key in ("to", "from") and val not in ("hand", "board", "shop"):
            errors.append(f"参数 {key} 非法 {val!r}（{chunk}）")
        elif key == "type" and val not in CARD_TYPES:
            errors.append(f"参数 type 非法 {val!r}（{chunk}）")
        elif key in ("canKill", "healthOnly", "free") and val not in ("true", "false"):
            errors.append(f"参数 {key} 必须为布尔值，实为 {val!r}（{chunk}）")
        elif key == "keyword" and val.split()[0] not in KEYWORD_BASES:
            errors.append(f"keyword 非法 {val!r}（{chunk}）")
        elif key == "basis" and val not in DECOMPOSE_BASES:
            errors.append(f"basis 非法 {val!r}（{chunk}）")
        elif key == "condition":
            match = CONDITION_RE.fullmatch(val)
            if not match:
                errors.append(f"condition 格式非法 {val!r}（{chunk}）")
            elif match.group(2) not in FILTER_KEYS:
                errors.append(f"condition 筛选键非法 {match.group(2)!r}（{chunk}）")
            else:
                errors.extend(check_filter_value(match.group(2), match.group(3), chunk))
        elif key == "timesIf":
            errors.extend(times_if_errors(val, chunk))
        elif key == "when":
            if "=" not in val or val.split("=", 1)[0] not in FILTER_KEYS:
                errors.append(f"when 格式非法 {val!r}（{chunk}）")
            else:
                when_key, when_val = val.split("=", 1)
                errors.extend(check_filter_value(when_key, when_val, chunk))
    if op == "GrantMinion" and params.get("to") != "hand":
        errors.append(f"GrantMinion 的 to 只接受 hand，实为 {params.get('to')!r}（{chunk}）")
    if params.get("level") == "next" and op != "GrantMinion":
        errors.append(f"level=next 只用于 GrantMinion（{chunk}）")
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
        if selector not in ("self", "target") and not selector.startswith("target:"):
            parts = selector.split(":")
            if len(parts) > 1 and parts[1] == "adjacent" \
                    and trigger not in ("Aura", "ConditionalAura"):
                errors.append(f"相邻选择器只能用于光环，实为 {trigger}（{chunk}）")
            if len(parts) > 1 and parts[1] == "adjacent" and parts[0] != "friendly":
                errors.append(f"相邻选择器必须以 friendly: 开头，实为 {selector!r}（{chunk}）")
        if CROWD_PARAMS & set(params) and trigger not in ("Aura", "ConditionalAura"):
            errors.append(f"群聚参数 {sorted(CROWD_PARAMS & set(params))} 只能用于光环（{chunk}）")
        if trigger in ("Aura", "ConditionalAura") and op not in AURA_OPS:
            errors.append(f"光环触发只能用 {sorted(AURA_OPS)}，实为 {op}（{chunk}）")
        effects.append({"trigger": trigger, "op": op,
                        "selector": selector, "params": params})
    return ([], errors) if errors else (effects, errors)


def allowed_triggers(text):
    """按技能文本的固定前缀推出允许的触发时机；无法判定时返回 None。"""
    text = text.strip()
    if text.startswith("分解："):
        return {"OnKill", "OnFriendlyDeath", "OnAllyKill"}
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


def _taxonomy_table_rows(text, heading):
    """返回指定小节的表格数据行；小节以同数量 `#` 的下一个标题为界。"""
    match = re.search(rf"(?m)^(##+) {re.escape(heading)}\s*$", text)
    if not match:
        return None
    marks = match.group(1)
    rest = text[match.end():]
    next_heading = re.search(rf"(?m)^{marks} ", rest)
    if next_heading:
        rest = rest[:next_heading.start()]
    rows = []
    for line in rest.splitlines():
        line = line.strip()
        if not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if cells[0] == "id" or set(cells[0]) <= {"-"}:
            continue
        rows.append(cells)
    return rows


def parse_taxonomy(path=TAXONOMY_FILE):
    """解析 taxonomy.md，返回 (nodes, playstyles)。

    nodes 为 {id: {parent, rank, name}} 顺序字典；playstyles 为 [(id, 名称), ...]。
    """
    text = path.read_text(encoding="utf-8")
    rows = _taxonomy_table_rows(text, "1. 科学分类树")
    if rows is None:
        fail(f"{path.name}: 缺少「## 1. 科学分类树」小节")
        return None, None
    nodes = collections.OrderedDict()
    for cells in rows:
        if len(cells) != 5:
            fail(f"{path.name}: 分类树行列数 {len(cells)}，期望 5：{cells}")
            continue
        node_id, name, parent, rank, _ = cells
        nodes[node_id] = {"parent": parent or None, "rank": rank, "name": name}
    playstyle_rows = _taxonomy_table_rows(text, "2. 玩法")
    if playstyle_rows is None:
        fail(f"{path.name}: 缺少「## 2. 玩法」小节")
        return nodes, None
    playstyles = []
    for cells in playstyle_rows:
        if len(cells) != 4:
            fail(f"{path.name}: 玩法行列数 {len(cells)}，期望 4：{cells}")
            continue
        playstyles.append((cells[0], cells[1]))
    return nodes, playstyles


def _root_of(nodes, node_id):
    """沿父链上溯到根；父链断或成环时返回 None。"""
    seen = set()
    current = node_id
    while True:
        node = nodes.get(current)
        if node is None:
            return None
        if node["parent"] is None:
            return current
        if current in seen:
            return None
        seen.add(current)
        current = node["parent"]


def check_taxonomy():
    """校验分类树与玩法表；返回 (nodes, children, playstyles, names)，失败时对应项为 None。"""
    nodes, playstyles = parse_taxonomy()
    if nodes is None:
        return None, None, None, {}

    children = collections.defaultdict(list)
    for node_id, node in nodes.items():
        parent = node["parent"]
        if parent is not None:
            children[parent].append(node_id)

    for node_id, node in nodes.items():
        if not ID_RE.fullmatch(node_id):
            fail(f"分类树节点 {node_id!r}: id 格式非法（需匹配 [a-z][a-z0-9-]*）")
        if node["rank"] not in TAXONOMY_RANKS:
            fail(f"分类树节点 {node_id!r}: 层级 {node['rank']!r} 不在 "
                 f"{sorted(TAXONOMY_RANKS)} 中")
        parent = node["parent"]
        if parent is None:
            continue
        if parent not in nodes:
            fail(f"分类树节点 {node_id!r}: 上级 {parent!r} 不存在")
            continue
        parent_rank = nodes[parent]["rank"]
        allowed = RANK_PARENT_ALLOWED.get(TAXONOMY_RANKS.get(node["rank"]), set())
        if TAXONOMY_RANKS.get(parent_rank) not in allowed:
            fail(f"分类树节点 {node_id!r}: 层级 {node['rank']} 不能挂在上级 "
                 f"{parent!r}（{parent_rank}）下")

    # 无环与根节点校验。
    roots = []
    for node_id in nodes:
        if nodes[node_id]["parent"] is None:
            roots.append(node_id)
            if nodes[node_id]["rank"] not in ROOT_RANKS:
                fail(f"分类树节点 {node_id!r}: 根节点层级必须是界，实为 {nodes[node_id]['rank']}")
            continue
        seen = {node_id}
        current = nodes[node_id]["parent"]
        while current is not None:
            if current in seen:
                fail(f"分类树节点 {node_id!r}: 父链成环（经过 {current!r}）")
                break
            if current not in nodes:
                break
            seen.add(current)
            current = nodes[current]["parent"]

    # 叶子层级：生物只到科；造物只到技术分类；科学/人文/社会只到学科。
    for node_id in nodes:
        if children.get(node_id):
            continue
        rank = nodes[node_id]["rank"]
        if rank in ("科", "技术分类", "学科"):
            continue
        root = _root_of(nodes, node_id)
        if root in BIOLOGICAL_KINGDOMS:
            fail(f"分类树节点 {node_id!r}: 生物叶子层级必须是科，实为 {rank}")
        elif root == CRAFT_ID:
            fail(f"分类树节点 {node_id!r}: 造物叶子层级必须是技术分类，实为 {rank}")
        elif root in DISCIPLINE_ROOTS:
            fail(f"分类树节点 {node_id!r}: 学科叶子层级必须是学科，实为 {rank}")
        else:
            fail(f"分类树节点 {node_id!r}: 非科非技术分类非学科的叶子节点 {rank}")

    for kingdom in BIOLOGICAL_KINGDOMS:
        if kingdom not in nodes:
            fail(f"分类树缺少生物界根节点 {kingdom!r}")
        elif nodes[kingdom]["parent"] is not None:
            fail(f"分类树节点 {kingdom!r}: 生物界必须是根节点")
    if CRAFT_ID not in nodes or nodes[CRAFT_ID]["parent"] is not None:
        fail(f"分类树缺少造物根节点 {CRAFT_ID!r} 或其上级非空")

    if playstyles is None:
        return nodes, children, None, {}
    ids = [pid for pid, _ in playstyles]
    names = dict(playstyles)
    if len(ids) != len(PLAYSTYLE_IDS):
        fail(f"玩法数量 {len(ids)}，期望 {len(PLAYSTYLE_IDS)}")
    if tuple(ids) != PLAYSTYLE_IDS:
        fail(f"玩法 id 与方案不一致：{ids}，期望 {list(PLAYSTYLE_IDS)}")
    dup = [v for v, n in collections.Counter(ids).items() if n > 1]
    if dup:
        fail(f"玩法 id 重复: {dup}")
    return nodes, children, ids, names


# ---- 卡牌分类字段：categories / taxon / 关键词 ----

def card_label(doc):
    """错误信息定位：优先卡牌 id，其次文件名。"""
    return str(doc.get("id") or doc.get("_file") or "?")


def is_biological(doc):
    """生物卡：taxon 的根节点属于六个生物界之一。"""
    root = _root_of(TAXONOMY_NODES, doc.get("taxon"))
    return root in BIOLOGICAL_KINGDOMS


def parse_categories(doc):
    """把 `categories: [kind:value, ...]` 拆成 [(kind, value)]；非法项由 category_errors 报告。"""
    value = doc.get("categories")
    if not isinstance(value, list):
        return []
    items = []
    for item in value:
        if not isinstance(item, str) or ":" not in item:
            continue
        kind, target = item.split(":", 1)
        items.append((kind, target))
    return items


def effective_playstyles(doc):
    """卡牌的全部玩法归属。"""
    return [target for kind, target in parse_categories(doc) if kind == "play"]


def _category_order(target, kind):
    """排序键：先按玩法表顺序，其余按分类树声明顺序。"""
    if kind == "play":
        index = PLAYSTYLE_IDS.index(target) if target in PLAYSTYLE_IDS else len(PLAYSTYLE_IDS)
        return (CATEGORY_RANK[kind], index)
    index = TAXONOMY_ORDER.get(target)
    if index is None and target in TAXONOMY_NODES:
        index = list(TAXONOMY_NODES).index(target)
    return (CATEGORY_RANK[kind], index if index is not None else len(TAXONOMY_NODES))


def category_errors(doc, check_biological=True):
    """解析并校验 categories：语法、前缀、重复、至少 1 条 play、取值与顺序。

    不进商店卡池的卡（衍生物、生成卡）不参与玩法筛选，允许没有 categories。
    """
    label = card_label(doc)
    value = doc.get("categories")
    if value is None:
        if doc.get("in_shop_pool", True):
            return [f"{label}: 缺字段 categories"]
        return []
    if not isinstance(value, list):
        return [f"{label}: categories 必须是列表，实为 {value!r}"]
    errors = []
    seen = set()
    plays = 0
    ordered = []
    for item in value:
        if not isinstance(item, str):
            errors.append(f"{label}: categories 项必须是字符串，实为 {item!r}")
            continue
        if ":" not in item:
            errors.append(f"{label}: categories 项缺少前缀 {item!r}（需 kind:value）")
            continue
        kind, target = item.split(":", 1)
        if not target:
            errors.append(f"{label}: categories 项缺少取值 {item!r}")
            continue
        if item in seen:
            errors.append(f"{label}: categories 条目重复 {item!r}")
        seen.add(item)
        if kind not in CATEGORY_KINDS:
            errors.append(f"{label}: categories 前缀非法 {kind!r}（需 play / prey / host）")
            continue
        if not ID_RE.fullmatch(target):
            errors.append(f"{label}: categories 取值格式非法 {item!r}")
            continue
        ordered.append((kind, target))
        if kind == "play":
            plays += 1
            if target not in PLAYSTYLE_IDS:
                errors.append(f"{label}: play:{target} 取值必须是 13 个玩法之一（不含 neutral）")
        else:
            if TAXONOMY_NODES and target not in TAXONOMY_NODES:
                errors.append(f"{label}: {item!r} 的节点不在分类树中")
            if check_biological and TAXONOMY_NODES and not is_biological(doc):
                errors.append(f"{label}: {item!r} 只允许生物卡（taxon 根节点必须是生物界）")
    if plays == 0 and doc.get("in_shop_pool", True):
        errors.append(f"{label}: categories 至少要有 1 条 play:")
    expected = sorted(ordered, key=lambda pair: _category_order(pair[1], pair[0]))
    if ordered != expected:
        errors.append(f"{label}: categories 顺序必须先 play（按玩法顺序）、再 prey、再 host，"
                      f"同类按分类树顺序")
    return errors


def classification_errors(doc):
    """卡牌 taxon 与 categories 的交叉校验；taxonomy 未加载时只做取值域校验。"""
    label = card_label(doc)
    errors = []
    taxon = doc.get("taxon")
    if taxon is None:
        errors.append(f"{label}: 缺字段 taxon")
        taxon = None
    elif not ID_RE.fullmatch(str(taxon)):
        errors.append(f"{label}: taxon 格式非法 {taxon!r}")
        taxon = None
    elif TAXONOMY_NODES:
        if taxon not in TAXONOMY_NODES:
            errors.append(f"{label}: taxon 节点 {taxon!r} 不在分类树中")
            taxon = None
        elif TAXONOMY_CHILDREN.get(taxon):
            errors.append(f"{label}: taxon {taxon!r} 不是叶子节点")
    errors.extend(category_errors(doc))
    return errors


def keyword_errors(doc, keyword):
    """专属关键词的玩法/形态约束：任一 play 命中允许集合即可。"""
    label = card_label(doc)
    base = str(keyword).split()[0]
    plays = set(effective_playstyles(doc))
    if base in KEYWORD_PLAYSTYLE:
        allowed = KEYWORD_PLAYSTYLE[base]
        if not plays & allowed:
            return [f"{label}: 关键词 {keyword!r} 只允许 play 命中 {sorted(allowed)}，"
                    f"实为 {sorted(plays)}"]
    if base == "evolve":
        if not TAXONOMY_NODES:
            return []
        if not is_biological(doc):
            return [f"{label}: 关键词 {keyword!r} 只允许生物卡（taxon 根节点必须是生物界）"]
    return []


def mechanism_errors(doc, skill_map):
    """新机制原语与选择器的玩法归属；技能解析错误由 check_skills 报告。"""
    label = card_label(doc)
    plays = set(effective_playstyles(doc))
    errors = []
    for sid in card_skills(doc):
        skill = skill_map.get(sid)
        if skill is None:
            continue
        effects, parse_errors = parse_effects(skill.get("_rendered_effects"))
        if parse_errors:
            continue
        for effect in effects:
            op = effect["op"]
            allowed = MECHANISM_PLAYSTYLE.get(op)
            if allowed is not None and not plays & allowed:
                errors.append(f"{label}: 原语 {op} 只允许 play 命中 {sorted(allowed)}，"
                              f"实为 {sorted(plays)}")
            selector = effect["selector"]
            if selector in ("self", "target") or selector.startswith("target:"):
                continue
            parts = selector.split(":")
            pick = parts[1] if len(parts) > 1 else ""
            allowed = PICK_PLAYSTYLE.get(pick)
            if allowed is not None and not plays & allowed:
                errors.append(f"{label}: 选择器 {pick} 只允许 play 命中 {sorted(allowed)}，"
                              f"实为 {sorted(plays)}")
    return errors


def skill_count_errors(doc, skill_map):
    """随从技能数量上限与无技能工具卡约束。"""
    label = card_label(doc)
    skills = card_skills(doc)
    if not skills:
        if doc.get("in_shop_pool", True) is False:
            return []
        if doc.get("taxon") in CRAFT_TOOL_TAXA:
            return []
        return [f"{label}: 无技能随从只允许造物工具卡"]
    level = doc.get("level")
    errors = []
    if level in (1, 2) and len(skills) > 1:
        errors.append(f"{label}: {level} 星最多 1 个技能，实为 {len(skills)} 个")
    elif level in (3, 4) and len(skills) > 2:
        errors.append(f"{label}: {level} 星最多 2 个技能，实为 {len(skills)} 个")
    elif level in (5, 6, 7):
        if len(skills) > 3:
            errors.append(f"{label}: {level} 星最多 3 个技能，实为 {len(skills)} 个")
        keywords = [sid for sid in skills
                    if (skill_map.get(sid) or {}).get("keyword")]
        if len(keywords) > 2:
            errors.append(f"{label}: {level} 星最多 2 个登记关键词，实为 {len(keywords)} 个")
    return errors


def load(kind, required, root=ROOT):
    docs = []
    for path in sorted((root / kind).glob("*.md")):
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
        parts.append(skill.get("_rendered_text", skill.get("text", "")))
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


# 文档属性表里的中文名。校验时由 front matter 的英文 id 反查；gen_text 复用。
ENUM_LABELS = {}
KEYWORD_LABELS = {}
ENUM_COLUMNS = {"card_type": "类型", "playstyle": "玩法", "taxon": "科学分类",
                "era": "时代", "subtype": "子类"}


def _enum_label_value(table, field, value):
    """取属性表里某枚举的中文名；taxon 列保存完整路径，取末级节点名。"""
    label = table.get(ENUM_COLUMNS[field])
    if field == "taxon" and label is not None and TAXONOMY_NODES:
        return re.split(r"\s*›\s*", label)[-1].strip()
    return label


def card_enum_labels(raw, root=ROOT, kind="cards"):
    """从卡牌或英雄属性表收集枚举 id → 中文名；同一 id 映射冲突时报错。"""
    labels = {field: {} for field in ENUM_COLUMNS}
    for doc in raw:
        table = parse_attr_table(root / kind / doc["_file"])
        if table is None:
            continue
        for field in ENUM_COLUMNS:
            value = doc.get(field)
            label = _enum_label_value(table, field, value)
            if value is None or label is None:
                continue
            known = labels[field].get(value)
            if known is not None and known != label:
                raise ValueError(
                    f"{doc['_file']}: {field} {value} 对应多个中文名 {known!r} / {label!r}")
            labels[field][value] = label
    return labels


def keyword_labels():
    """解析关键词判定表「关键词 / 英文名」两列，返回 slug → 中文名。"""
    text = RULES_FILE.read_text(encoding="utf-8")
    marker = "## 1. 关键词判定表"
    if marker not in text:
        raise ValueError(f"{RULES_FILE.name}: 缺少「{marker}」")
    section = text.split(marker, 1)[1].split("\n## ", 1)[0]
    labels = {}
    for line in section.splitlines():
        line = line.strip()
        if not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if len(cells) < 2 or cells[0] == "关键词" or set(cells[0]) <= {"-"}:
            continue
        slug = i18n_terms.keyword_slug(cells[1])
        label = re.sub(r"\s*N$", "", cells[0])
        known = labels.get(slug)
        if known is not None and known != label:
            raise ValueError(f"关键词 {slug} 对应多个中文名 {known!r} / {label!r}")
        labels[slug] = label
    return labels


def keyword_display(value):
    """把关键词 id（如 `breed 2`）翻成属性表里的中文名，层数原样保留。"""
    if not value:
        return value
    words = str(value).split()
    label = KEYWORD_LABELS.get(words[0], words[0])
    return " ".join([label] + words[1:])


def enum_label(kind, value):
    return ENUM_LABELS.get(kind, {}).get(value, value)


def taxon_path(node_id):
    """taxon 的中文完整路径「界 › 门 › 纲 › 目 › 科」；未知节点返回 None。"""
    parts = []
    seen = set()
    current = node_id
    while current is not None:
        node = TAXONOMY_NODES.get(current)
        if node is None or current in seen:
            return None
        seen.add(current)
        parts.append(node.get("name") or current)
        current = node["parent"]
    return " › ".join(reversed(parts))


def playstyle_display(playstyle):
    """单值玩法展示名；neutral 显示「中立」（筛选兜底使用）。"""
    if playstyle == "neutral":
        return "中立"
    return playstyle_name(playstyle)


def playstyle_name(playstyle):
    """13 个玩法的中文名；未知取值原样返回。"""
    if PLAYSTYLE_NAMES:
        return PLAYSTYLE_NAMES.get(playstyle, str(playstyle))
    return enum_label("playstyle", playstyle)


def play_categories_display(doc):
    """玩法列：全部 play 归属，按玩法表固定顺序用「、」分隔。"""
    plays = set(effective_playstyles(doc))
    ordered = [pid for pid in PLAYSTYLE_IDS if pid in plays]
    if not ordered:
        return "无"
    return "、".join(playstyle_name(pid) for pid in ordered)


def specialty_display(doc, hero=False):
    """关联分类列：只渲染 prey / host，段内用「、」分隔；没有时返回空串。

    hero 参数保留调用兼容；英雄与随从使用同一套「捕食：…；寄生：…」格式。
    """
    groups = collections.OrderedDict()
    for kind, target in parse_categories(doc):
        if kind in ("prey", "host"):
            groups.setdefault(kind, []).append(target)
    parts = []
    for prefix in ("prey", "host"):
        targets = groups.get(prefix)
        if not targets:
            continue
        names = [
            (TAXONOMY_NODES.get(target, {}).get("name") or target)
            if TAXONOMY_NODES else enum_label("taxon", target)
            for target in targets
        ]
        title = "捕食：{paths}" if prefix == "prey" else "寄生：{paths}"
        parts.append(title.format(paths="、".join(names)))
    return "；".join(parts)


def expected_card_attrs(doc, skill_map):
    skills = card_skills(doc)
    skill_value = "；".join(skills) if skills else "无"
    keyword_list = derived_keywords(doc, skill_map)
    keyword_value = "；".join(keyword_display(k) for k in keyword_list) if keyword_list else "无"
    playstyle_value = play_categories_display(doc)
    taxon_value = taxon_path(doc.get("taxon")) if TAXONOMY_NODES else ""
    specialty_value = specialty_display(doc) if TAXONOMY_NODES else ""
    if doc["card_type"] == "minion":
        attrs = {
            "id": doc["id"], "名称": doc["name"],
            "类型": enum_label("card_type", doc["card_type"]),
            "玩法": playstyle_value,
            "子类": enum_label("subtype", doc.get("subtype")), "星级": str(doc["level"]),
            "时代": enum_label("era", doc["era"]), "攻击力": str(doc["attack"]),
            "生命值": str(doc["health"]), "关键词": keyword_value,
            "技能": skill_value,
        }
        if taxon_value:
            attrs["科学分类"] = taxon_value
        if specialty_value:
            attrs["关联分类"] = specialty_value
        if not doc.get("in_shop_pool", True):
            attrs["进商店卡池"] = "否"
        return attrs
    attrs = {
        "id": doc["id"], "名称": doc["name"],
        "类型": enum_label("card_type", doc["card_type"]),
        "星级": str(doc["level"]), "时代": enum_label("era", doc.get("era", "")),
        "玩法": playstyle_value,
        "技能": skill_value, "费用": str(doc["cost"]),
    }
    if taxon_value:
        attrs["科学分类"] = taxon_value
    if specialty_value:
        attrs["关联分类"] = specialty_value
    if not doc.get("in_shop_pool", True):
        attrs["进商店卡池"] = "否"
    return attrs


def expected_hero_attrs(doc, skill_map):
    """英雄卡属性表：只含身份与技能，不含数值、时代与科学分类。"""
    skills = card_skills(doc)
    attrs = {
        "id": doc["id"],
        "名称": doc["name"],
        "玩法": play_categories_display(doc),
        "技能": "；".join(skills) if skills else "无",
    }
    specialty_value = specialty_display(doc, hero=True) if TAXONOMY_NODES else ""
    if specialty_value:
        attrs["关联分类"] = specialty_value
    return attrs


def expected_skill_attrs(skill):
    attrs = {
        "id": skill["id"],
        "关键词": keyword_display(skill.get("keyword")) if skill.get("keyword") else "无",
    }
    if "template" in skill:
        try:
            ids = [call.template_id
                   for call in skill_templates.parse_template_field(skill["template"])]
        except ValueError:
            ids = []  # 解析错误由 validate_skill_template 报告
        if ids:
            attrs["模板"] = "；".join(ids)
    if "cost" in skill:
        attrs["费用"] = str(skill["cost"])
        attrs["每回合次数"] = str(skill["uses_per_turn"])
    return attrs


def validate_skill_template(skill, templates, renderer):
    """校验模板实例：模板存在、参数合法、展开后的 DSL 可解析、文案非空。"""
    name = skill.get("_file", skill.get("id", "?"))
    errors = []
    try:
        calls = skill_templates.parse_template_field(skill["template"])
    except ValueError as exc:
        return [f"{name}: {exc}"]
    for call in calls:
        template = templates.get(call.template_id)
        if template is None:
            errors.append(f"{name}: 模板不存在 {call.template_id!r}")
            continue
        try:
            renderer.expand(call)
            renderer.text(call)
        except ValueError as exc:
            errors.append(f"{name}: {exc}")
    if not errors:
        segments = renderer.instance_segments(skill)
        _, parse_errors = parse_effects(";".join(segments) if segments else "[]")
        errors.extend(f"{name}: {message}" for message in parse_errors)
        if not renderer.instance_text(skill).strip():
            errors.append(f"{name}: 渲染文案为空")
    return errors


def skill_field_errors(skill):
    """主动技能字段成对性与取值的校验消息。"""
    name = skill["_file"]
    messages = []
    has_cost = "cost" in skill
    has_uses = "uses_per_turn" in skill
    if has_cost != has_uses:
        messages.append(f"{name}: cost 与 uses_per_turn 必须成对出现")
    if has_cost:
        if type(skill["cost"]) is not int or not 0 <= skill["cost"] <= 3:
            messages.append(f"{name}: 技能费用越界 {skill['cost']!r}")
        if has_uses and (type(skill["uses_per_turn"]) is not int
                         or skill["uses_per_turn"] not in (1, 2)):
            messages.append(f"{name}: 每回合次数非法 {skill['uses_per_turn']!r}")
        if not skill.get("name"):
            messages.append(f"{name}: 主动技能必须填写 name")
    return messages


def check_skills():
    global KEYWORD_LABELS
    KEYWORD_LABELS = keyword_labels()
    skills = load("skills", SKILL_FIELDS)
    if len(skills) != 255:
        fail(f"技能数量 {len(skills)}，期望 255")
    dup = [v for v, n in collections.Counter(s["id"] for s in skills).items() if n > 1]
    if dup:
        fail(f"技能 id 重复: {dup}")
    try:
        templates = skill_templates.load_templates(ROOT)
        renderer = skill_templates.Renderer(ROOT)
    except (OSError, ValueError) as exc:
        fail(f"技能模板加载失败: {exc}")
        return
    for skill in skills:
        name = skill["_file"]
        unknown = set(skill) - set(SKILL_FIELDS) - SKILL_OPTIONAL_FIELDS - {"_file"}
        if unknown:
            fail(f"{name}: 未知字段 {sorted(unknown)}")
        if "template" not in skill:
            fail(f"{name}: 缺少 template 字段")
            continue
        template_errors = validate_skill_template(skill, templates, renderer)
        if template_errors:
            for message in template_errors:
                fail(message)
            continue
        segments = renderer.instance_segments(skill)
        raw_effects = ";".join(segments) if segments else "[]"
        text = renderer.instance_text(skill)
        skill["_rendered_text"] = text
        skill["_rendered_effects"] = raw_effects
        if not text.strip():
            fail(f"{name}: text 为空")
        keyword = skill.get("keyword")
        if keyword and not SKILL_KEYWORD_RE.fullmatch(str(keyword)):
            fail(f"{name}: 关键词非法 {keyword!r}")
        for message in skill_field_errors(skill):
            fail(message)
        parsed_effects, effect_errors = parse_effects(raw_effects)
        for message in effect_errors:
            fail(f"{name}: {message}")
        for message in check_text_trigger(text, raw_effects):
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
        if body_text(path) != text:
            fail(f"{name}: 「## 效果」与 text 不一致")
    return skills


def check_skill_context(_skill_map, _effects):
    """保留调用点；被效果牌引用的技能不再要求额外文本字段。"""
    return


def check_attr_tables(raw, skill_map):
    global ENUM_LABELS
    ENUM_LABELS = card_enum_labels(raw)
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


def check_hero_attr_tables(heroes, skill_map):
    for doc in heroes:
        path = ROOT / "heroes" / doc["_file"]
        table = parse_attr_table(path)
        if table is None:
            fail(f"{doc['_file']}: 缺少「## 属性」表")
            continue
        expected = expected_hero_attrs(doc, skill_map)
        for key, value in expected.items():
            if table.get(key) != value:
                fail(f"{doc['_file']}: 属性表 {key}={table.get(key)!r}，front matter 为 {value!r}")
        extra = set(table) - set(expected)
        if extra:
            fail(f"{doc['_file']}: 属性表多出字段 {sorted(extra)}")


def check_card_rendering(raw, skill_map, kind="cards"):
    for doc in raw:
        expected = rendered_text(doc, skill_map)
        if expected is None:
            continue
        actual = body_text(ROOT / kind / doc["_file"])
        if actual != expected:
            fail(f"{doc['_file']}: 「## 效果」与技能文本拼接不一致：{actual!r} != {expected!r}")


def check_card_sources(raw, kind="cards"):
    for doc in raw:
        text = (ROOT / kind / doc["_file"]).read_text(encoding="utf-8")
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
        if k == "taunt":
            total += .5
        elif k == "divine_shield":
            total += 2
        elif k in ("windfury", "reborn", "evolve"):
            total += 3
        elif k in ("contagion", "ignore_taunt", "sow"):
            total += 2
        elif k in ("poison", "stealth"):
            total += 3
        elif k.startswith("armor"):
            total += 2 + 2 * (int(k.split()[1]) - 1)
        elif k.startswith("breed"):
            total += 2
        elif k.startswith("assemble"):
            total += 2 + 2 * (int(k.split()[1]) - 1)
        elif k in ("battlecry", "deathrattle"):
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


def skill_ref_errors(doc, skill_map):
    """卡牌引用的技能必须在技能池中存在。"""
    return [f"{doc['_file']}: 引用的技能不存在 {sid!r}"
            for sid in card_skills(doc) if sid not in skill_map]


def check_cards(skill_map):
    raw = load("cards", ["id", "name", "card_type", "level", "taxon"])
    for doc in raw:
        if doc["card_type"] not in CARD_TYPES:
            fail(f"{doc['_file']}: card_type 非法 {doc['card_type']!r}")
        if "in_shop_pool" in doc and not isinstance(doc["in_shop_pool"], bool):
            fail(f"{doc['_file']}: in_shop_pool 必须是 true / false，实为 {doc['in_shop_pool']!r}")
        raw_skills = doc.get("skills")
        if raw_skills is not None and not isinstance(raw_skills, list):
            fail(f"{doc['_file']}: skills 必须是列表，实为 {raw_skills!r}")
        subtype = doc.get("subtype")
        if subtype is not None and not ID_RE.fullmatch(str(subtype)):
            fail(f"{doc['_file']}: subtype 格式非法 {subtype!r}")
        if "keywords" in doc:
            fail(f"{doc['_file']}: 残留旧字段 keywords，应改用 skills")
        if "tribe" in doc:
            fail(f"{doc['_file']}: 残留旧字段 tribe，应改用 categories / taxon")
        if "playstyle" in doc:
            fail(f"{doc['_file']}: 残留旧字段 playstyle，应改用 categories")
        if "specialty" in doc:
            fail(f"{doc['_file']}: 残留旧字段 specialty，应改用 categories")
        if "is_hero" in doc:
            fail(f"{doc['_file']}: 残留 is_hero，英雄卡应放在 heroes 目录")
        for message in skill_ref_errors(doc, skill_map):
            fail(message)
        for message in mechanism_errors(doc, skill_map):
            fail(message)
        # 分类字段校验：缺字段时 classification_errors 自己报告，这里对所有卡都跑。
        for message in classification_errors(doc):
            fail(message)
    minions = [c for c in raw if c["card_type"] == "minion"]
    effects = [c for c in raw if c["card_type"] == "effect"]

    skillless = [c for c in minions if not card_skills(c) and c.get("in_shop_pool", True)]
    if len(skillless) != 1:
        fail(f"无技能随从 {len(skillless)} 张，期望 1 张（仅造物工具卡）")
    for c in skillless:
        if body_text(ROOT / "cards" / c["_file"]) != "无":
            fail(f"{c['_file']}: 无技能随从的「## 效果」必须为「无」")

    for c in minions:
        missing = [k for k in MINION_FIELDS if k not in c]
        if missing:
            fail(f"{c['_file']}: 随从缺字段 {missing}")
            continue
        name = card_label(c)
        if c["era"] not in ERAS:
            fail(f"{name}: 时代非法 {c['era']!r}")
        if c["level"] not in RANGE:
            fail(f"{name}: 星级非法 {c['level']!r}")
            continue
        for message in skill_count_errors(c, skill_map):
            fail(message)
        if c.get("in_shop_pool", True):
            lo, hi, lh, hh = RANGE[c["level"]]
            if c["attack"] == 0:
                # 攻击力为 0 的随从保留生命上限 ×1.3 的数值口径，不做攻击下限检查。
                expected_health = hh * 1.3
                if abs(c["health"] - expected_health) > 1:
                    fail(
                        f"{name}: 攻击力 0 的随从生命值 {c['health']} 偏离 "
                        f"{c['level']} 星生命上限×1.3（{expected_health:g}）超过 1 点"
                    )
            elif not (lo <= c["attack"] <= hi and lh <= c["health"] <= hh):
                fail(f"{name}: 数值 {c['attack']}/{c['health']} 越出 {c['level']} 星区间")
            if c["attack"] + c["health"] + keyword_cost(c, skill_map) > CAP[c["level"]]:
                fail(f"{name}: 超模")

    for c in minions:
        for k in derived_keywords(c, skill_map):
            if not KEYWORD_RE.fullmatch(k):
                fail(f"{c['name']}: 派生关键词非法 {k!r}")
            for message in keyword_errors(c, k):
                fail(message)

    for c in effects:
        missing = [k for k in EFFECT_FIELDS if k not in c]
        if missing:
            fail(f"{c['_file']}: 效果牌缺字段 {missing}")
            continue
        name = card_label(c)
        if not card_skills(c):
            fail(f"{c['_file']}: 效果牌缺技能，skills 不能为空")
        if "attack" in c or "health" in c:
            fail(f"{name}: 效果牌不得有 attack/health")
        if c["era"] not in ERAS:
            fail(f"{name}: 时代非法 {c['era']!r}")
        if c["level"] not in RANGE:
            fail(f"{name}: 星级非法 {c['level']!r}")
        if not 0 <= c["cost"] <= 4:
            fail(f"{name}: 费用越界 {c['cost']}")

    for c in effects:
        for k in derived_keywords(c, skill_map):
            for message in keyword_errors(c, k):
                fail(message)

    counts = collections.Counter(c["card_type"] for c in raw)
    if counts != {"minion": 332, "effect": 100}:
        fail(f"卡牌类型分布不符: {dict(counts)}")
    ids = [c["id"] for c in raw]
    dup = [v for v, n in collections.Counter(ids).items() if n > 1]
    if dup:
        fail(f"卡牌 id 重复: {dup}")
    for label, group in (("随从", minions), ("效果", effects)):
        pool = [c for c in group if c.get("in_shop_pool", True)]
        dup = [v for v, n in collections.Counter(c["name"] for c in pool).items() if n > 1]
        if dup:
            fail(f"{label}名称重复: {dup}")

    actual_era = collections.Counter(c.get("era") for c in raw)
    if dict(actual_era) != EXPECT_ERA:
        fail(f"卡牌时代分布不符: {dict(actual_era)}")
    actual_level = collections.Counter(c["level"] for c in minions)
    if dict(actual_level) != EXPECT_LEVEL:
        fail(f"随从星级分布不符: {dict(actual_level)}")
    return raw, minions, effects


def playstyle_counts(minions):
    """按玩法多归属统计随从数量；一张卡计入每条 play，缺 play 的卡单独计数。"""
    counts = collections.Counter()
    for card in minions:
        plays = effective_playstyles(card)
        if not plays:
            counts["<无 play>"] += 1
        for play in plays:
            counts[play] += 1
    return dict(sorted(counts.items()))


HERO_PASSIVE_TRIGGERS = {"Aura", "ConditionalAura", "OnFirstRefresh",
                         "OnCombatStart", "OnTurnStart"}


def hero_skill_errors(doc, skill):
    """英雄技能只有主动与被动：有 cost 只走 OnActivate，无 cost 只走被动触发。"""
    name = doc["_file"]
    skill_id = skill["id"]
    rendered = skill.get("_rendered_effects")
    if rendered is None:
        return []
    effects, _ = parse_effects(rendered)
    triggers = {effect["trigger"] for effect in effects}
    messages = []
    if "cost" in skill:
        if "OnActivate" not in triggers:
            messages.append(f"{name}: 英雄主动技能 {skill_id} 必须有 OnActivate 效果")
        illegal = triggers - {"OnActivate"}
        if illegal:
            messages.append(f"{name}: 英雄主动技能 {skill_id} 不得有触发 {sorted(illegal)}")
    else:
        illegal = triggers - HERO_PASSIVE_TRIGGERS
        if illegal:
            messages.append(f"{name}: 英雄被动技能 {skill_id} 不得有触发 {sorted(illegal)}")
        if not triggers & HERO_PASSIVE_TRIGGERS:
            messages.append(f"{name}: 英雄被动技能 {skill_id} 缺少被动触发")
    return messages


def check_hero_cards(cards, minions, effects, skill_map):
    heroes = load("heroes", HERO_REQUIRED_FIELDS)
    if len(heroes) != HERO_COUNT:
        fail(f"英雄卡 {len(heroes)} 张，期望 {HERO_COUNT}")
    allowed = set(HERO_REQUIRED_FIELDS) | HERO_OPTIONAL_FIELDS | {"_file"}
    by_id = {c["id"]: c for c in minions}
    for doc in heroes:
        extra = set(doc) - allowed
        if extra:
            fail(f"{doc['_file']}: 英雄卡不得有字段 {sorted(extra)}")
        raw_skills = doc.get("skills")
        if raw_skills is not None and not isinstance(raw_skills, list):
            fail(f"{doc['_file']}: skills 必须是列表，实为 {raw_skills!r}")
        intro = doc.get("introduction")
        if not isinstance(intro, str) or not intro.strip():
            fail(f"{doc['_file']}: 缺少非空 introduction")
        elif not 40 <= len(intro.strip()) <= 80:
            fail(f"{doc['_file']}: introduction 长度 {len(intro.strip())}，期望 40–80")
        else:
            text = (ROOT / "heroes" / doc["_file"]).read_text(encoding="utf-8")
            body = section_text(text, "简介")
            if body is None:
                fail(f"{doc['_file']}: 缺少「## 简介」小节")
            elif body.strip() != intro.strip():
                fail(f"{doc['_file']}: 「## 简介」与 introduction 不一致")
        for message in skill_ref_errors(doc, skill_map):
            fail(message)
        # 英雄没有 taxon，生物限制由同 id 随从卡校验。
        for message in category_errors(doc, check_biological=False):
            fail(message)
        for skill_id in doc.get("skills", []):
            skill = skill_map.get(skill_id)
            if skill is None:
                continue
            if skill.get("keyword"):
                fail(f"{doc['_file']}: 英雄技能 {skill_id} 不得登记关键词")
            for message in hero_skill_errors(doc, skill):
                fail(message)
        base = by_id.get(doc["id"])
        if base is None:
            fail(f"{doc['_file']}: 英雄卡 {doc['id']} 没有对应的随从卡")
            continue
        for field in ("name", "categories"):
            if doc.get(field) != base.get(field):
                fail(f"{doc['_file']}: 英雄卡 {field} 与随从卡 {base['_file']} 不一致")
    for label, group in (("英雄卡", heroes),):
        dup = [v for v, n in collections.Counter(c["id"] for c in group).items() if n > 1]
        if dup:
            fail(f"{label} id 重复: {dup}")
        dup = [v for v, n in collections.Counter(c["name"] for c in group).items() if n > 1]
        if dup:
            fail(f"{label}名称重复: {dup}")
    for doc in cards:
        if "is_hero" in doc:
            fail(f"{doc['_file']}: 随从与效果牌不得再有 is_hero")
    for kind, group in (("cards", cards), ("heroes", heroes)):
        for doc in group:
            legacy = [k for k in LEGACY_HERO_FIELDS if k in doc]
            if legacy:
                fail(f"{doc['_file']}: 残留英雄技能字段 {legacy}")
            text = (ROOT / kind / doc["_file"]).read_text(encoding="utf-8")
            if re.search(r"(?m)^## 英雄技能\s*$", text):
                fail(f"{doc['_file']}: 不得再有「## 英雄技能」小节")
    return heroes


def check_achievements():
    achievements = load("achievements", ACHIEVEMENT_FIELDS, V02_ROOT)
    if len(achievements) != 6:
        fail(f"成就数量 {len(achievements)}，期望 6")
    for key in ("id", "name", "steam_api_name"):
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
        if not STEAM_API_NAME_RE.fullmatch(str(a["steam_api_name"])):
            fail(f"{a['name']}: steam_api_name 格式非法 {a['steam_api_name']!r}")
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
    global TAXONOMY_NODES, TAXONOMY_CHILDREN, TAXONOMY_ORDER, PLAYSTYLE_NAMES, ENUM_LABELS
    taxonomy_nodes, taxonomy_children, playstyles, playstyle_names = check_taxonomy()
    TAXONOMY_NODES = taxonomy_nodes or {}
    TAXONOMY_CHILDREN = taxonomy_children or {}
    TAXONOMY_ORDER = {node_id: index for index, node_id in enumerate(TAXONOMY_NODES)}
    PLAYSTYLE_NAMES = playstyle_names or {}
    skills = check_skills()
    skill_map = {s["id"]: s for s in skills}
    raw, minions, effects = check_cards(skill_map)
    check_keyword_unlock(minions, skill_map)
    hero_cards = check_hero_cards(raw, minions, effects, skill_map)
    check_skill_context(skill_map, effects)
    achievements = check_achievements()
    check_attr_tables(raw, skill_map)
    check_hero_attr_tables(hero_cards, skill_map)
    check_card_rendering(raw, skill_map)
    check_card_rendering(hero_cards, skill_map, "heroes")
    check_card_sources(raw)
    check_card_sources(hero_cards, "heroes")
    check_enemy_selectors(raw)
    check_auras(minions)
    if errors:
        print(f"违规 {len(errors)}")
        for item in errors:
            print(" -", item)
        return 1
    print("时代", dict(collections.Counter(c["era"] for c in raw)))
    print("玩法", playstyle_counts(minions))
    print("星级", dict(collections.Counter(c["level"] for c in minions)))
    print("关键词", dict(sorted(collections.Counter(
        k.split()[0] for c in minions for k in derived_keywords(c, skill_map)).items())))
    print("技能引用", dict(sorted(collections.Counter(
        sid for c in raw for sid in card_skills(c)).items())))
    print(f"随从 {len(minions)} 张、效果牌 {len(effects)} 张、技能 {len(skills)} 个、"
          f"英雄卡 {len(hero_cards)} 张，校验通过")
    if taxonomy_nodes is not None:
        print(f"分类树节点 {len(taxonomy_nodes)} 个、玩法 {len(playstyles or [])} 个，校验通过")
    print(f"v0.2 成就 {len(achievements)} 个，校验通过")
    return 0


if __name__ == "__main__":
    sys.exit(main())
