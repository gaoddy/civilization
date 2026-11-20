#!/usr/bin/env python3
"""审计技能 effects 字段的 DSL 用法，输出「触发 × 动作」矩阵与不合规项。

只读技能目录，不修改数据；语法解析复用 scripts/validate_docs.py。
审计只报告违规，不新增产品规则。
"""

import collections
import dataclasses
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import skill_templates
import validate_docs

ROOT = pathlib.Path(__file__).resolve().parent.parent
DEFAULT_SKILLS = ROOT / "docs" / "versions" / "v0.1" / "product" / "content" / "skills"

# 带层数/数值的关键词：赋予时必须带 value，其余关键词禁止 value。
# breed / assemble 的层数写在 keyword 上（如「breed 2」），contagion 的层数写在 value 上。
VALUE_KEYWORDS = {"breed", "assemble", "contagion", "armor"}
# when 限定事件对象；OnDeath 同为事件类触发（validate_docs.EVENT_TRIGGERS 未含 OnDeath）。
WHEN_TRIGGERS = validate_docs.EVENT_TRIGGERS | {"OnDeath"}
# 筛选白名单以 validate_docs 为唯一来源，这里只读不另写一份。
FILTER_KEYS = validate_docs.FILTER_KEYS
# 分解的生命值口径；duration 只接受 validate_docs.DURATIONS（permanent / combat）。
DECOMPOSE_BASES = validate_docs.DECOMPOSE_BASES
# 新机制原语的目标侧约定（技术方案 §3）；选择器白名单仍以 validate_docs 为准。
ENEMY_TARGET_OPS = {"Devour", "Detonate", "Convert"}
FRIENDLY_OR_ENEMY_TARGET_OPS = {"Mimic"}
SELF_TARGET_OPS = {"Revive", "Recall", "Salvage", "Latent"}


@dataclasses.dataclass
class Report:
    skill_count: int
    matrix: dict
    violations: list

    def to_json(self):
        return json.dumps(
            {
                "skill_count": self.skill_count,
                "matrix": [
                    {"trigger": trigger, "op": op, "count": count}
                    for (trigger, op), count in sorted(self.matrix.items())
                ],
                "violations": self.violations,
            },
            ensure_ascii=False,
            indent=2,
        )


def check_effect(trigger, op, params, selector=""):
    """返回单段效果的审计违规消息（不含技能 id 前缀）。"""
    violations = []
    if op in ENEMY_TARGET_OPS and not selector.startswith("enemy:"):
        violations.append(f"{op} 必须以 enemy 选择器指定目标，实为 {selector!r}")
    if op in FRIENDLY_OR_ENEMY_TARGET_OPS and not selector.startswith(("enemy:", "friendly:")):
        violations.append(f"{op} 必须以 enemy 或 friendly 选择器指定目标，实为 {selector!r}")
    if op in SELF_TARGET_OPS and selector != "self":
        violations.append(f"{op} 只能以 self 为目标，实为 {selector!r}")
    if op == "Copy" and params.get("from") == "hand" \
            and not selector.startswith("target:hand"):
        violations.append(
            f"Copy 从手牌复制必须用 target:hand 选择器指定目标，实为 {selector!r}")
    if op == "Recall" and params.get("to") != "hand":
        violations.append(f"Recall 的 to 必须为 hand，实为 {params.get('to')!r}")
    if "timesIf" in params and (op != "Growth" or "times" not in params):
        violations.append("timesIf 只能与 Growth 的 times 同时出现")
    if "when" in params and trigger not in WHEN_TRIGGERS:
        violations.append(f"when 只能用于事件类触发，不能用于 {trigger}")
    if op == "GrantKeyword":
        words = params.get("keyword", "").split()
        base = words[0] if words else ""
        has_layer = len(words) > 1 or "value" in params
        if base in VALUE_KEYWORDS:
            if not has_layer:
                violations.append(f"GrantKeyword 赋予 {base} 必须带层数或 value")
        elif "value" in params:
            violations.append(f"GrantKeyword 的 {base or '空'} 关键词禁止 value")
    if op == "Summon":
        present = [key for key in ("token", "card") if key in params]
        if len(present) != 1:
            violations.append("Summon 必须且只能有 token 或 card 之一")
    if op == "Discover":
        if "pool" not in params or "choose" not in params:
            violations.append("Discover 必须同时有 pool 与 choose")
        elif params["choose"] != "1":
            violations.append(f"Discover 的 choose 必须为 1，实为 {params['choose']!r}")
    if op == "GainMinion":
        missing = [key for key in ("level", "to") if key not in params]
        if missing:
            violations.append(f"GainMinion 缺少参数 {missing}")
    if "timesIf" in params:
        for message in validate_docs.times_if_errors(params["timesIf"]):
            violations.append(message)
    if op == "Decompose":
        if params.get("basis") not in DECOMPOSE_BASES:
            violations.append(f"Decompose 的 basis 必须为 {sorted(DECOMPOSE_BASES)} 之一，实为 {params.get('basis')!r}")
        if params.get("duration") not in validate_docs.DURATIONS:
            violations.append(
                f"Decompose 的 duration 必须为 {sorted(validate_docs.DURATIONS)} 之一，实为 {params.get('duration')!r}"
            )
    return violations


def audit(skills_dir):
    """审计技能目录，返回 Report。"""
    matrix = collections.Counter()
    violations = []
    docs = []
    for path in sorted(pathlib.Path(skills_dir).glob("*.md")):
        if path.name == "README.md":
            continue
        try:
            doc = validate_docs.parse_front_matter(path)
        except (OSError, ValueError) as exc:
            violations.append(f"{path.stem}: {exc}")
            continue
        docs.append((str(doc.get("id", path.stem)), doc))
    docs.sort(key=lambda item: item[0])
    renderer = None
    for skill_id, doc in docs:
        if "template" not in doc:
            violations.append(f"{skill_id}: 缺少 template 字段")
            continue
        if renderer is None:
            renderer = skill_templates.Renderer(pathlib.Path(skills_dir).parent)
        try:
            segments = renderer.instance_segments(doc)
        except ValueError as exc:
            violations.append(f"{skill_id}: {exc}")
            continue
        raw_effects = ";".join(segments) if segments else "[]"
        effects, errors = validate_docs.parse_effects(raw_effects)
        for message in errors:
            violations.append(f"{skill_id}: {message}")
        for effect in effects:
            trigger, op = effect["trigger"], effect["op"]
            matrix[(trigger, op)] += 1
            for message in check_effect(trigger, op, effect["params"], effect["selector"]):
                violations.append(f"{skill_id}: {message}")
    return Report(
        skill_count=len(docs),
        matrix=dict(sorted(matrix.items())),
        violations=violations,
    )


def main(argv=None):
    argv = sys.argv if argv is None else argv
    skills_dir = pathlib.Path(argv[1]) if len(argv) > 1 else DEFAULT_SKILLS
    report = audit(skills_dir)
    if report.violations:
        print(f"违规 {len(report.violations)}")
        for item in report.violations:
            print(" -", item)
        return 1
    print(f"技能 {report.skill_count} 个，效果 {sum(report.matrix.values())} 段")
    print("触发 × 动作：")
    for (trigger, op), count in sorted(report.matrix.items()):
        print(f"  {trigger} × {op}: {count}")
    print("审计通过")
    return 0


if __name__ == "__main__":
    sys.exit(main())
