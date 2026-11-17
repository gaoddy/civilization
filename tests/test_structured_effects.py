import pathlib
import sys
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import gen_card_data  # noqa: E402


class StructuredEffectsTest(unittest.TestCase):
    def test_growth_with_condition(self):
        self.assertEqual(
            gen_card_data.structured_effects(
                "OnActivate|Growth|friendly:single:tribe=人类|"
                "times=1,timesIf=人类:2,condition=count:friendly:tribe=人类:>=3"
            ),
            [
                {
                    "trigger": "on_activate",
                    "op": "growth",
                    "selector": {
                        "kind": "side",
                        "side": "friendly",
                        "pick": "single",
                        "filters": [{"key": "tribe", "value": "人类"}],
                    },
                    "params": {
                        "times": 1,
                        "times_if": {"tribe": "人类", "times": 2},
                        "condition": {
                            "side": "friendly",
                            "filter": {"key": "tribe", "value": "人类"},
                            "compare": "ge",
                            "count": 3,
                        },
                    },
                }
            ],
        )

    def test_empty_effects(self):
        self.assertEqual(gen_card_data.structured_effects("[]"), [])

    def test_multiple_segments(self):
        effects = gen_card_data.structured_effects(
            "OnActivate|RefreshShop|self|free=true;"
            "OnActivate|GainMinion|self|from=shop,level=current,to=hand"
        )
        self.assertEqual(
            [effect["op"] for effect in effects], ["refresh_shop", "gain_minion"]
        )
        self.assertEqual(effects[0]["selector"], {"kind": "self"})
        self.assertEqual(
            effects[1]["params"],
            {"from": "shop", "level": "current", "to": "hand"},
        )

    def test_keyword_value_is_normalized(self):
        effect = gen_card_data.structured_effects(
            "OnPlayEffect|GrantKeyword|friendly:single|keyword=繁衍,value=2"
        )[0]
        self.assertEqual(
            effect["params"]["keyword"], {"keyword": "繁衍", "value": 2}
        )

    def test_invalid_dsl_raises(self):
        with self.assertRaises(ValueError):
            gen_card_data.structured_effects("OnPlay|Explode|self")

    def test_invalid_from_raises(self):
        with self.assertRaises(ValueError):
            gen_card_data.structured_effects(
                "OnActivate|GainMinion|self|from=board,level=current,to=hand"
            )


if __name__ == "__main__":
    unittest.main()
