#!/usr/bin/env python3
"""客户端边界断言：不引用规则实现，只通过服务端视图驱动界面。"""

import pathlib
import re
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
CLIENT = ROOT / "client"
FORBIDDEN = re.compile(r"rule[-_]engine|resolve_combat|start_game|cargo|server/data")


class ClientBoundaryTest(unittest.TestCase):
    def test_client_exists(self):
        self.assertTrue((CLIENT / "project.godot").is_file())

    def test_autoloads_declared(self):
        project = (CLIENT / "project.godot").read_text(encoding="utf-8")
        for name in ("GameStore", "Net", "CardDB"):
            self.assertRegex(project, rf"(?m)^{name}=\"")

    def test_no_rule_engine_reference(self):
        offenders = []
        for path in sorted(CLIENT.rglob("*.gd")):
            for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                if FORBIDDEN.search(line):
                    offenders.append(f"{path.relative_to(ROOT)}:{number}: {line.strip()}")
        self.assertEqual(offenders, [], "\n".join(offenders))

    def test_protocol_literals_use_generated_constants(self):
        """客户端发送协议时不得手写字面量，必须引用生成的 Protocol 常量。"""
        offenders = []
        for path in sorted(CLIENT.rglob("*.gd")):
            relative = path.relative_to(CLIENT).as_posix()
            if relative.startswith("generated/") or relative in ("tests/fixture.gd", "tests/ui_smoke.gd"):
                continue
            for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                if re.search(r'Net\.send\("', line) or re.search(r'"kind"\s*:\s*"', line):
                    offenders.append(f"{path.relative_to(ROOT)}:{number}: {line.strip()}")
                if re.search(r"(?:\"|')(Lobby|Recruit|Combat|Settle|GameOver)(?:\"|')", line):
                    offenders.append(f"{path.relative_to(ROOT)}:{number}: {line.strip()}")
        self.assertEqual(offenders, [], "\n".join(offenders))


    def test_play_card_only_from_drag(self):
        """出牌只有拖拽一条路径：手牌没有点击回调，PlayCard 只在落点回调里构造。"""
        source = (CLIENT / "scenes" / "game.gd").read_text(encoding="utf-8")
        self.assertNotIn("_on_play", source)
        owner = None
        offenders = []
        for number, line in enumerate(source.splitlines(), 1):
            match = re.match(r"^func (\w+)\(", line)
            if match:
                owner = match.group(1)
            if "INTENT_PLAY_CARD" in line and owner != "_on_board_dropped":
                offenders.append(f"{number}: {line.strip()}（位于 {owner}）")
        self.assertEqual(offenders, [], "\n".join(offenders))

    def test_hand_card_has_no_click_callback(self):
        """HandCard 只提供拖拽数据，不接点击。"""
        source = (CLIENT / "scenes" / "hand_card.gd").read_text(encoding="utf-8")
        self.assertNotIn("pressed.connect", source)
        self.assertIn("_get_drag_data", source)


if __name__ == "__main__":
    unittest.main()
