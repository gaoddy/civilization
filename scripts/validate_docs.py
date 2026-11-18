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

ROOT = pathlib.Path(__file__).resolve().parent.parent / "docs" / "versions" / "v0.1" / "product" / "content"
V02_ROOT = pathlib.Path(__file__).resolve().parent.parent / "docs" / "versions" / "v0.2" / "product" / "content"
RULES_FILE = ROOT.parent / "rules" / "keywords-and-mechanics.md"

ID_RE = re.compile(r"[a-z][a-z0-9-]*")
_KEYWORD_NAMES = (
    "battlecry|deathrattle|reborn|taunt|divine_shield|windfury|contagion|"
    "optimal_attack|sow|evolve|poison|stealth|decompose"
)
KEYWORD_RE = re.compile(rf"^({_KEYWORD_NAMES}|breed \d+|assemble \d+|armor \d+)$")
# 技能上的 keyword 是携带者自身登记的常驻关键词，不接受卡牌层的「无」哨兵值。
SKILL_KEYWORD_RE = re.compile(rf"^({_KEYWORD_NAMES}|breed \d+|assemble \d+|armor \d+)$")
AURA_RE = re.compile(r"^你的(其他)?(人类|人科|机械|生物|动物|植物|真菌|细菌|原生生物|病毒|学者|鱼纲|鸟纲|爬行纲|昆虫纲|头足纲|水螅纲|真菌界|原生生物界)?随从获得 ?(.+)$")
AURA_VALUE_RE = re.compile(
    r"^(?:\+\d+/\+\d+|\+\d+ 攻击力|\+\d+ 生命值|最优攻击|嘲讽|护盾|复生|连击)$"
)
TAXONOMY_FILE = ROOT / "taxonomy.md"
# 分类树层级：中文编辑值 → 生成物 rank。
TAXONOMY_RANKS = {"界": "kingdom", "门": "phylum", "纲": "class", "目": "order",
                  "科": "family", "技术分类": "technique", "现象": "phenomenon"}
# rank 的合法出现位置：根只允许界，现象节点自成一树。
RANK_PARENT_ALLOWED = {
    "kingdom": set(),
    "phylum": {"kingdom"},
    "class": {"phylum"},
    "order": {"phylum", "class"},
    "family": {"kingdom", "phylum", "class", "order", "family"},
    "technique": {"kingdom"},
}
ROOT_RANKS = {"界"}
PLAYSTYLE_IDS = ("mammal", "bird", "reptile", "fish", "insect", "cephalopod", "cnidarian",
                 "plant", "fungus", "bacteria", "protozoa", "virus", "machine")
PLAYSTYLE_VALUES = set(PLAYSTYLE_IDS) | {"neutral"}
# 非学者中立卡只允许造物六类工具卡；自动机械归机械玩法。
CRAFT_TOOL_TAXA = {"power-energy", "transport", "weapon-siege",
                   "production-construction", "information-computing", "life-medical"}
MACHINE_TAXON = "automation"
SPECIALTY_KINDS = ("play", "prey", "host")
# 专属关键词及其允许的玩法。evolve 另要求生物卡且非学者，在关键词校验里单独处理。
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
PHENOMENON_ID = "phenomenon"
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
    "OnAllyEvolve", "OnCombatEnd", "Aura", "ConditionalAura",
}
EFFECT_OPS = {
    "Growth", "ModifyStats", "GrantKeyword", "RemoveGrowth", "CannotAttack",
    "DealDamage", "Summon", "Copy", "Discover", "RefreshShop", "AddShopSlot",
    "GainMinion", "Decompose", "Mimic", "Devour", "Detonate", "Convert", "Revive",
    "Recall", "Salvage", "Latent",
}
SELECTOR_SIDES = {"friendly", "enemy", "both", "self", "target"}
SELECTOR_PICKS = {"single", "all", "random", "leftmost", "leftmost+adjacent", "topAttack",
                  "lowestAttack", "adjacent"}
FILTER_KEYS = {"taxon", "playstyle", "play", "prey", "host", "subtype", "id", "type",
               "scholar", "keyword", "other"}
DURATIONS = {"permanent", "combat"}
DECOMPOSE_BASES = {"original", "combat_start", "pre_kill"}
NUMERIC_KEYS = {"times", "count", "amount", "attack", "health", "value", "extra",
                "perAttack", "perHealth", "maxCount", "threshold"}
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
    "RefreshShop": (set(), {"free", "extra"}, set()),
    "AddShopSlot": ({"era", "count"}, set(), set()),
    "GainMinion": ({"level", "to"}, {"from"}, set()),
    "Decompose": ({"basis", "duration"}, {"timesIf", "condition", "when"}, set()),
    "Mimic": ({"duration"}, {"statsOnly"}, set()),
    "Devour": ({"value"}, {"duration", "when"}, set()),
    "Detonate": ({"amount"}, {"threshold", "when"}, set()),
    "Convert": ({"threshold"}, {"token", "card", "count", "when"}, {("token", "card")}),
    "Revive": ({"attack", "health"}, set(), set()),
    "Recall": ({"to"}, set(), set()),
    "Salvage": ({"pool", "count"}, set(), set()),
    "Latent": ({"amount", "count"}, {"threshold", "when"}, set()),
}
# 群聚参数只允许出现在光环的 ModifyStats 上。
CROWD_PARAMS = {"perAttack", "perHealth", "maxCount"}
CONDITION_RE = re.compile(
    r"^count:(friendly|enemy|both):([a-z_]+)=([a-z0-9_-]+):(>=|<=|==|>|<)\d+$"
)
EVENT_TRIGGERS = {
    "OnEnemyDeath", "OnFriendlyDeath", "OnAllySummon", "OnAllyGrowth",
    "OnAllyEvolve", "OnDealDamage", "OnCombatEnd", "OnUse", "OnKill", "OnAllyKill",
}
ACHIEVEMENT_FIELDS = ["id", "name", "event", "condition", "count", "reward", "affects_power"]
RANGE = {1: (1, 2, 2, 4), 2: (2, 3, 3, 5), 3: (3, 5, 4, 7),
         4: (4, 7, 6, 9), 5: (6, 9, 8, 12), 6: (8, 12, 10, 16),
         7: (10, 15, 14, 20)}
CAP = {1: 8, 2: 10, 3: 13, 4: 17, 5: 22, 6: 26, 7: 33}
KW_UNLOCK = {"breed": 1, "taunt": 2, "windfury": 3, "optimal_attack": 3,
             "contagion": 3, "reborn": 4, "divine_shield": 5,
             "poison": 3, "armor": 2, "stealth": 3}
# TODO(Task 15): 新关键词的卡牌数据落地后删除该常量，并把三个关键词改为硬校验。
# 在此之前，关键词未出现时不要求其已出现，出现后仍校验最小星级。
KW_UNLOCK_PENDING = {"poison", "armor", "stealth"}
EXPECT_ERA = {"prehistoric": 49, "primitive": 47, "classical": 34, "feudal": 51,
              "industrial": 38, "modern": 80, "future": 41}
EXPECT_LEVEL = {1: 35, 2: 52, 3: 57, 4: 58, 5: 54, 6: 27, 7: 20}
SKILL_FIELDS = ["id", "text", "effects"]
MINION_FIELDS = ["id", "name", "card_type", "playstyle", "taxon", "subtype", "level", "era",
                 "attack", "health", "is_scholar"]
EFFECT_FIELDS = ["id", "name", "card_type", "level", "era", "playstyle", "taxon",
                 "cost", "match_text"]
LEGACY_HERO_FIELDS = ["hero_skill", "hero_skill_cost", "hero_skill_type",
                      "hero_skill_uses_per_turn", "hero_skill_trigger"]

# 分类上下文，main 在 check_taxonomy 之后写入；单测可直接赋值。
TAXONOMY_NODES = {}
TAXONOMY_CHILDREN = {}
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
    elif key in ("other", "scholar"):
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
    """timesIf 新格式：<键>:<值>:<次数>；scholar 无值，写 scholar:<次数>。"""
    parts = str(value).split(":")
    if len(parts) == 2 and parts[0] == "scholar" and parts[1].isdigit():
        return []
    if len(parts) == 3 and parts[0] in FILTER_KEYS and parts[0] != "scholar" \
            and parts[2].isdigit():
        return check_filter_value(parts[0], parts[1], chunk)
    return [f"timesIf 格式非法 {value!r}（{chunk}）"]


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
        if selector not in ("self", "target"):
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
        if node["rank"] == "现象":
            fail(f"分类树节点 {node_id!r}: 现象节点必须是独立根节点")
            continue
        allowed = RANK_PARENT_ALLOWED.get(TAXONOMY_RANKS.get(node["rank"]), set())
        if TAXONOMY_RANKS.get(parent_rank) not in allowed:
            fail(f"分类树节点 {node_id!r}: 层级 {node['rank']} 不能挂在上级 "
                 f"{parent!r}（{parent_rank}）下")

    # 无环与根节点校验。
    roots = []
    for node_id in nodes:
        if nodes[node_id]["parent"] is None:
            roots.append(node_id)
            if nodes[node_id]["rank"] not in (ROOT_RANKS | {"现象"}):
                fail(f"分类树节点 {node_id!r}: 根节点层级必须是界或现象，实为 {nodes[node_id]['rank']}")
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

    # 叶子层级：生物只到科；造物只到技术分类；现象唯一。
    for node_id in nodes:
        if children.get(node_id):
            continue
        rank = nodes[node_id]["rank"]
        if rank == "科":
            continue
        if rank == "技术分类":
            continue
        if rank == "现象":
            if node_id != PHENOMENON_ID:
                fail(f"分类树节点 {node_id!r}: 现象节点 id 必须为 {PHENOMENON_ID!r}")
            continue
        root = _root_of(nodes, node_id)
        if root in BIOLOGICAL_KINGDOMS:
            fail(f"分类树节点 {node_id!r}: 生物叶子层级必须是科，实为 {rank}")
        elif root == CRAFT_ID:
            fail(f"分类树节点 {node_id!r}: 造物叶子层级必须是技术分类，实为 {rank}")
        else:
            fail(f"分类树节点 {node_id!r}: 非科非技术分类的叶子节点 {rank}")

    phenomena = [i for i, n in nodes.items() if n["rank"] == "现象"]
    if phenomena != [PHENOMENON_ID]:
        fail(f"分类树现象节点必须唯一且为 {PHENOMENON_ID!r}，实为 {phenomena}")

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


# ---- 卡牌分类字段：playstyle / taxon / specialty / 关键词 ----

def card_label(doc):
    """错误信息定位：优先卡牌 id，其次文件名。"""
    return str(doc.get("id") or doc.get("_file") or "?")


def is_biological(doc):
    """生物卡：taxon 存在且不在 craft / phenomenon 下。"""
    root = _root_of(TAXONOMY_NODES, doc.get("taxon"))
    return root is not None and root not in (CRAFT_ID, PHENOMENON_ID)


def specialty_errors(doc):
    """解析并校验 specialty 列表：每条为 kind:value，kind ∈ play/prey/host，且不带空白。"""
    value = doc.get("specialty")
    if value is None:
        return []
    if not isinstance(value, list):
        return [f"specialty 必须是列表，实为 {value!r}"]
    errors = []
    for item in value:
        if ":" not in item:
            errors.append(f"specialty 项缺少前缀 {item!r}（需 kind:value）")
            continue
        kind, target = item.split(":", 1)
        if kind not in SPECIALTY_KINDS:
            errors.append(f"specialty 前缀非法 {kind!r}（需 play / prey / host）")
        elif not ID_RE.fullmatch(target):
            errors.append(f"specialty 取值格式非法 {item!r}")
    return errors


def classification_errors(doc):
    """卡牌 playstyle / taxon / specialty 的交叉校验；taxonomy 未加载时只做取值域校验。"""
    label = card_label(doc)
    errors = []
    card_type = doc.get("card_type")
    kind = "效果牌" if card_type == "effect" else "随从"
    playstyle = doc.get("playstyle")
    taxon = doc.get("taxon")
    is_scholar = doc.get("is_scholar") is True
    if playstyle is None:
        errors.append(f"{label}: 缺字段 playstyle")
    elif playstyle not in PLAYSTYLE_VALUES:
        errors.append(f"{label}: playstyle 非法 {playstyle!r}（需 13 个玩法之一或 neutral）")
        playstyle = None
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
    errors.extend(f"{label}: {e}" for e in specialty_errors(doc))

    if kind == "效果牌":
        if playstyle not in (None, "neutral"):
            errors.append(f"{label}: 效果牌 playstyle 必须为 neutral，实为 {playstyle!r}")
        if taxon not in (None, PHENOMENON_ID):
            errors.append(f"{label}: 效果牌 taxon 必须为 {PHENOMENON_ID}，实为 {taxon!r}")
        if doc.get("specialty"):
            errors.append(f"{label}: 效果牌 specialty 必须为空")
    else:
        if playstyle == "neutral" and not is_scholar:
            if taxon is not None and taxon not in CRAFT_TOOL_TAXA:
                errors.append(f"{label}: 非学者中立卡只允许造物六类工具卡，taxon 实为 {taxon!r}")
        elif playstyle == "machine" and taxon in CRAFT_TOOL_TAXA:
            errors.append(f"{label}: 造物卡的机械玩法要求 taxon={MACHINE_TAXON}，实为 {taxon!r}")
        if is_scholar and playstyle not in (None, "neutral"):
            errors.append(f"{label}: 学者卡 playstyle 必须为 neutral，实为 {playstyle!r}")

    for item in doc.get("specialty") or []:
        if ":" not in item:
            continue
        prefix, target = item.split(":", 1)
        if prefix == "play":
            if card_type == "effect":
                errors.append(f"{label}: 效果牌不得写 {item!r}")
            elif not (is_scholar or playstyle == "neutral"):
                errors.append(f"{label}: play:{target} 只允许学者卡或中立非学者卡"
                              f"（本卡 playstyle={playstyle}）")
            if target not in PLAYSTYLE_IDS:
                errors.append(f"{label}: play:{target} 取值必须是 13 个玩法之一（不含 neutral）")
        elif prefix in ("prey", "host"):
            if TAXONOMY_NODES and taxon is not None and not is_biological(doc):
                errors.append(f"{label}: {item!r} 只允许生物卡（taxon 不在 craft / phenomenon 下）")
            elif TAXONOMY_NODES and target not in TAXONOMY_NODES:
                errors.append(f"{label}: {item!r} 的节点不在分类树中")
    return errors


def keyword_errors(doc, keyword):
    """专属关键词的玩法/形态约束。"""
    label = card_label(doc)
    base = str(keyword).split()[0]
    playstyle = doc.get("playstyle")
    if base in KEYWORD_PLAYSTYLE:
        allowed = KEYWORD_PLAYSTYLE[base]
        if base == "assemble":
            # 中立工具卡用 play:machine 声明机械关联，允许携带装配。
            plays = {item.split(":", 1)[1] for item in doc.get("specialty") or []
                     if item.startswith("play:")}
            if playstyle in allowed or "machine" in plays:
                return []
        if playstyle not in allowed:
            return [f"{label}: 关键词 {keyword!r} 只允许 playstyle ∈ {sorted(allowed)}，"
                    f"实为 {playstyle!r}"]
    if base == "evolve":
        if not TAXONOMY_NODES:
            return []
        if not is_biological(doc):
            return [f"{label}: 关键词 {keyword!r} 只允许生物卡（taxon 不在 craft / phenomenon 下）"]
        if doc.get("is_scholar") is True:
            return [f"{label}: 关键词 {keyword!r} 不允许学者卡"]
    return []


def mechanism_errors(doc, skill_map):
    """新机制原语与选择器的玩法归属；技能解析错误由 check_skills 报告。"""
    label = card_label(doc)
    playstyle = doc.get("playstyle")
    errors = []
    for sid in card_skills(doc):
        skill = skill_map.get(sid)
        if skill is None:
            continue
        effects, parse_errors = parse_effects(skill.get("effects"))
        if parse_errors:
            continue
        for effect in effects:
            op = effect["op"]
            allowed = MECHANISM_PLAYSTYLE.get(op)
            if allowed is not None and playstyle not in allowed:
                errors.append(f"{label}: 原语 {op} 只允许 playstyle ∈ {sorted(allowed)}，"
                              f"实为 {playstyle!r}")
            selector = effect["selector"]
            if selector in ("self", "target"):
                continue
            parts = selector.split(":")
            pick = parts[1] if len(parts) > 1 else ""
            allowed = PICK_PLAYSTYLE.get(pick)
            if allowed is not None and playstyle not in allowed:
                errors.append(f"{label}: 选择器 {pick} 只允许 playstyle ∈ {sorted(allowed)}，"
                              f"实为 {playstyle!r}")
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


def card_enum_labels(raw, root=ROOT):
    """从卡牌属性表收集枚举 id → 中文名；同一 id 映射冲突时报错。"""
    labels = {field: {} for field in ENUM_COLUMNS}
    for doc in raw:
        table = parse_attr_table(root / "cards" / doc["_file"])
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
    """玩法列：13 个玩法取名，neutral 显示「中立」。"""
    if playstyle == "neutral":
        return "中立"
    if PLAYSTYLE_NAMES:
        return PLAYSTYLE_NAMES.get(playstyle, str(playstyle))
    return enum_label("playstyle", playstyle)


def play_heading(is_scholar):
    """play: 段标题：学者为「擅长：」，中立非学者为「关联：」。"""
    return "擅长：{paths}" if is_scholar else "关联：{paths}"


def specialty_display(doc):
    """关联分类列：按 play / prey / host 分段，段内用、分隔；无内容写「无」。"""
    items = doc.get("specialty") or []
    groups = collections.OrderedDict()
    for item in items:
        if ":" not in item:
            continue
        prefix, target = item.split(":", 1)
        groups.setdefault(prefix, []).append(target)
    parts = []
    for prefix, targets in groups.items():
        names = []
        for target in targets:
            if prefix == "play":
                names.append((PLAYSTYLE_NAMES.get(target) or enum_label("playstyle", target))
                             if PLAYSTYLE_NAMES else enum_label("playstyle", target))
            else:
                names.append((TAXONOMY_NODES.get(target, {}).get("name") or target)
                             if TAXONOMY_NODES else enum_label("taxon", target))
        if prefix == "play":
            title = play_heading(doc.get("is_scholar") is True)
        elif prefix == "prey":
            title = "捕食：{paths}"
        elif prefix == "host":
            title = "寄生：{paths}"
        else:
            continue
        parts.append(title.format(paths="、".join(names)))
    return "；".join(parts) if parts else "无"


def expected_card_attrs(doc, skill_map):
    skills = card_skills(doc)
    skill_value = "；".join(skills) if skills else "无"
    keyword_list = derived_keywords(doc, skill_map)
    keyword_value = "；".join(keyword_display(k) for k in keyword_list) if keyword_list else "无"
    playstyle = doc.get("playstyle")
    playstyle_value = playstyle_display(playstyle) if playstyle is not None else ""
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
            "技能": skill_value, "学者": "是" if doc["is_scholar"] else "否",
            "英雄资格": "是" if doc.get("is_hero") else "否",
        }
        if taxon_value:
            attrs["科学分类"] = taxon_value
        if specialty_value:
            attrs["关联分类"] = specialty_value
        return attrs
    attrs = {
        "id": doc["id"], "名称": doc["name"],
        "类型": enum_label("card_type", doc["card_type"]),
        "星级": str(doc["level"]), "时代": enum_label("era", doc.get("era", "")),
        "玩法": playstyle_value,
        "技能": skill_value, "费用": str(doc["cost"]),
        "确定性替换": doc["match_text"],
    }
    if taxon_value:
        attrs["科学分类"] = taxon_value
    if specialty_value:
        attrs["关联分类"] = specialty_value
    return attrs


def expected_skill_attrs(skill):
    attrs = {
        "id": skill["id"],
        "关键词": keyword_display(skill.get("keyword")) if skill.get("keyword") else "无",
        "确定性替换": skill.get("match_text") or "不适用",
    }
    if "cost" in skill:
        attrs["费用"] = str(skill["cost"])
        attrs["每回合次数"] = str(skill["uses_per_turn"])
    return attrs


def skill_field_errors(skill):
    """主动技能字段成对性与取值的校验消息。"""
    name = skill["_file"]
    messages = []
    has_cost = "cost" in skill
    has_uses = "uses_per_turn" in skill
    if has_cost != has_uses:
        messages.append(f"{name}: cost 与 uses_per_turn 必须成对出现")
    if has_cost:
        if type(skill["cost"]) is not int or not 0 <= skill["cost"] <= 2:
            messages.append(f"{name}: 技能费用越界 {skill['cost']!r}")
        if has_uses and (type(skill["uses_per_turn"]) is not int
                         or skill["uses_per_turn"] not in (1, 2)):
            messages.append(f"{name}: 每回合次数非法 {skill['uses_per_turn']!r}")
        if not skill.get("name") or not skill.get("match_text"):
            messages.append(f"{name}: 主动技能必须填写 name 与 match_text")
    elif skill.get("name") and not skill.get("match_text"):
        messages.append(f"{name}: 有 name 的技能必须填写 match_text")
    return messages


def check_skills():
    global KEYWORD_LABELS
    KEYWORD_LABELS = keyword_labels()
    skills = load("skills", SKILL_FIELDS)
    if len(skills) != 193:
        fail(f"技能数量 {len(skills)}，期望 193")
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
        for message in skill_field_errors(skill):
            fail(message)
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


def check_skill_context(skill_map, effects):
    effect_refs = {sid for c in effects for sid in card_skills(c)}
    for sid in sorted(effect_refs):
        skill = skill_map.get(sid)
        if skill is not None and not skill.get("match_text"):
            fail(f"{skill['_file']}: 被效果牌引用的技能缺少 match_text")


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
        if k == "taunt":
            total += .5
        elif k == "divine_shield":
            total += 2
        elif k in ("windfury", "reborn", "evolve"):
            total += 3
        elif k in ("contagion", "optimal_attack", "sow"):
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
        if keyword in KW_UNLOCK_PENDING and keyword not in actual:
            continue
        if actual.get(keyword) != era_level:
            fail(f"关键词 {keyword} 的最小星级为 {actual.get(keyword)}，期望 {era_level}")


def skill_ref_errors(doc, skill_map):
    """卡牌引用的技能必须在技能池中存在。"""
    return [f"{doc['_file']}: 引用的技能不存在 {sid!r}"
            for sid in card_skills(doc) if sid not in skill_map]


def check_cards(skill_map):
    raw = load("cards", ["id", "name", "card_type", "level", "playstyle", "taxon"])
    for doc in raw:
        if doc["card_type"] not in CARD_TYPES:
            fail(f"{doc['_file']}: card_type 非法 {doc['card_type']!r}")
        raw_skills = doc.get("skills")
        if raw_skills is not None and not isinstance(raw_skills, list):
            fail(f"{doc['_file']}: skills 必须是列表，实为 {raw_skills!r}")
        subtype = doc.get("subtype")
        if subtype is not None and not ID_RE.fullmatch(str(subtype)):
            fail(f"{doc['_file']}: subtype 格式非法 {subtype!r}")
        if "keywords" in doc:
            fail(f"{doc['_file']}: 残留旧字段 keywords，应改用 skills")
        if "tribe" in doc:
            fail(f"{doc['_file']}: 残留旧字段 tribe，应改用 playstyle / taxon / specialty")
        for message in skill_ref_errors(doc, skill_map):
            fail(message)
        for message in mechanism_errors(doc, skill_map):
            fail(message)
        # 分类字段校验：缺字段时 classification_errors 自己报告，这里对所有卡都跑。
        for message in classification_errors(doc):
            fail(message)
    minions = [c for c in raw if c["card_type"] == "minion"]
    effects = [c for c in raw if c["card_type"] == "effect"]

    skillless = [c for c in minions if not card_skills(c)]
    if len(skillless) != 7:
        fail(f"无技能随从 {len(skillless)} 张，期望 7 张（skills 缺失或为空）")
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
    if counts != {"minion": 303, "effect": 37}:
        fail(f"卡牌类型分布不符: {dict(counts)}")
    ids = [c["id"] for c in raw]
    dup = [v for v, n in collections.Counter(ids).items() if n > 1]
    if dup:
        fail(f"卡牌 id 重复: {dup}")
    for label, group in (("随从", minions), ("效果", effects)):
        dup = [v for v, n in collections.Counter(c["name"] for c in group).items() if n > 1]
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
    """按玩法统计随从数量（含中立）；缺字段的卡单独计数。"""
    return dict(sorted(collections.Counter(
        c.get("playstyle") if c.get("playstyle") is not None else "<缺字段>"
        for c in minions).items()))


def check_hero_cards(cards, minions, effects):
    hero_cards = [c for c in minions if c.get("is_hero") is True]
    if len(hero_cards) != 32:
        fail(f"英雄卡 {len(hero_cards)} 张，期望 32")
    for c in minions:
        if c.get("is_hero") is True:
            continue
        if "is_hero" in c:
            fail(f"{c['_file']}: 非英雄随从不得有 is_hero")
    for c in effects:
        if "is_hero" in c:
            fail(f"{c['_file']}: 效果牌不得有 is_hero")
    for doc in cards:
        legacy = [k for k in LEGACY_HERO_FIELDS if k in doc]
        if legacy:
            fail(f"{doc['_file']}: 残留英雄技能字段 {legacy}")
        text = (ROOT / "cards" / doc["_file"]).read_text(encoding="utf-8")
        if re.search(r"(?m)^## 英雄技能\s*$", text):
            fail(f"{doc['_file']}: 不得再有「## 英雄技能」小节")
    return hero_cards


def check_achievements():
    achievements = load("achievements", ACHIEVEMENT_FIELDS, V02_ROOT)
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
    global TAXONOMY_NODES, TAXONOMY_CHILDREN, PLAYSTYLE_NAMES, ENUM_LABELS
    taxonomy_nodes, taxonomy_children, playstyles, playstyle_names = check_taxonomy()
    TAXONOMY_NODES = taxonomy_nodes or {}
    TAXONOMY_CHILDREN = taxonomy_children or {}
    PLAYSTYLE_NAMES = playstyle_names or {}
    skills = check_skills()
    skill_map = {s["id"]: s for s in skills}
    raw, minions, effects = check_cards(skill_map)
    check_keyword_unlock(minions, skill_map)
    hero_cards = check_hero_cards(raw, minions, effects)
    check_skill_context(skill_map, effects)
    achievements = check_achievements()
    check_attr_tables(raw, skill_map)
    check_card_rendering(raw, skill_map)
    check_card_sources(raw)
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
