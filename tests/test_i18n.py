import csv, io, pathlib, re, sys, unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import gen_card_data, gen_text, i18n_terms, validate_docs

_CJK = re.compile(r"[一-鿿]")


def _keys(text):
    rows = list(csv.reader(io.StringIO(text)))
    return [row[0] for row in rows[1:]]


def _strip_comment(line):
    """去掉字符串外的 `#` 注释，保留引号内与转义后的井号。"""
    quote = None
    escaped = False
    for index, char in enumerate(line):
        if quote is not None:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == quote:
                quote = None
        elif char in ("'", '"'):
            quote = char
        elif char == "#":
            return line[:index]
    return line


def _string_literals(line):
    """依次扫描单引号、双引号与转义，返回该行的字符串字面量内容。"""
    literals = []
    quote = None
    escaped = False
    start = 0
    for index, char in enumerate(line):
        if quote is not None:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == quote:
                literals.append(line[start:index])
                quote = None
        elif char in ("'", '"'):
            quote = char
            start = index + 1
    return literals


class TextTableTest(unittest.TestCase):
    def test_csv_header_and_unique_keys(self):
        rows = list(csv.reader(io.StringIO(gen_text.build(ROOT))))
        self.assertEqual(rows[0], ["keys", "zh_CN"])
        keys = [row[0] for row in rows[1:]]
        self.assertEqual(keys, sorted(keys))
        self.assertEqual(len(keys), len(set(keys)))
        self.assertEqual([row for row in rows[1:] if not row[1]], [])

    def test_content_keys_cover_cards_and_skills(self):
        keys = _keys(gen_text.build(ROOT))
        self.assertIn("card.adam-smith.name", keys)
        self.assertIn("skill.division-of-labor.text", keys)
        self.assertIn("card.seedling.name", keys)

    def test_generated_prefix_keys_are_replaced(self):
        existing = {"ui.test.only": "只用于测试", "card.old.name": "旧卡"}
        merged = gen_text.merge(existing, {"card.new.name": "新卡"})
        self.assertEqual(merged["ui.test.only"], "只用于测试")
        self.assertNotIn("card.old.name", merged)

    def test_unknown_prefix_is_rejected(self):
        with self.assertRaises(ValueError):
            gen_text.merge({"page.title": "标题"}, {})

    def test_no_cjk_runtime_literals(self):
        offenders = []
        for path in sorted((ROOT / "client").rglob("*.gd")):
            rel = path.relative_to(ROOT / "client").as_posix()
            if rel.startswith(("generated/", "tests/")):
                continue
            for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                if re.search(r"\b(push_error|push_warning|print|printerr)\s*\(", line):
                    continue
                for literal in _string_literals(_strip_comment(line)):
                    if _CJK.search(literal):
                        offenders.append(f"{rel}:{number}: {literal}")
        self.assertEqual(offenders, [], "\n".join(offenders))

    def test_all_error_codes_have_text(self):
        protocol = (ROOT / "client" / "generated" / "protocol.gd").read_text(encoding="utf-8")
        codes = re.findall(r'^const ERROR_[A-Z0-9_]+ := "([A-Z0-9_]+)"$', protocol, re.M)
        keys = _keys((ROOT / "client/text/zh_CN.csv").read_text(encoding="utf-8"))
        self.assertEqual([f"error.{code}" for code in codes if f"error.{code}" not in keys], [])

    def test_server_error_ids_exist(self):
        keys = _keys((ROOT / "client/text/zh_CN.csv").read_text(encoding="utf-8"))
        missing = []
        for path in sorted((ROOT / "server" / "crates").rglob("*.rs")):
            for match in re.finditer(r'"(error\.[A-Za-z0-9_.]+)"', path.read_text(encoding="utf-8")):
                if match.group(1) not in keys:
                    missing.append(f"{path.name}: {match.group(1)}")
        self.assertEqual(missing, [])

    def test_csv_keys_are_ascii_ids(self):
        """文本表的 key 必须是不含 CJK 的 ASCII 标识，避免误把文案写进 key 列。"""
        keys = _keys((ROOT / "client/text/zh_CN.csv").read_text(encoding="utf-8"))
        offenders = [k for k in keys if _CJK.search(k) or not re.fullmatch(r"[A-Za-z0-9._-]+", k)]
        self.assertEqual(offenders, [])

    def test_tr_keys_exist(self):
        keys = set(_keys((ROOT / "client/text/zh_CN.csv").read_text(encoding="utf-8")))
        missing = []
        scanned_files = 0
        scanned_keys = 0
        for path in sorted((ROOT / "client").rglob("*.gd")):
            rel = path.relative_to(ROOT / "client").as_posix()
            if rel.startswith(("generated/", "tests/")):
                continue
            scanned_files += 1
            for line in path.read_text(encoding="utf-8").splitlines():
                found = re.findall(
                    r'\b(?:tr|TranslationServer\.translate)\(\s*"([^"]+)"',
                    _strip_comment(line),
                )
                scanned_keys += len(found)
                for key in found:
                    if "%" in key:
                        continue
                    if key not in keys:
                        missing.append(f"{rel}: {key}")
        # 防止格式变化（例如改用单引号或换行）后正则静默匹配不到而假通过。
        self.assertGreater(scanned_files, 0)
        self.assertGreater(scanned_keys, 0)
        self.assertEqual(missing, [])

    def test_every_enum_value_has_text(self):
        keys = set(_keys((ROOT / "client/text/zh_CN.csv").read_text(encoding="utf-8")))
        nodes, playstyles = validate_docs.parse_taxonomy()
        for group, values in (
            ("card_type", i18n_terms.CARD_TYPES),
            ("playstyle", [pid for pid, _ in playstyles]),
            ("taxon", list(nodes)),
            ("era", i18n_terms.ERAS),
            ("evolve", i18n_terms.EVOLVE_CHOICES),
            ("stat_layer", i18n_terms.STAT_LAYERS),
            ("keyword", i18n_terms.KEYWORD_BASES),
        ):
            for value in values:
                self.assertIn(f"enum.{group}.{value}", keys)
        labels = validate_docs.card_enum_labels(
            validate_docs.load("cards", ["id", "card_type", "taxon", "era"])
        )
        for slug in labels["subtype"]:
            self.assertIn(f"enum.subtype.{slug}", keys)

    def test_content_keys_cover_all_entities(self):
        keys = set(_keys((ROOT / "client/text/zh_CN.csv").read_text(encoding="utf-8")))
        content = ROOT / "docs/versions/v0.1/product/content"
        for doc in gen_card_data.load_docs(content, "cards"):
            self.assertIn(f"card.{doc['id']}.name", keys)
        for doc in gen_card_data.load_docs(content, "skills"):
            self.assertIn(f"skill.{doc['id']}.text", keys)

    def test_csv_has_no_literal_newlines(self):
        raw = (ROOT / "client/text/zh_CN.csv").read_text(encoding="utf-8")
        self.assertTrue(raw.endswith("\n"))
        self.assertEqual(len(raw.splitlines()), len(list(csv.reader(io.StringIO(raw)))))

    def test_generated_data_has_no_cjk(self):
        for base in (ROOT / "client/generated", ROOT / "server/data/v0.1"):
            for path in sorted(base.glob("*.json")):
                self.assertIsNone(_CJK.search(path.read_text(encoding="utf-8")), path.name)
