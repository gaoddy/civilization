#!/usr/bin/env python3
"""跨设备一致性样式约束：页面入口、控件工厂、颜色 / 字体 / 间距令牌。"""

import pathlib
import re
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
CLIENT = ROOT / "client"

PAGES = (
    "pages/login/login.gd",
    "pages/lobby/lobby.gd",
    "pages/room/room.gd",
    "pages/game/game.gd",
    "pages/result/result.gd",
    "pages/gallery/gallery.gd",
)

SKIP_PREFIXES = ("generated/", "tests/")
COLOR_LITERAL = re.compile(r"\bColor\(")
CONTROL_NEW = re.compile(r"\b(?:Button|Label|LineEdit)\.new\(\)")
FONT_LITERAL = re.compile(
    r'add_theme_font_size_override\(\s*"font_size"\s*,\s*\d+\s*\)'
    r"|UIControls\.label\([^()\n]*,\s*\d+\s*[,)]"
)
SPACING_LITERAL = re.compile(
    r'add_theme_constant_override\(\s*"(?:separation|margin_[a-z]+)"\s*,\s*\d+\s*\)'
)


def client_sources():
    """客户端全部 GDScript，跳过生成物与测试夹具。"""
    for path in sorted(CLIENT.rglob("*.gd")):
        relative = path.relative_to(CLIENT).as_posix()
        if relative.startswith(SKIP_PREFIXES):
            continue
        yield path, relative


def function_body(lines, name):
    """取 `func <name>(...)` 的函数体行，缩进回到顶格即结束。"""
    start = None
    for index, line in enumerate(lines):
        if re.match(rf"^func {re.escape(name)}\(", line):
            start = index + 1
            break
    if start is None:
        return []
    body = []
    for line in lines[start:]:
        if line.strip() and not line[:1].isspace():
            break
        body.append(line)
    return body


def first_statement(lines):
    for line in lines:
        stripped = line.strip()
        if stripped and not stripped.startswith("#"):
            return stripped
    return ""


class ClientStyleTest(unittest.TestCase):
    def test_pages_call_safe_area_first(self):
        offenders = []
        for relative in PAGES:
            path = CLIENT / relative
            self.assertTrue(path.is_file(), f"缺少页面脚本 {relative}")
            body = function_body(path.read_text(encoding="utf-8").splitlines(), "_ready")
            statement = first_statement(body)
            if statement != "UILayout.safe_area(self)":
                offenders.append(f"{relative}: {statement or '<空>'}")
        self.assertEqual(
            offenders,
            [],
            "页面 `_ready()` 第一句必须是 `UILayout.safe_area(self)`：\n" + "\n".join(offenders),
        )

    def test_colors_use_tokens(self):
        offenders = []
        for path, relative in client_sources():
            if relative == "ui/theme.gd":
                continue
            for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                if COLOR_LITERAL.search(line):
                    offenders.append(f"{relative}:{number}: {line.strip()}")
        self.assertEqual(offenders, [], "颜色必须使用 color-* 令牌：\n" + "\n".join(offenders))

    def test_controls_use_factories(self):
        offenders = []
        for path, relative in client_sources():
            if relative == "ui/controls.gd":
                continue
            for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                if CONTROL_NEW.search(line):
                    offenders.append(f"{relative}:{number}: {line.strip()}")
        self.assertEqual(
            offenders,
            [],
            "按钮 / 标签 / 输入框必须通过 UIControls 工厂创建：\n" + "\n".join(offenders),
        )

    def test_font_sizes_use_tokens(self):
        offenders = []
        for path, relative in client_sources():
            if relative == "ui/controls.gd":
                continue
            for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                if FONT_LITERAL.search(line):
                    offenders.append(f"{relative}:{number}: {line.strip()}")
        self.assertEqual(offenders, [], "字体大小必须使用 font-* 令牌：\n" + "\n".join(offenders))

    def test_spacing_uses_tokens(self):
        offenders = []
        for path, relative in client_sources():
            for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                if SPACING_LITERAL.search(line):
                    offenders.append(f"{relative}:{number}: {line.strip()}")
        self.assertEqual(offenders, [], "间距必须使用 space-* 令牌：\n" + "\n".join(offenders))


if __name__ == "__main__":
    unittest.main()
