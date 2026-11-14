#!/usr/bin/env python3
"""校验 docs/versions/v0.1/product/content 下的卡牌与英雄实体文档。

只使用标准库；front matter 仅支持本项目生成的两种形式：
    key: 标量
    keywords: [词 1, 词 2]
卡牌分两类：card_type=随从 必须有攻击力/生命值；card_type=效果 必须没有。
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
TRIBE_KEYWORD = {"装配": "机械", "播种": "植物", "进化": "生物"}
TRIBES = {"人类", "机械", "生物", "植物", "病毒", "学者"}
ERAS = ["史前", "原始", "封建", "现代", "未来"]
SOURCES = {"素材", "新增"}
CARD_TYPES = {"随从", "效果"}
SKILL_TRIGGERS = {"主动使用", "首次刷新时", "战斗开始时"}
RANGE = {1: (1, 2, 2, 4), 2: (2, 3, 3, 5), 3: (3, 5, 4, 7),
         4: (4, 7, 6, 9), 5: (6, 9, 8, 12), 6: (8, 12, 10, 16)}
CAP = {1: 8, 2: 10, 3: 13, 4: 17, 5: 22, 6: 26}
EXPECT_ERA = {"史前": 27, "原始": 26, "封建": 27, "现代": 36, "未来": 20}
EXPECT_TRIBE = {"学者": 28, "生物": 24, "人类": 28, "机械": 22, "植物": 18, "病毒": 16}
EXPECT_LEVEL = {1: 19, 2: 28, 3: 28, 4: 24, 5: 22, 6: 15}
MINION_FIELDS = ["id", "name", "card_type", "tribe", "subtype", "level", "era",
                 "attack", "health", "keywords", "is_scholar", "source"]
EFFECT_FIELDS = ["id", "name", "card_type", "level", "tribe", "cost", "match_text"]

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
    if "## 效果" not in text:
        return "无"
    return text.split("## 效果", 1)[1].strip()


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


def expected_attrs(doc, text):
    if doc["card_type"] == "随从":
        kw = "；".join(doc["keywords"]) if doc["keywords"] else "无"
        return {
            "id": doc["id"], "名称": doc["name"], "类型": doc["card_type"],
            "流派": doc["tribe"], "子类": doc["subtype"], "星级": str(doc["level"]),
            "时代": doc["era"], "攻击力": str(doc["attack"]),
            "生命值": str(doc["health"]), "关键词": kw,
            "学者": "是" if doc["is_scholar"] else "否", "来源": doc["source"],
        }
    return {
        "id": doc["id"], "名称": doc["name"], "类型": doc["card_type"],
        "星级": str(doc["level"]), "流派": doc["tribe"],
        "费用": str(doc["cost"]), "比赛模式替换": doc["match_text"],
    }


def check_attr_tables(raw):
    for doc in raw:
        path = ROOT / "cards" / doc["_file"]
        table = parse_attr_table(path)
        if table is None:
            fail(f"{doc['_file']}: 缺少「## 属性」表")
            continue
        expected = expected_attrs(doc, body_text(path))
        for key, value in expected.items():
            if table.get(key) != value:
                fail(f"{doc['_file']}: 属性表 {key}={table.get(key)!r}，front matter 为 {value!r}")
        extra = set(table) - set(expected)
        if extra:
            fail(f"{doc['_file']}: 属性表多出字段 {sorted(extra)}")


def keyword_cost(card):
    total = 0
    for k in card["keywords"]:
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
            total += 2
    return total


def check_hero_intros(heroes):
    for h in heroes:
        text = (ROOT / "heroes" / h["_file"]).read_text(encoding="utf-8")
        match = re.search(r"## 简介\n\n(.+?)\n", text)
        if not match:
            fail(f"{h['_file']}: 缺少「## 简介」")
        elif match.group(1).strip() != h["summary"]:
            fail(f"{h['_file']}: 简介与 front matter 不一致")


def check_cards():
    raw = load("cards", ["id", "name", "card_type", "level", "tribe"])
    for doc in raw:
        if doc["card_type"] not in CARD_TYPES:
            fail(f"{doc['_file']}: card_type 非法 {doc['card_type']!r}")
    minions = [c for c in raw if c["card_type"] == "随从"]
    effects = [c for c in raw if c["card_type"] == "效果"]

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
        if c["source"] not in SOURCES:
            fail(f"{name}: 来源非法 {c['source']!r}")
        if c["level"] not in RANGE:
            fail(f"{name}: 星级非法 {c['level']!r}")
            continue
        if (c["tribe"] == "学者") != c["is_scholar"]:
            fail(f"{name}: 流派与 is_scholar 不一致")
        if c["is_scholar"] and c["attack"] != 0:
            fail(f"{name}: 学者攻击力必须为 0")
        if c["is_scholar"]:
            continue
        lo, hi, lh, hh = RANGE[c["level"]]
        if not (lo <= c["attack"] <= hi and lh <= c["health"] <= hh):
            fail(f"{name}: 数值 {c['attack']}/{c['health']} 越出 {c['level']} 星区间")
        if c["attack"] + c["health"] + keyword_cost(c) > CAP[c["level"]]:
            fail(f"{name}: 超模")

    for c in minions:
        for k in c.get("keywords", []):
            if not KEYWORD_RE.fullmatch(k):
                fail(f"{c['name']}: 关键词非法 {k!r}")
            base = k.split()[0]
            if base in TRIBE_KEYWORD and c["tribe"] != TRIBE_KEYWORD[base]:
                fail(f"{c['name']}: 专属词 {k!r} 与流派 {c['tribe']} 越界")

    for c in effects:
        missing = [k for k in EFFECT_FIELDS if k not in c]
        if missing:
            fail(f"{c['_file']}: 效果牌缺字段 {missing}")
            continue
        name = c["name"]
        if "attack" in c or "health" in c:
            fail(f"{name}: 效果牌不得有 attack/health")
        if c["tribe"] not in TRIBES:
            fail(f"{name}: 流派非法 {c['tribe']!r}")
        if c["level"] not in RANGE:
            fail(f"{name}: 星级非法 {c['level']!r}")
        if not 0 <= c["cost"] <= 4:
            fail(f"{name}: 费用越界 {c['cost']}")

    counts = collections.Counter(c["card_type"] for c in raw)
    if counts != {"随从": 136, "效果": 30}:
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


def check_heroes():
    heroes = load("heroes", ["id", "name", "summary", "discipline", "skill_name",
                             "skill_cost", "skill_type", "skill_uses_per_turn",
                             "skill_trigger", "skill_text", "match_text"])
    if len(heroes) != 13:
        fail(f"英雄数量 {len(heroes)}，期望 13")
    for key in ("id", "name"):
        values = [h[key] for h in heroes]
        dup = [v for v, n in collections.Counter(values).items() if n > 1]
        if dup:
            fail(f"英雄 {key} 重复: {dup}")
    for h in heroes:
        if not h["summary"]:
            fail(f"{h['name']}: 简介为空")
        if not h["match_text"]:
            fail(f"{h['name']}: match_text 为空")
        if h["skill_type"] not in ("主动", "被动"):
            fail(f"{h['name']}: 技能类型非法 {h['skill_type']!r}")
        if not 0 <= h["skill_cost"] <= 2:
            fail(f"{h['name']}: 费用越界 {h['skill_cost']}")
        if h["skill_type"] == "被动" and h["skill_cost"] != 0:
            fail(f"{h['name']}: 被动技能费用必须为 0")
        if h["skill_uses_per_turn"] not in (1, 2):
            fail(f"{h['name']}: 每回合次数非法 {h['skill_uses_per_turn']!r}")
        if h["skill_trigger"] not in SKILL_TRIGGERS:
            fail(f"{h['name']}: 技能触发时机非法 {h['skill_trigger']!r}")
        if h["skill_type"] == "主动" and h["skill_trigger"] != "主动使用":
            fail(f"{h['name']}: 主动技能的触发时机必须为「主动使用」")
        if h["skill_type"] == "被动" and h["skill_trigger"] == "主动使用":
            fail(f"{h['name']}: 被动技能不能使用「主动使用」触发")
    return heroes


def check_enemy_selectors(raw):
    picks = ("随机", "最左侧", "攻击力最高", "全体")
    for doc in raw:
        text = body_text(ROOT / "cards" / doc["_file"])
        for phrase in ("对一个敌方随从", "使一个敌方随从"):
            if phrase in text and not any(pick in text for pick in picks):
                fail(f"{doc['_file']}: 单体敌方目标缺少选择器（随机/最左侧/攻击力最高/全体）")


def main():
    raw, minions, effects = check_cards()
    heroes = check_heroes()
    check_attr_tables(raw)
    check_hero_intros(heroes)
    check_enemy_selectors(raw)
    if errors:
        print(f"违规 {len(errors)}")
        for item in errors:
            print(" -", item)
        return 1
    print(f"随从 {len(minions)} 张、效果牌 {len(effects)} 张、英雄 {len(heroes)} 位，校验通过")
    print("时代", dict(collections.Counter(c["era"] for c in minions)))
    print("流派", dict(collections.Counter(c["tribe"] for c in minions)))
    print("星级", dict(collections.Counter(c["level"] for c in minions)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
