#!/usr/bin/env python3
"""协议样例校验测试：校验器行为、类型覆盖、重复与陈旧消息样例。"""

import json
import pathlib
import sys
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

import validate_protocol as vp  # noqa: E402


def sample(direction, name):
    return json.loads((ROOT / "protocol/samples/v1" / direction / f"{name}.json").read_text())


class ValidateProtocolTest(unittest.TestCase):
    def test_valid_samples_have_no_errors(self):
        for direction in ("c2s", "s2c"):
            for path in (ROOT / "protocol/samples/v1" / direction).glob("*.json"):
                self.assertEqual(vp.validate_message(direction, json.loads(path.read_text())), [], path.name)

    def test_every_mapped_type_has_a_sample(self):
        mapping = vp.load_mapping()
        for direction in ("c2s", "s2c"):
            for name in mapping["directions"][direction]:
                self.assertTrue((ROOT / "protocol/samples/v1" / direction / f"{name}.json").exists(),
                                f"{direction}/{name}")

    def test_missing_required_field_is_reported(self):
        doc = sample("c2s", "Intent")
        del doc["payload"]["clientIntentId"]
        self.assertIn("payload.clientIntentId: 缺少必填字段", vp.validate_message("c2s", doc))

    def test_extra_field_is_reported(self):
        doc = sample("c2s", "Intent")
        doc["payload"]["unexpected"] = 1
        self.assertTrue(any("unexpected" in e for e in vp.validate_message("c2s", doc)))

    def test_unknown_type_is_reported(self):
        doc = sample("c2s", "Intent")
        doc["type"] = "NoSuchType"
        self.assertIn("type: 未知消息类型 NoSuchType", vp.validate_message("c2s", doc))

    def test_out_of_range_and_enum(self):
        doc = sample("c2s", "Intent")
        doc["payload"]["intent"]["slot"] = 6
        self.assertTrue(vp.validate_message("c2s", doc))

    def test_duplicate_and_stale_samples_are_valid_shape(self):
        self.assertEqual(vp.validate_message("c2s", sample("c2s", "IntentRetry")), [])
        self.assertEqual(vp.validate_message("s2c", sample("s2c", "StaleStateUpdate")), [])


if __name__ == "__main__":
    unittest.main()
