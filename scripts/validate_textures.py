#!/usr/bin/env python3
"""校验客户端纹理使用点与纹理登记册一致。

扫描 `client/**/*.gd`（排除 `client/tests/` 与 `client/.godot/`）里的纹理入口、
`res://assets/**.png` 字面量与 `CardDB.art_path` 的槽位名，要求它们都出现在
`docs/current/technical/client/textures.md`；同时要求登记册里的源码路径与静态
资源路径真实存在。任一不满足时退出码为 1。

运行：python3 scripts/validate_textures.py
"""

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
CLIENT = ROOT / "client"
DOC = ROOT / "docs" / "current" / "technical" / "client" / "textures.md"
SKIP_DIRS = {"tests", ".godot", ".import"}

# 纹理入口：命中任意一条的脚本都必须在登记册里出现。
TRIGGERS = (
    (re.compile(r"\bUIArt\.(?:texture|layer)\s*\("), "UIArt"),
    (re.compile(r"\bUIControls\.(?:background|page_surface)\s*\("), "UIControls"),
    (re.compile(r"\bCardDB\.art_path\s*\("), "CardDB.art_path"),
    (re.compile(r"\bTextureRect\.new\s*\("), "TextureRect"),
    (re.compile(r"\bset_art\s*\("), "AvatarFrame.set_art"),
)
RESOURCE_RE = re.compile(r"res://assets/[^\"'\s]+\.png")
KIND_RE = re.compile(r"\bart_path\([^)]*,\s*\"([a-z_]+)\"")
DOC_SOURCE_RE = re.compile(r"`(client/[^`\s]+\.gd)`")
DOC_RESOURCE_RE = re.compile(r"`(client/assets/[^`\s]+\.png)`")


def _repo_path(res_url):
    return "client/" + res_url.removeprefix("res://")


def _iter_scripts():
    for path in sorted(CLIENT.rglob("*.gd")):
        rel = path.relative_to(ROOT)
        if any(part in SKIP_DIRS for part in rel.parts):
            continue
        yield path, rel.as_posix()


def _scan():
    sources = {}
    resources = set()
    kinds = set()
    for path, rel in _iter_scripts():
        text = path.read_text(encoding="utf-8")
        hits = [label for pattern, label in TRIGGERS if pattern.search(text)]
        if hits:
            sources[rel] = hits
        for match in RESOURCE_RE.findall(text):
            resources.add(_repo_path(match))
        for match in KIND_RE.findall(text):
            kinds.add(match)
    return sources, resources, kinds


def _doc_paths(doc_text):
    return set(DOC_SOURCE_RE.findall(doc_text)), set(DOC_RESOURCE_RE.findall(doc_text))


def validate():
    if not DOC.is_file():
        return [f"缺少纹理登记册：{DOC.relative_to(ROOT)}"]
    doc_text = DOC.read_text(encoding="utf-8")
    sources, resources, kinds = _scan()
    doc_sources, doc_resources = _doc_paths(doc_text)
    failures = []

    for rel, hits in sorted(sources.items()):
        if rel not in doc_text:
            failures.append(f"未登记源码：{rel}（命中 {'、'.join(hits)}）")
    for path in sorted(resources):
        if path not in doc_text:
            failures.append(f"未登记资源：{path}")
    for kind in sorted(kinds):
        if kind not in doc_text:
            failures.append(f"未登记清单槽位：{kind}")

    for rel in sorted(doc_sources):
        if "*" in rel or "<" in rel:
            continue
        if not (ROOT / rel).is_file():
            failures.append(f"登记册里的源码不存在：{rel}")
    for rel in sorted(doc_resources):
        if "<" in rel or "*" in rel:
            continue
        if not (ROOT / rel).is_file():
            failures.append(f"登记册里的资源不存在：{rel}")
    return failures


def main():
    failures = validate()
    for line in failures:
        print(line)
    if failures:
        print(f"纹理登记校验失败：{len(failures)} 项", file=sys.stderr)
        return 1
    print("纹理登记校验通过")
    return 0


if __name__ == "__main__":
    sys.exit(main())
