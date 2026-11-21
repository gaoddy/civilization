#!/usr/bin/env python3
"""线框图渲染：裁剪、标签取舍与 SVG 结构。"""

import importlib.util
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
_spec = importlib.util.spec_from_file_location(
    "ui_wireframe", ROOT / "scripts" / "ui_wireframe.py"
)
wireframe = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(wireframe)

SVG_NS = "{http://www.w3.org/2000/svg}"


def node(name, cls, rect, depth=1, text=None):
    entry = {"name": name, "class": cls, "rect": list(rect), "depth": depth}
    if text is not None:
        entry["text"] = text
    return entry


def render(*nodes, size=(1280, 720)):
    return wireframe.render_svg({"size": list(size), "nodes": list(nodes)})


def svg_root(svg):
    return ET.fromstring(svg)


class ClipTest(unittest.TestCase):
    def test_drops_nodes_entirely_outside_canvas(self):
        svg = render(node("Below", "Label", (100, 800, 200, 20), text="看不见"))
        self.assertEqual(svg_root(svg).findall(f".//{SVG_NS}text"), [])
        self.assertNotIn('y="800.0"', svg)

    def test_clamps_partially_visible_rect(self):
        svg = render(node("Corner", "PanelContainer", (-10, -10, 100, 100)))
        rect = svg_root(svg).find(f".//{SVG_NS}rect[@stroke='#c6c6c3']")
        self.assertEqual(
            (rect.get("x"), rect.get("y"), rect.get("width"), rect.get("height")),
            ("0.0", "0.0", "90.0", "90.0"),
        )

    def test_skips_texture_rect(self):
        svg = render(node("Background", "TextureRect", (0, 0, 1280, 720)))
        self.assertNotIn("Background", svg)


class LabelTest(unittest.TestCase):
    def test_auto_named_leaf_keeps_its_text(self):
        svg = render(node("@Label@4", "Label", (10, 10, 200, 20), text="请选择英雄"))
        self.assertIn("请选择英雄", svg)

    def test_auto_named_container_is_not_labelled(self):
        svg = render(node("@VBoxContainer@3", "VBoxContainer", (10, 10, 200, 200), depth=2))
        self.assertNotIn("@VBoxContainer@3", svg)

    def test_named_leaf_falls_back_to_node_name(self):
        svg = render(node("RecruitTurnLabel", "Label", (10, 10, 200, 20), depth=3))
        self.assertIn("RecruitTurnLabel", svg)

    def test_deep_container_names_are_dropped(self):
        svg = render(
            node("DeepBox", "HBoxContainer", (10, 10, 400, 100), depth=9),
        )
        self.assertNotIn("DeepBox", svg)


class StructureTest(unittest.TestCase):
    def test_output_is_parseable_svg_with_canvas_size(self):
        root = svg_root(render(node("Title", "Label", (10, 10, 200, 20), text="图鉴")))
        self.assertEqual(root.get("width"), "1280")
        self.assertEqual(root.get("height"), "720")
        self.assertEqual(root.get("viewBox"), "0 0 1280 720")

    def test_uses_reported_canvas_size(self):
        root = svg_root(render(size=(960, 540)))
        self.assertEqual((root.get("width"), root.get("height")), ("960", "540"))


if __name__ == "__main__":
    unittest.main()
