#!/usr/bin/env python3
"""从 v0.1 实体文档生成卡牌、技能、Token 与经济配置数据。

只读 docs/versions/v0.1/product 下的实体文档与数值系统文档，输出确定性 JSON
到 server/data/v0.1/。front matter 解析复用 scripts/validate_docs.py，不重复实现。
输出使用 ensure_ascii=False、indent=2、sort_keys=True，并以换行结尾。
"""

import argparse
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import validate_docs
import audit_effects

ROOT = pathlib.Path(__file__).resolve().parent.parent
DEFAULT_OUTPUT = ROOT / "server" / "data" / "v0.1"
FILES = ("cards.json", "skills.json", "tokens.json", "config.json")

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
}

PICK_JSON = {
    "single": "single",
    "all": "all",
    "random": "random",
    "leftmost": "leftmost",
    "leftmost+adjacent": "leftmost_adjacent",
    "topAttack": "top_attack",
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
INTEGER_KEYS = ("times", "count", "amount", "attack", "health", "value", "extra", "choose")
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
        ):
            result[key] = int(value)
        elif key in ("canKill", "healthOnly", "free"):
            name = {
                "canKill": "can_kill",
                "healthOnly": "health_only",
                "free": "free",
            }[key]
            result[name] = value == "true"
        elif key == "timesIf":
            tribe, times = value.split(":", 1)
            result["times_if"] = {"tribe": tribe, "times": int(times)}
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
        violations = list(audit_effects.check_effect(trigger, op, params))
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


def card_entry(doc):
    return {
        "id": doc["id"],
        "card_type": doc["card_type"],
        "name": doc["name"],
        "tribe": doc["tribe"],
        "subtype": doc.get("subtype"),
        "level": doc["level"],
        "era": doc.get("era"),
        "attack": doc.get("attack"),
        "health": doc.get("health"),
        "skills": doc.get("skills", []),
        "is_scholar": doc.get("is_scholar", False),
        "is_hero": doc.get("is_hero", False),
        "cost": doc.get("cost"),
        "match_text": doc.get("match_text"),
    }


def skill_entry(doc):
    return {
        "id": doc["id"],
        "name": doc.get("name"),
        "keyword": doc.get("keyword"),
        "text": doc["text"],
        "effects": structured_effects(doc["effects"]),
        "cost": doc.get("cost"),
        "uses_per_turn": doc.get("uses_per_turn"),
        "match_text": doc.get("match_text"),
    }


def parse_tokens(path):
    """解析 tokens/README.md 的「## 2. 注册表」表格。"""
    text = path.read_text(encoding="utf-8")
    marker = "## 2. 注册表"
    if marker not in text:
        raise ValueError(f"{path.name}: 缺少「{marker}」小节")
    rest = text.split(marker, 1)[1]
    section = rest.split("\n## ", 1)[0]
    tokens = []
    for line in section.splitlines():
        line = line.strip()
        if not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if len(cells) != 6:
            continue
        if set(cells[0]) <= {"-"} or cells[0] == "id":
            continue
        name, attack, health, keywords = cells[1], cells[2], cells[3], cells[4]
        tokens.append({
            "id": cells[0].strip("`"),
            "name": name,
            "attack": int(attack),
            "health": int(health),
            "keywords": [] if keywords == "无" else keywords.split("、"),
        })
    return sorted(tokens, key=lambda token: token["id"])


def build(root):
    """返回文件名到确定性 JSON 文本的映射。"""
    content = pathlib.Path(root) / "docs" / "versions" / "v0.1" / "product" / "content"
    cards = [card_entry(doc) for doc in load_docs(content, "cards")]
    skills = [skill_entry(doc) for doc in load_docs(content, "skills")]
    tokens = parse_tokens(content / "tokens" / "README.md")
    return {
        "cards.json": dumps(cards),
        "skills.json": dumps(skills),
        "tokens.json": dumps(tokens),
        "config.json": dumps(CONFIG),
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
