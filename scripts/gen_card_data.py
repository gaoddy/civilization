#!/usr/bin/env python3
"""从 v0.1 实体文档生成卡牌、技能、Token 与经济配置数据。

只读 docs/versions/v0.1/product 下的实体文档与数值系统文档，输出确定性 JSON
到 server/data/v0.1/。front matter 解析复用 scripts/validate_docs.py，不重复实现。
输出使用 ensure_ascii=False、indent=2、sort_keys=True，并以换行结尾。
"""

import argparse
import json
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import validate_docs
import audit_effects

ROOT = pathlib.Path(__file__).resolve().parent.parent
DEFAULT_OUTPUT = ROOT / "server" / "data" / "v0.1"
FILES = ("cards.json", "skills.json", "tokens.json", "config.json", "taxonomy.json")

# 产品基线常量；来源 docs/versions/v0.1/product/systems/economy-and-numbers.md
# 与 rules/ 下的核心规则（生命、槽位、回合上限、伤害上下限）。
CONFIG = {
    "initial_hp": 40,
    "board_slots": 7,
    "hand_slots": 10,
    "shop_slots": 6,
    "turn_limit": 20,
    "damage_cap": 15,
    "damage_floor": 1,
    "era_costs": [5, 7, 9, 10, 11, 12],
    "pool_counts": [16, 15, 13, 11, 9, 7, 5],
    "shop_weights": [
        [100, 0, 0, 0, 0, 0, 0],
        [70, 30, 0, 0, 0, 0, 0],
        [50, 35, 15, 0, 0, 0, 0],
        [30, 35, 25, 10, 0, 0, 0],
        [20, 25, 25, 20, 10, 0, 0],
        [10, 15, 25, 25, 15, 10, 0],
        [5, 10, 15, 20, 20, 18, 12],
    ],
    "gold_curve": [3, 3, 4, 5, 6, 7, 8, 9, 10, 10,
                   10, 10, 10, 10, 10, 10, 10, 10, 10, 10],
}


def dumps(value):
    """确定性 JSON 文本：UTF-8 原文、两空格缩进、键排序、末尾换行。"""
    return json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n"


def load_docs(content, kind):
    """按给定内容根读取实体文档并校验文件名与 id 一致；按 id 排序。"""
    docs = []
    for path in sorted((content / kind).glob("*.md")):
        if path.name == "README.md":
            continue
        doc = validate_docs.parse_front_matter(path)
        if path.stem != doc["id"]:
            raise ValueError(f"{path.name}: 文件名与 id {doc['id']!r} 不一致")
        docs.append(doc)
    return sorted(docs, key=lambda doc: doc["id"])


TRIGGER_JSON = {
    "OnPlay": "on_play",
    "OnPlayEffect": "on_play_effect",
    "OnUse": "on_use",
    "OnActivate": "on_activate",
    "OnTurnStart": "on_turn_start",
    "OnFirstRefresh": "on_first_refresh",
    "OnCombatStart": "on_combat_start",
    "OnDealDamage": "on_deal_damage",
    "OnKill": "on_kill",
    "OnAllyKill": "on_ally_kill",
    "OnDeath": "on_death",
    "OnFriendlyDeath": "on_friendly_death",
    "OnEnemyDeath": "on_enemy_death",
    "OnAllySummon": "on_ally_summon",
    "OnAllyGrowth": "on_ally_growth",
    "OnAllyEvolve": "on_ally_evolve",
    "OnCombatEnd": "on_combat_end",
    "Aura": "aura",
    "ConditionalAura": "conditional_aura",
}

OP_JSON = {
    "Growth": "growth",
    "ModifyStats": "modify_stats",
    "GrantKeyword": "grant_keyword",
    "RemoveGrowth": "remove_growth",
    "CannotAttack": "cannot_attack",
    "DealDamage": "deal_damage",
    "Summon": "summon",
    "Copy": "copy",
    "Discover": "discover",
    "RefreshShop": "refresh_shop",
    "AddShopSlot": "add_shop_slot",
    "GainMinion": "gain_minion",
    "Decompose": "decompose",
    "Mimic": "mimic",
    "Devour": "devour",
    "Detonate": "detonate",
    "Convert": "convert",
    "Revive": "revive",
    "Recall": "recall",
    "Salvage": "salvage",
    "Latent": "latent",
}

# timesIf 的键与取值域；与 validate_docs.FILTER_KEYS / check_filter_value 一致。
TIMES_IF_KEYS = validate_docs.FILTER_KEYS
SPECIALTY_KINDS = validate_docs.SPECIALTY_KINDS

PICK_JSON = {
    "single": "single",
    "all": "all",
    "random": "random",
    "leftmost": "leftmost",
    "leftmost+adjacent": "leftmost_adjacent",
    "topAttack": "top_attack",
    "lowestAttack": "lowest_attack",
    "adjacent": "adjacent",
}

COMPARE_JSON = (
    (">=", "ge"),
    ("<=", "le"),
    ("==", "eq"),
    (">", "gt"),
    ("<", "lt"),
)


def _json_selector(selector):
    """把选择器 DSL 转成 self / target / side 对象。"""
    if selector == "self":
        return {"kind": "self"}
    if selector == "target":
        return {"kind": "target"}
    parts = selector.split(":")
    filters = []
    for segment in parts[2:]:
        if not segment:
            continue
        key, value = segment.split("=", 1)
        filters.append({"key": key, "value": value})
    return {
        "kind": "side",
        "side": parts[0],
        "pick": PICK_JSON[parts[1]],
        "filters": filters,
    }


def _json_condition(value):
    """把 `count:<side>:<key>=<value>:<cmp><n>` 转成结构化条件。"""
    rest = value[len("count:"):]
    side, filter_segment, compare_segment = rest.split(":")
    key, filter_value = filter_segment.split("=", 1)
    for prefix, name in COMPARE_JSON:
        if compare_segment.startswith(prefix):
            return {
                "side": side,
                "filter": {"key": key, "value": filter_value},
                "compare": name,
                "count": int(compare_segment[len(prefix):]),
            }
    raise ValueError(f"condition 无法解析: {value!r}")


def _json_keyword(value, params):
    """把 `keyword=繁衍 2` / `keyword=传染,value=1` 归一化为具名对象。"""
    words = value.split()
    base = words[0]
    layer = int(words[1]) if len(words) > 1 else None
    if layer is None and "value" in params:
        layer = int(params["value"])
    result = {"keyword": base}
    if layer is not None:
        result["value"] = layer
    return result


I32_MIN, I32_MAX = -(2**31), 2**31 - 1
INTEGER_KEYS = ("times", "count", "amount", "attack", "health", "value", "extra", "choose",
                "per_attack", "per_health", "max_count", "threshold")
IDENTIFIER_KEYS = ("token", "card", "pool", "era")


def check_structured(effect):
    """按引擎 validate.rs 的同一组规则校验结构化效果，返回违规消息。

    `structured_effects` 在输出前调用，保证 `make data` 通过的数据能被引擎加载；
    规则与 Rust `effects/validate.rs` 一一对应，改一处必须同步另一处。
    """
    trigger = effect["trigger"]
    op = effect["op"]
    selector = effect["selector"]
    params = effect["params"]
    violations = []

    if trigger in ("aura", "conditional_aura") and op not in ("modify_stats", "grant_keyword"):
        violations.append(f"光环触发只能用 modify_stats / grant_keyword，实为 {op}")

    for key in INTEGER_KEYS:
        value = params.get(key)
        if isinstance(value, int) and not I32_MIN <= value <= I32_MAX:
            violations.append(f"参数 {key} 超出 i32 范围：{value}")

    if "times_if" in params:
        times = params["times_if"]["times"]
        if op != "growth" or "times" not in params:
            violations.append("timesIf 只能与 Growth 的 times 同时出现")
        if not 0 <= times <= I32_MAX:
            violations.append(f"timesIf 的次数非法：{times}")

    if "condition" in params and not 0 <= params["condition"]["count"] <= I32_MAX:
        violations.append(f"condition 的 count 非法：{params['condition']['count']}")

    for key in IDENTIFIER_KEYS:
        if key in params and not params[key]:
            violations.append(f"参数 {key} 不能为空")

    if "keyword" in params:
        keyword = params["keyword"]
        base = keyword["keyword"]
        layer = keyword.get("value", params.get("value"))
        if base in audit_effects.VALUE_KEYWORDS:
            if layer is None:
                violations.append(f"keyword {base} 必须带层数")
            elif not 0 <= layer <= I32_MAX:
                violations.append(f"keyword {base} 的层数不能为负：{layer}")
        elif layer is not None:
            violations.append(f"keyword {base} 禁止层数与 value")

    if op == "discover" and params.get("choose") != 1:
        violations.append(f"Discover 的 choose 必须为 1，实为 {params.get('choose')!r}")

    if op == "gain_minion" and params.get("from") not in (None, "shop"):
        violations.append(f"GainMinion 的 from 只接受 shop，实为 {params.get('from')!r}")

    if selector["kind"] == "side" and selector["side"] not in ("friendly", "enemy", "both"):
        violations.append(f"选择器阵营非法 {selector['side']!r}")

    return violations


def _json_times_if(value):
    """把 `<键>:<值>:<次数>` 转为结构化；`scholar:<次数>` 没有值，输出 value 为 null。

    格式与取值域由 validate_docs.times_if_errors 先行校验，这里只做拆分。
    """
    parts = value.split(":")
    if len(parts) == 2 and parts[0] == "scholar":
        key, filter_value, times = parts[0], None, parts[1]
    elif len(parts) == 3 and parts[0] in TIMES_IF_KEYS and parts[0] != "scholar":
        key, filter_value, times = parts
    else:
        raise ValueError(f"timesIf 无法解析: {value!r}")
    return {"key": key, "value": filter_value, "times": int(times)}


def _json_params(params):
    """把 DSL 参数字典转成带类型的结构。"""
    result = {}
    for key, value in params.items():
        if key == "type":
            result["card_type"] = value
        elif key in (
            "times",
            "count",
            "amount",
            "attack",
            "health",
            "value",
            "extra",
            "choose",
            "perAttack",
            "perHealth",
            "maxCount",
            "threshold",
        ):
            name = {
                "perAttack": "per_attack",
                "perHealth": "per_health",
                "maxCount": "max_count",
            }.get(key, key)
            result[name] = int(value)
        elif key in ("canKill", "healthOnly", "statsOnly", "free"):
            name = {
                "canKill": "can_kill",
                "healthOnly": "health_only",
                "statsOnly": "stats_only",
                "free": "free",
            }[key]
            result[name] = value == "true"
        elif key == "timesIf":
            result["times_if"] = _json_times_if(value)
        elif key == "condition":
            result["condition"] = _json_condition(value)
        elif key == "when":
            filter_key, filter_value = value.split("=", 1)
            result["when"] = {"key": filter_key, "value": filter_value}
        elif key == "keyword":
            result["keyword"] = _json_keyword(value, params)
        else:
            result[key] = value
    return result


def structured_effects(value):
    """把技能 effects DSL 编译为结构化数组；非法输入抛 ValueError。"""
    effects, errors = validate_docs.parse_effects(value)
    if errors:
        raise ValueError("; ".join(errors))

    result = []
    for effect in effects:
        trigger = effect["trigger"]
        op = effect["op"]
        params = effect["params"]
        violations = list(audit_effects.check_effect(
            trigger, op, params, effect["selector"]))
        structured = {
            "trigger": TRIGGER_JSON[trigger],
            "op": OP_JSON[op],
            "selector": _json_selector(effect["selector"]),
            "params": _json_params(params),
        }
        violations.extend(check_structured(structured))
        if violations:
            raise ValueError(f"{trigger}|{op}: " + "; ".join(violations))
        result.append(structured)
    return result


def parse_specialty(value):
    """把 `kind:value` 列表转成 `[{"kind": ..., "value": ...}]`；省略时为 []。

    前缀与取值由 validate_docs 校验；这里只做拆分，未知前缀直接报错。
    """
    if value is None:
        return []
    if not isinstance(value, list):
        raise ValueError(f"specialty 必须是列表，实为 {value!r}")
    items = []
    for item in value:
        kind, _, target = item.partition(":")
        if kind not in SPECIALTY_KINDS or not target:
            raise ValueError(f"specialty 项无法解析: {item!r}")
        items.append({"kind": kind, "value": target})
    return items


def card_entry(doc):
    return {
        "id": doc["id"],
        "card_type": doc["card_type"],
        "playstyle": doc["playstyle"],
        "taxon": doc["taxon"],
        "specialty": parse_specialty(doc.get("specialty")),
        "subtype": doc.get("subtype"),
        "level": doc["level"],
        "era": doc.get("era"),
        "attack": doc.get("attack"),
        "health": doc.get("health"),
        "skills": doc.get("skills", []),
        "is_scholar": doc.get("is_scholar", False),
        "is_hero": doc.get("is_hero", False),
        "cost": doc.get("cost"),
    }


def skill_entry(doc):
    return {
        "id": doc["id"],
        "keyword": doc.get("keyword"),
        "effects": structured_effects(doc["effects"]),
        "cost": doc.get("cost"),
        "uses_per_turn": doc.get("uses_per_turn"),
    }


def _token_rows(path):
    """解析 tokens/README.md 的「## 2. 注册表」表格，逐行产出单元格。"""
    text = path.read_text(encoding="utf-8")
    marker = "## 2. 注册表"
    if marker not in text:
        raise ValueError(f"{path.name}: 缺少「{marker}」小节")
    rest = text.split(marker, 1)[1]
    section = rest.split("\n## ", 1)[0]
    for line in section.splitlines():
        line = line.strip()
        if not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if len(cells) != 8:
            continue
        if set(cells[0]) <= {"-"} or cells[0] == "id":
            continue
        yield cells


def parse_tokens(path):
    """从 tokens/README.md 生成规则字段；展示名由 `parse_token_names` 提供。

    「玩法」「科学分类」两列缺失或为空时回退 `neutral` / `phenomenon`，与引擎口径一致。
    """
    tokens = []
    for cells in _token_rows(path):
        attack, health, keywords = cells[2], cells[3], cells[4]
        tokens.append({
            "id": cells[0].strip("`"),
            "attack": int(attack),
            "health": int(health),
            "keywords": [] if keywords == "无" else keywords.split("、"),
            "playstyle": cells[5] or "neutral",
            "taxon": cells[6] or "phenomenon",
        })
    return sorted(tokens, key=lambda token: token["id"])


def parse_token_names(path):
    """从 tokens/README.md 读取 token id 到展示名的映射，供文本表使用。"""
    return {
        cells[0].strip("`"): cells[1]
        for cells in _token_rows(path)
    }


TAXONOMY_FILE = "taxonomy.md"
# 分类树中文层级 → 生成物 rank；与 validate_docs.TAXONOMY_RANKS 保持一致。
TAXONOMY_RANKS = {
    "界": "kingdom",
    "门": "phylum",
    "纲": "class",
    "目": "order",
    "科": "family",
    "技术分类": "technique",
    "现象": "phenomenon",
}


def _section_rows(text, heading):
    """返回指定小节的表格数据行（单元格列表）。

    heading 不含 `## ` 前缀；`#` 数量不限，匹配同标记数的下一个标题即为小节结尾。
    """
    match = re.search(rf"(?m)^(##+) {re.escape(heading)}\s*$", text)
    if not match:
        raise ValueError(f"缺少「## {heading}」小节")
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


def parse_taxonomy(path):
    """解析 taxonomy.md，返回 {"nodes": [...], "playstyles": [...]}。

    rank 由中文层级映射为英文；节点按 id 排序以保证确定性。
    """
    text = pathlib.Path(path).read_text(encoding="utf-8")
    nodes = []
    for cells in _section_rows(text, "1. 科学分类树"):
        if len(cells) != 5:
            raise ValueError(f"{path.name}: 分类树行列数非法 {cells!r}")
        node_id, _, parent, rank, _ = cells
        nodes.append({
            "id": node_id,
            "parent": parent or None,
            "rank": TAXONOMY_RANKS[rank],
        })
    playstyles = [cells[0] for cells in _section_rows(text, "2. 玩法")]
    return {
        "nodes": sorted(nodes, key=lambda node: node["id"]),
        "playstyles": playstyles,
    }


def build(root):
    """返回文件名到确定性 JSON 文本的映射。"""
    content = pathlib.Path(root) / "docs" / "versions" / "v0.1" / "product" / "content"
    cards = [card_entry(doc) for doc in load_docs(content, "cards")]
    skills = [skill_entry(doc) for doc in load_docs(content, "skills")]
    tokens = parse_tokens(content / "tokens" / "README.md")
    taxonomy = parse_taxonomy(content / TAXONOMY_FILE)
    return {
        "cards.json": dumps(cards),
        "skills.json": dumps(skills),
        "tokens.json": dumps(tokens),
        "config.json": dumps(CONFIG),
        "taxonomy.json": dumps(taxonomy),
    }


def write(root, output_dir):
    """将生成结果写入 output_dir（不存在时创建）。"""
    out = pathlib.Path(output_dir)
    out.mkdir(parents=True, exist_ok=True)
    data = build(root)
    for name in FILES:
        (out / name).write_text(data[name], encoding="utf-8")


def check(output_dir):
    """比较当前生成结果与已提交数据；返回差异文件名列表。"""
    out = pathlib.Path(output_dir)
    data = build(ROOT)
    drift = []
    for name in FILES:
        path = out / name
        if not path.exists() or path.read_text(encoding="utf-8") != data[name]:
            drift.append(name)
    return drift


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="检测生成数据是否漂移，有差异退出 1")
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT), help="输出目录")
    args = parser.parse_args(argv)
    if args.check:
        drift = check(args.output)
        if drift:
            print(f"数据漂移 {len(drift)} 个文件：{', '.join(drift)}")
            return 1
        print("生成数据无漂移")
        return 0
    write(ROOT, args.output)
    print(f"已生成 {len(FILES)} 个文件到 {args.output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
