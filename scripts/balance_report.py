#!/usr/bin/env python3
"""玩法身材与效果牌平衡报告。

读取 ``server/data/v0.1/cards.json``（或 ``--cards`` 指定文件），统计：

- 每个玩法非学者随从的平均攻血比（攻击力 ÷（攻击力 + 生命值））与各星级分布；
- 3–4 星、5–7 星的多技能密度；
- 效果牌总数、按星级/费用分布、每星级占比、定向与通用数量。

``--check`` 有越界时退出码 1。

规则来源：docs/versions/v0.1/update/2026-11-18-card-differentiation/product.md
"""

import argparse
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
DEFAULT_CARDS = ROOT / "server/data/v0.1/cards.json"
DEFAULT_SKILLS = ROOT / "server/data/v0.1/skills.json"

# 产品 §5.2「玩法机制词汇」；键为 cards.json 的 playstyle 值。
MECHANISM_VOCAB = {
    "mammal": {"成长", "嘲讽", "连击", "群聚"},
    "bird": {"俯冲", "连击", "最优攻击", "战吼成长"},
    "insect": {"繁衍", "连击", "最优攻击", "召唤工蜂"},
    "fish": {"群聚", "复生", "分裂", "最优攻击"},
    "reptile": {"减伤", "剧毒", "嘲讽"},
    "cnidarian": {"复苏", "剧毒", "减伤", "分裂"},
    "plant": {"繁衍", "播种", "分裂", "嘲讽"},
    "fungus": {"分裂", "分解", "亡语 AOE"},
    "protozoa": {"吞噬", "分裂", "攻击光环"},
    "cephalopod": {"潜行", "拟态", "剧毒", "吞噬"},
    "bacteria": {"传染", "引爆", "转化", "减伤"},
    "virus": {"传染", "潜伏", "亡语 AOE"},
    "machine": {"装配", "零件回收", "最优攻击", "战吼成长"},
}
KEYWORD_TERMS = {
    "taunt": "嘲讽", "windfury": "连击", "poison": "剧毒", "reborn": "复生",
    "sow": "播种", "decompose": "分解", "stealth": "潜行", "contagion": "传染",
    "ignore_taunt": "最优攻击",
}
KEYWORD_PREFIX_TERMS = {"breed": "繁衍", "assemble": "装配", "armor": "减伤"}
OP_TERMS = {
    "revive": "复苏", "devour": "吞噬", "mimic": "拟态", "decompose": "分解",
    "detonate": "引爆", "convert": "转化", "latent": "潜伏", "salvage": "零件回收",
}

TARGETS = {
    "mammal": (0.42, 0.46), "bird": (0.50, 0.55), "insect": (0.48, 0.54),
    "fish": (0.43, 0.48), "reptile": (0.28, 0.33), "cnidarian": (0.28, 0.33),
    "plant": (0.26, 0.31), "fungus": (0.35, 0.40), "protozoa": (0.42, 0.47),
    "cephalopod": (0.39, 0.44), "bacteria": (0.31, 0.36), "virus": (0.34, 0.39),
    "machine": (0.42, 0.47), "neutral": (0.42, 0.47),
}
TOLERANCE = 0.04
MIN_SPREAD = 0.20
EFFECT_LEVELS = {1: 10, 2: 18, 3: 22, 4: 20, 5: 16, 6: 9, 7: 5}
EFFECT_COSTS = {0: 8, 1: 33, 2: 34, 3: 19, 4: 6}
EFFECT_SHARE_CAP = 0.30
TARGETED = 39
GENERIC = 61
TARGETED_PER_PLAYSTYLE = 3
PLAYSTYLES = [key for key in TARGETS if key != "neutral"]


def _num(value):
    return value if isinstance(value, (int, float)) else 0


def _combat_minions(cards):
    """参与攻血比统计的卡：非学者随从。"""
    return [c for c in cards if c.get("card_type") == "minion" and not c.get("is_scholar")]


def _share(attack, health):
    total = attack + health
    return attack / total if total else 0.0


def playstyle_stats(cards):
    stats = {
        playstyle: {"count": 0, "attack": 0, "health": 0, "attack_share": 0.0, "by_level": {}}
        for playstyle in TARGETS
    }
    for card in _combat_minions(cards):
        playstyle = card.get("playstyle")
        if playstyle not in stats:
            continue
        attack = _num(card.get("attack"))
        health = _num(card.get("health"))
        entry = stats[playstyle]
        entry["count"] += 1
        entry["attack"] += attack
        entry["health"] += health
        level = card.get("level")
        bucket = entry["by_level"].setdefault(
            level, {"count": 0, "attack": 0, "health": 0, "attack_share": 0.0}
        )
        bucket["count"] += 1
        bucket["attack"] += attack
        bucket["health"] += health

    for entry in stats.values():
        entry["attack_share"] = _share(entry["attack"], entry["health"])
        for bucket in entry["by_level"].values():
            bucket["attack_share"] = _share(bucket["attack"], bucket["health"])
    return stats


def skill_density(cards):
    minions = _combat_minions(cards)
    result = {}
    for label, levels in (("3-4", (3, 4)), ("5-7", (5, 6, 7))):
        pool = [c for c in minions if c.get("level") in levels]
        multi = [c for c in pool if len(c.get("skills") or []) >= 2]
        result[label] = len(multi) / len(pool) if pool else 0.0
    return result


def _targeted_playstyle(card):
    for specialty in card.get("specialty") or []:
        if specialty.get("kind") == "play":
            return specialty.get("value")
    return None


def effect_stats(cards):
    effects = [c for c in cards if c.get("card_type") == "effect"]
    by_level = {level: 0 for level in EFFECT_LEVELS}
    by_cost = {cost: 0 for cost in EFFECT_COSTS}
    for card in effects:
        by_level[card.get("level")] = by_level.get(card.get("level"), 0) + 1
        by_cost[card.get("cost")] = by_cost.get(card.get("cost"), 0) + 1

    level_totals = {}
    for card in cards:
        level = card.get("level")
        level_totals[level] = level_totals.get(level, 0) + 1
    level_share = {
        level: by_level.get(level, 0) / level_totals[level]
        for level in EFFECT_LEVELS
        if level_totals.get(level)
    }

    targeted_by_playstyle = {playstyle: 0 for playstyle in PLAYSTYLES}
    targeted = 0
    for card in effects:
        playstyle = _targeted_playstyle(card)
        if playstyle is None:
            continue
        targeted += 1
        targeted_by_playstyle[playstyle] = targeted_by_playstyle.get(playstyle, 0) + 1

    return {
        "total": len(effects),
        "by_level": by_level,
        "by_cost": by_cost,
        "level_share": level_share,
        "targeted": targeted,
        "generic": len(effects) - targeted,
        "targeted_by_playstyle": targeted_by_playstyle,
    }


def _band_errors(stats, playstyles):
    errors = []
    for playstyle in playstyles:
        entry = stats[playstyle]
        low, high = TARGETS[playstyle]
        if entry["count"] == 0:
            errors.append(f"玩法 {playstyle} 没有非学者随从")
            continue
        share = entry["attack_share"]
        if not (low - TOLERANCE <= share <= high + TOLERANCE):
            errors.append(
                f"玩法 {playstyle} 平均攻血比 {share:.3f} 超出目标带 "
                f"{low:.2f}–{high:.2f} ±{TOLERANCE:.2f}"
            )
    return errors


def skill_terms(skill):
    """从技能的结构化字段识别命中的玩法机制词汇。"""
    terms = set()
    keyword = (skill.get("keyword") or "").strip()
    base = keyword.split()[0] if keyword else ""
    if base in KEYWORD_TERMS:
        terms.add(KEYWORD_TERMS[base])
    if base in KEYWORD_PREFIX_TERMS:
        terms.add(KEYWORD_PREFIX_TERMS[base])
    for effect in skill.get("effects") or []:
        op = effect.get("op")
        trigger = effect.get("trigger") or ""
        params = effect.get("params") or {}
        selector = effect.get("selector") or {}
        if op == "growth":
            terms.add("成长")
            if trigger == "on_play":
                terms.add("战吼成长")
        if op in OP_TERMS:
            terms.add(OP_TERMS[op])
        if op == "modify_stats" and (params.get("per_attack") or params.get("per_health")):
            terms.add("群聚")
        if (op in ("modify_stats", "aura")
                and trigger in ("aura", "conditional_aura")
                and _num(params.get("attack")) > 0
                and selector.get("kind") == "side"
                and selector.get("side") == "friendly"
                and selector.get("pick") == "all"):
            terms.add("攻击光环")
        if op == "summon":
            token = str(params.get("token") or "")
            if trigger == "on_death" and base != "sow":
                terms.add("分裂")
            elif trigger != "on_death" and "worker-bee" in token:
                terms.add("召唤工蜂")
        if op == "deal_damage" and trigger == "on_death" and selector.get("pick") == "all":
            terms.add("亡语 AOE")
        if (op == "deal_damage" and trigger == "on_combat_start"
                and selector.get("pick") == "lowest_attack"):
            terms.add("俯冲")
        if op == "grant_keyword":
            granted = (params.get("keyword") or {}).get("keyword")
            if granted == "ignore_taunt":
                terms.add("最优攻击")
            if granted == "contagion":
                terms.add("传染")
    return terms


def effect_cost_errors(cards):
    """4 费效果牌只能落在 6–7 星。"""
    errors = []
    for card in cards:
        if card.get("card_type") != "effect" or card.get("cost") != 4:
            continue
        level = card.get("level")
        if level not in (6, 7):
            errors.append(
                f"效果牌 {card.get('id')} 为 4 费，星级应为 6–7，实际 {level}"
            )
    return errors


def vocabulary_errors(cards, skills):
    """多技能随从的第二/后续技能必须落在本玩法机制词汇内。"""
    errors = []
    for card in cards:
        if card.get("card_type") != "minion" or card.get("is_scholar") or card.get("is_hero"):
            continue
        vocab = MECHANISM_VOCAB.get(card.get("playstyle"))
        if vocab is None:
            continue
        ids = [sid for sid in card.get("skills") or [] if sid in skills]
        if len(ids) < 2:
            continue
        terms = [skill_terms(skills[sid]) for sid in ids]
        cid = card.get("id")
        level = card.get("level")
        if level in (3, 4):
            if not (terms[1] & vocab):
                errors.append(
                    f"卡 {cid}（{level} 星）第二个技能 {ids[1]} 不属于 3–4 星玩法词汇"
                )
            continue
        matched = sum(1 for entry in terms if entry & vocab)
        if matched == 0:
            errors.append(f"卡 {cid}（{level} 星）缺少本玩法签名机制")
        elif len(terms) - matched > 1:
            errors.append(f"卡 {cid}（{level} 星）表外技能超过 1 个")
    return errors


def violations(cards, playstyles=None, skills=None):
    stats = playstyle_stats(cards)
    checked = list(playstyles) if playstyles else list(TARGETS)
    errors = _band_errors(stats, checked)
    if playstyles:
        return errors

    errors.extend(effect_cost_errors(cards))
    if skills:
        errors.extend(vocabulary_errors(cards, skills))

    counted = [
        stats[playstyle]["attack_share"]
        for playstyle in PLAYSTYLES
        if stats[playstyle]["count"]
    ]
    if len(counted) >= 2:
        spread = max(counted) - min(counted)
        if spread < MIN_SPREAD:
            errors.append(f"13 个玩法攻血比差 {spread:.3f} 小于 {MIN_SPREAD:.2f}")

    density = skill_density(cards)
    if density["3-4"] < 0.50:
        errors.append(f"3–4 星多技能占比 {density['3-4']:.0%} 低于 50%")
    if density["5-7"] < 0.80:
        errors.append(f"5–7 星多技能占比 {density['5-7']:.0%} 低于 80%")

    effects = effect_stats(cards)
    if effects["total"] != sum(EFFECT_LEVELS.values()):
        errors.append(f"效果牌总数应为 100，实际 {effects['total']}")
    for level, expected in EFFECT_LEVELS.items():
        actual = effects["by_level"].get(level, 0)
        if actual != expected:
            errors.append(f"{level} 星效果牌应为 {expected} 张，实际 {actual}")
        share = effects["level_share"].get(level, 0.0)
        if share > EFFECT_SHARE_CAP:
            errors.append(f"{level} 星效果牌占比 {share:.0%} 超过 30%")
    for cost, expected in EFFECT_COSTS.items():
        actual = effects["by_cost"].get(cost, 0)
        if actual != expected:
            errors.append(f"{cost} 费效果牌应为 {expected} 张，实际 {actual}")
    if effects["targeted"] != TARGETED:
        errors.append(f"定向效果牌应为 {TARGETED} 张，实际 {effects['targeted']}")
    if effects["generic"] != GENERIC:
        errors.append(f"通用效果牌应为 {GENERIC} 张，实际 {effects['generic']}")
    for playstyle in PLAYSTYLES:
        actual = effects["targeted_by_playstyle"].get(playstyle, 0)
        if actual != TARGETED_PER_PLAYSTYLE:
            errors.append(
                f"玩法 {playstyle} 的定向效果牌应为 3 张，实际 {actual} 张"
            )
    return errors


def report(cards, skills=None):
    stats = playstyle_stats(cards)
    counted = [entry["attack_share"] for entry in stats.values() if entry["count"]]
    return {
        "playstyles": stats,
        "spread": (max(counted) - min(counted)) if len(counted) >= 2 else 0.0,
        "skill_density": skill_density(cards),
        "effects": effect_stats(cards),
        "violations": violations(cards, skills=skills),
    }


def _print_human(data):
    print(f"{'玩法':<10}{'数量':>5}{'攻血比':>9}  星级攻血比")
    for playstyle, entry in data["playstyles"].items():
        if not entry["count"]:
            print(f"{playstyle:<10}{0:>5}{'—':>9}")
            continue
        levels = " ".join(
            f"{level}:{bucket['attack_share']:.2f}"
            for level, bucket in sorted(entry["by_level"].items(), key=lambda kv: kv[0] or 0)
        )
        print(f"{playstyle:<10}{entry['count']:>5}{entry['attack_share']:>9.3f}  {levels}")
    print(f"\n攻血比差：{data['spread']:.3f}")
    density = data["skill_density"]
    print(f"多技能密度：3–4 星 {density['3-4']:.0%}，5–7 星 {density['5-7']:.0%}")
    effects = data["effects"]
    print(
        f"效果牌：{effects['total']} 张（定向 {effects['targeted']}，通用 {effects['generic']}）"
    )
    print("  按星级：" + "，".join(f"{k} 星 {v}" for k, v in sorted(effects["by_level"].items())))
    print("  按费用：" + "，".join(f"{k} 费 {v}" for k, v in sorted(effects["by_cost"].items())))
    print("  星级占比：" + "，".join(
        f"{k} 星 {v:.0%}" for k, v in sorted(effects["level_share"].items())
    ))


def main(argv=None):
    parser = argparse.ArgumentParser(description="玩法身材与效果牌平衡报告")
    parser.add_argument("--check", action="store_true", help="有越界时退出码 1")
    parser.add_argument("--json", action="store_true", help="输出 JSON")
    parser.add_argument("--playstyle", action="append", default=None,
                        help="只检查指定玩法（可重复），跳过攻血比差与效果牌检查")
    parser.add_argument("--cards", default=str(DEFAULT_CARDS), help="卡牌 JSON 路径")
    parser.add_argument("--skills", default=str(DEFAULT_SKILLS), help="技能 JSON 路径")
    args = parser.parse_args(argv)

    with open(args.cards, encoding="utf-8") as handle:
        cards = json.load(handle)
    with open(args.skills, encoding="utf-8") as handle:
        skills = {skill["id"]: skill for skill in json.load(handle)}

    if args.playstyle:
        unknown = [style for style in args.playstyle if style not in TARGETS]
        if unknown:
            parser.error(f"未知玩法 {unknown}")
        data = report(cards)
        errors = _band_errors(data["playstyles"], args.playstyle)
        data["violations"] = errors
    else:
        data = report(cards, skills)
        errors = data["violations"]

    if args.json:
        print(json.dumps(data, ensure_ascii=False, indent=2))
    else:
        _print_human(data)
        if errors:
            print("\n违规：")
            for error in errors:
                print(f"  - {error}")
        else:
            print("\n平衡检查通过")

    if args.check and errors:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
