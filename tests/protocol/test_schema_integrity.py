#!/usr/bin/env python3
"""协议 schema 完整性测试：文件可解析、消息映射完整、每个消息类型都能解析到 $def。"""

import json
import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2] / "protocol" / "schema" / "v1"

EXPECTED_C2S = {"AuthRequest", "CreateRoom", "JoinRoom", "LeaveRoom", "Ready",
                "SelectHeroCard", "Intent", "SkipAnimation", "RequestSnapshot",
                "Reconnect", "FriendRequest"}
EXPECTED_S2C = {"AuthResult", "RoomUpdate", "GameStarted", "Snapshot",
                "StateUpdate", "PhaseChanged", "TimerWarning", "CombatResult",
                "GameOver", "Error", "FriendUpdate"}


class SchemaIntegrityTest(unittest.TestCase):
    def test_all_schema_files_parse_and_declare_draft(self):
        for path in sorted(ROOT.glob("*.schema.json")):
            doc = json.loads(path.read_text())
            self.assertEqual(doc["$schema"], "https://json-schema.org/draft/2020-12/schema", path.name)
            self.assertIn("$id", doc, path.name)

    def test_messages_map_matches_expected_types(self):
        mapping = json.loads((ROOT / "messages.json").read_text())
        self.assertEqual(mapping["v"], 1)
        self.assertEqual(set(mapping["directions"]["c2s"]), EXPECTED_C2S)
        self.assertEqual(set(mapping["directions"]["s2c"]), EXPECTED_S2C)

    def test_every_message_type_resolves_to_a_def(self):
        messages = json.loads((ROOT / "messages.schema.json").read_text())
        defs = messages["$defs"]
        mapping = json.loads((ROOT / "messages.json").read_text())
        referenced = set()
        for direction in ("c2s", "s2c"):
            for name, target in mapping["directions"][direction].items():
                self.assertEqual(target, f"messages.schema.json#/$defs/{name}")
                self.assertIn(name, defs, f"{direction}.{name}")
                referenced.add(name)
        self.assertEqual(referenced, set(defs), "messages.schema.json 存在未被映射的 $def")


if __name__ == "__main__":
    unittest.main()
