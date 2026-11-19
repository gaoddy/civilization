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

    def test_insecure_tls_default_off(self):
        project = (CLIENT / "project.godot").read_text(encoding="utf-8")
        self.assertRegex(project, r"(?m)^allow_insecure_tls=false$")

    def test_ws_client_reads_tls_setting(self):
        source = (CLIENT / "autoload" / "ws_client.gd").read_text(encoding="utf-8")
        self.assertIn('ProjectSettings.get_setting("network/allow_insecure_tls"', source)
        self.assertIn("TLSOptions.client_unsafe()", source)

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
        # 棋盘落点、手牌落点共用 `_send_play_card`；三者之外不得构造 PlayCard。
        allowed = {"_on_board_dropped", "_on_hand_target_dropped", "_send_play_card"}
        owner = None
        offenders = []
        for number, line in enumerate(source.splitlines(), 1):
            match = re.match(r"^func (\w+)\(", line)
            if match:
                owner = match.group(1)
            if "INTENT_PLAY_CARD" in line and owner not in allowed:
                offenders.append(f"{number}: {line.strip()}（位于 {owner}）")
        self.assertEqual(offenders, [], "\n".join(offenders))

    def test_hand_card_has_no_click_callback(self):
        """HandCard 只提供拖拽数据，不接点击。"""
        source = (CLIENT / "scenes" / "hand_card.gd").read_text(encoding="utf-8")
        self.assertNotIn("pressed.connect", source)
        self.assertIn("_get_drag_data", source)

    def test_headless_driver_consumes_target_pending(self):
        """无头驱动必须先按队首待选类型分派：目标待选发 ChooseTarget，不能一律当发现。"""
        source = (CLIENT / "tests" / "headless_game.gd").read_text(encoding="utf-8")
        self.assertIn('str(choices[0].get("kind", "")) == "target"', source)
        self.assertIn("Protocol.INTENT_CHOOSE_TARGET", source)
        act = source.split("func _act(", 1)[1].split("\nfunc ", 1)[0]
        target_branch = act.index("_choose_target")
        discover_branch = act.index("Protocol.INTENT_CHOOSE_DISCOVER")
        self.assertLess(target_branch, discover_branch, "目标待选分支必须先于发现分支")


if __name__ == "__main__":
    unittest.main()
