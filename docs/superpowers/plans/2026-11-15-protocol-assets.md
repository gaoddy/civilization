# v0.1 协议资产实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 产出 v0.1 通信协议的独立 IDL、样例、生成与校验脚本、人工规则文档，并让现有基线文档与之一致。

**Architecture:** JSON Schema（draft 2020-12）是唯一契约来源，放在仓库根 `protocol/schema/v1/`；样例按方向分目录；两个 Python 标准库脚本分别生成 GDScript 常量与字段表，`--check` 模式用于漂移检测；`make protocol-check` 汇总全部校验。本轮不建 `server/` 或 `client/` 工程。

**Tech Stack:** Python 3 标准库（现有脚本与测试均只用标准库）、unittest、GNU Make、JSON Schema draft 2020-12、GDScript 文本生成。

**Spec:** `docs/superpowers/specs/2026-11-15-protocol-design.md`

## Global Constraints

- 所有 Python 脚本与测试只使用标准库；测试运行命令为 `python3 -m unittest discover -s tests -v`。
- 协议版本 `v` 固定为整数 `1`；信封字段与消息清单以设计文档 §2、§3、§4、§5、§10 为准。
- 生成物必须随源提交，生成结果必须确定（同一输入两次运行逐字节相同）。
- 未知字段在运行时被忽略；但 schema 一律 `additionalProperties: false`，新增字段必须同时改 schema。
- 文档中不得再出现 `StateDelta`；统一为 `StateUpdate`。
- 不创建 `server/`、`client/` 工程目录；Rust 类型生成本轮不执行。
- 提交按任务主题拆分；每个任务结束时工作区只含本任务改动。

## Review Focus

- **每个消息类型都有样例**：`messages.json` 声明了类型，`samples/v1/` 必须有对应文件；漏一个，契约测试就存在盲区。对应 Task 2 的覆盖测试。
- **生成物漂移**：手改 `protocol.gd` 或 `protocol-messages.md` 后必须被 `--check` 抓住；漏检会让文档与 schema 悄悄分叉。对应 Task 3、4 的 `--check` 测试。
- **未知字段与未知类型**：运行时必须忽略而不是报错；schema 严格只作用于开发期样例。对应 Task 2 的额外字段测试与 Task 6 的协议文档断言。
- **文档链接与命名**：改了 `StateDelta` 却漏改某一处，读者会按旧名实现。对应 Task 7 的全局一致性测试。
- **重复与陈旧消息**：`clientIntentId` 重复、`seq ≤ 当前` 的语义只写在文档，没有样例覆盖；至少要有重复 `Intent` 与陈旧 `StateUpdate` 样例。对应 Task 2。

---

## File Structure

```
protocol/
  schema/v1/
    envelope.schema.json      # ClientEnvelope / ServerEnvelope
    errors.schema.json        # ErrorCode 枚举与 ErrorMessage
    intents.schema.json       # 9 种意图的 kind 判别联合
    view.schema.json          # PlayerView、ShopSlot、HandCard、BoardMinion、PlayerSummary、Affordances、GameEvent
    messages.schema.json      # 全部消息载荷 $defs
    messages.json             # type → 方向 → $defs 引用
  samples/v1/
    c2s/*.json                # 每个上行消息至少一份
    s2c/*.json                # 每个下行消息至少一份
  generated/
    protocol.gd               # 生成：常量
scripts/
  validate_protocol.py        # 样例校验与类型覆盖检查
  gen_protocol_gdscript.py    # 生成 protocol.gd
  gen_protocol_docs.py        # 生成 protocol-messages.md
tests/protocol/
  test_schema_integrity.py
  test_validate_protocol.py
  test_gen_protocol_gdscript.py
  test_gen_protocol_docs.py
  test_docs_consistency.py
  test_docs_links.py
Makefile                      # test / protocol / protocol-check
docs/current/technical/architecture/
  protocol.md                 # 人工规则文档
  protocol-messages.md        # 生成字段表
docs/superpowers/specs/2026-11-15-protocol-design.md  # 已存在
```

---

### Task 1: 协议 schema 骨架与消息映射

**Files:**
- Create: `protocol/schema/v1/envelope.schema.json`
- Create: `protocol/schema/v1/errors.schema.json`
- Create: `protocol/schema/v1/intents.schema.json`
- Create: `protocol/schema/v1/view.schema.json`
- Create: `protocol/schema/v1/messages.schema.json`
- Create: `protocol/schema/v1/messages.json`
- Test: `tests/protocol/test_schema_integrity.py`

**Interfaces:**
- Produces: `protocol/schema/v1/messages.json` 的结构 `{"v": 1, "directions": {"c2s": {type: "$defs 名"}, "s2c": {...}}}`，供 Task 2–5 读取。
- Produces: 全部载荷定义位于 `messages.schema.json#/$defs/`，共享类型位于 `view.schema.json#/$defs/`、`intents.schema.json#/$defs/`、`errors.schema.json#/$defs/`。

- [ ] **Step 1: 写失败的完整性测试**

```python
# tests/protocol/test_schema_integrity.py
import json, pathlib, unittest

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
```

- [ ] **Step 2: 运行测试确认失败**

Run: `python3 -m unittest tests.protocol.test_schema_integrity -v`
Expected: FAIL，`FileNotFoundError` 或断言失败。

- [ ] **Step 3: 写五个 schema 与映射**

按设计文档填写，不发明字段：

- `envelope.schema.json`：`$defs.ClientEnvelope` 与 `$defs.ServerEnvelope`，均 `additionalProperties: false`。必填 `v`(const 1)、`id`、`type`、`ts`(format date-time)、`payload`；可选 `roomId`、`playerId`、`replyTo`、`seq`（仅 Server 必填）。
- `errors.schema.json`：`$defs.ErrorCode` 为设计文档 §10 全量枚举的 `enum`；`$defs.ErrorMessage` 为 `{code, message, clientIntentId?, retryable}`。
- `intents.schema.json`：`$defs.Intent` 为 `oneOf`，每支含 `kind` 的 `const` 与设计文档 §5 的参数，`additionalProperties: false`。
- `view.schema.json`：`PlayerView`、`Phase`（枚举 `Lobby/Recruit/Combat/Settle/GameOver`）、`ShopSlot`、`HandCard`、`BoardMinion`、`PlayerSummary`、`Affordances`、`GameEvent`（`kind` 枚举：`MinionBought`、`MinionSold`、`ShopRolled`、`EraUpgraded`、`TripleFormed`、`CardDiscovered`、`DamageTaken`、`PlayerEliminated`）。`ShopSlot` 的槽位索引用 `minimum: 0` 与 `maximum: 5`。
- `messages.schema.json`：22 个 `$defs`，名称与 `messages.json` 完全一致；`Intent` 载荷为 `{clientIntentId, intent: intents.schema.json#/$defs/Intent}`。
- `messages.json`：按测试里的 `EXPECTED_C2S` / `EXPECTED_S2C` 逐项映射。

- [ ] **Step 4: 运行测试确认通过**

Run: `python3 -m unittest tests.protocol.test_schema_integrity -v`
Expected: PASS（3 个测试）。

- [ ] **Step 5: 提交**

```bash
git add protocol/schema/v1 tests/protocol/test_schema_integrity.py
git commit -m "feat(protocol): 新增 v1 协议 schema 与消息映射"
```

---

### Task 2: 样例与标准库校验器

**Files:**
- Create: `scripts/validate_protocol.py`
- Create: `protocol/samples/v1/c2s/*.json`（11 个）
- Create: `protocol/samples/v1/s2c/*.json`（11 个）
- Test: `tests/protocol/test_validate_protocol.py`

**Interfaces:**
- Produces: `validate_message(direction: str, doc: dict) -> list[str]`，方向为 `"c2s"` 或 `"s2c"`，返回人类可读错误列表，空列表表示通过。
- Produces: CLI `python3 scripts/validate_protocol.py`，失败时退出码 1 并逐条打印错误；成功退出码 0。
- Produces: `load_mapping() -> dict` 与 `_resolve(ref: str) -> dict`，供生成器复用（Task 3、4 直接 import）。

- [ ] **Step 1: 写失败的校验器测试**

```python
# tests/protocol/test_validate_protocol.py
import json, pathlib, sys, unittest
ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import validate_protocol as vp

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
                self.assertTrue((ROOT / "protocol/samples/v1" / direction / f"{name}.json").exists(), f"{direction}/{name}")

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
```

- [ ] **Step 2: 运行确认失败**

Run: `python3 -m unittest tests.protocol.test_validate_protocol -v`
Expected: FAIL，`ModuleNotFoundError: validate_protocol`。

- [ ] **Step 3: 实现校验器与样例**

`validate_protocol.py` 只支持本项目使用的子集：`$ref`（仅 `文件#/$defs/名字` 与 `#/$defs/名字`）、`type`、`required`、`properties`、`additionalProperties`(bool)、`items`、`enum`、`const`、`oneOf`、`minimum`、`maximum`、`format: date-time`（用 `datetime.fromisoformat` 校验）。错误消息统一为 `<json路径>: <原因>`，路径从 `payload.` 起，信封字段直接写字段名。

样例命名与内容：

| 方向 | 文件 | 要点 |
| --- | --- | --- |
| c2s | `AuthRequest.json` | `payload:{phone, code}` |
| c2s | `CreateRoom.json` / `JoinRoom.json` / `LeaveRoom.json` / `Ready.json` / `SelectHeroCard.json` | 最小合法载荷 |
| c2s | `Intent.json` | `payload:{clientIntentId:"ci_1", intent:{kind:"BuyMinion", slot:2}}` |
| c2s | `IntentRetry.json` | 与上一条完全相同的 `clientIntentId`，验证重复意图 |
| c2s | `IntentEndRecruit.json` | `kind:"EndRecruit"`，验证同一 `type` 的第二种意图 |
| c2s | `SkipAnimation.json` / `RequestSnapshot.json` / `Reconnect.json` / `FriendRequest.json` | 最小合法载荷 |
| s2c | `AuthResult.json`、`RoomUpdate.json`、`GameStarted.json`、`Snapshot.json`、`StateUpdate.json`、`PhaseChanged.json`、`TimerWarning.json`、`CombatResult.json`、`GameOver.json`、`Error.json`、`FriendUpdate.json` | 覆盖设计文档 §3–§5 的全部字段；`seq` 从 1 起 |
| s2c | `StaleStateUpdate.json` | 与 `StateUpdate.json` 同结构，`seq` 更小，验证陈旧消息形状合法 |

- [ ] **Step 4: 运行确认通过**

Run: `python3 -m unittest tests.protocol.test_validate_protocol -v`
Expected: PASS（7 个测试）。

- [ ] **Step 5: 提交**

```bash
git add scripts/validate_protocol.py protocol/samples tests/protocol/test_validate_protocol.py
git commit -m "feat(protocol): 新增协议样例与标准库校验器"
```

---

### Task 3: GDScript 常量生成器

**Files:**
- Create: `scripts/gen_protocol_gdscript.py`
- Create: `protocol/generated/protocol.gd`（由脚本生成）
- Test: `tests/protocol/test_gen_protocol_gdscript.py`

**Interfaces:**
- Consumes: `validate_protocol.load_mapping()`、`validate_protocol._resolve()`。
- Produces: `render(schema_dir: pathlib.Path, mapping: dict) -> str`；CLI 支持 `--check`（有差异退出 1，无差异退出 0）与默认覆写。
- Produces: 生成文件 `protocol/generated/protocol.gd`，首行 `class_name Protocol`。

- [ ] **Step 1: 写失败的生成器测试**

```python
# tests/protocol/test_gen_protocol_gdscript.py
import pathlib, subprocess, sys, tempfile, unittest
ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import gen_protocol_gdscript as gen

class GenGdscriptTest(unittest.TestCase):
    def test_render_is_deterministic_and_contains_constants(self):
        first = gen.render(ROOT / "protocol/schema/v1", gen.load_mapping())
        self.assertEqual(first, gen.render(ROOT / "protocol/schema/v1", gen.load_mapping()))
        self.assertIn("const PROTOCOL_VERSION := 1", first)
        for expected in ('const MSG_AUTH_REQUEST := "AuthRequest"',
                         'const MSG_STATE_UPDATE := "StateUpdate"',
                         'const INTENT_BUY_MINION := "BuyMinion"',
                         'const ERROR_NOT_ENOUGH_GOLD := "NOT_ENOUGH_GOLD"',
                         'const PHASE_RECRUIT := "Recruit"',
                         'const EVENT_TRIPLE_FORMED := "TripleFormed"'):
            self.assertIn(expected, first)

    def test_check_mode_detects_drift(self):
        result = subprocess.run([sys.executable, str(ROOT / "scripts/gen_protocol_gdscript.py"), "--check"],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_check_mode_fails_on_different_content(self):
        with tempfile.TemporaryDirectory() as tmp:
            target = pathlib.Path(tmp) / "protocol.gd"
            target.write_text("class_name Protocol\n")
            result = subprocess.run([sys.executable, str(ROOT / "scripts/gen_protocol_gdscript.py"),
                                     "--check", "--output", str(target)],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 1)
```

- [ ] **Step 2: 运行确认失败**

Run: `python3 -m unittest tests.protocol.test_gen_protocol_gdscript -v`
Expected: FAIL，`ModuleNotFoundError`。

- [ ] **Step 3: 实现生成器并产出生成物**

输出分区顺序固定：版本 → 消息类型（`MSG_*`，按字母序）→ 意图（`INTENT_*`）→ 错误码（`ERROR_*`）→ 阶段（`PHASE_*`，按 `Lobby/Recruit/Combat/Settle/GameOver` 顺序）→ 事件（`EVENT_*`）→ `PlayerView` 字段名（`FIELD_*`）。常量名按 snake→UPPER_SNAKE 规则从类型名转换；每个分区前加一行注释。脚本支持 `--output PATH`，默认 `protocol/generated/protocol.gd`。

- [ ] **Step 4: 生成并运行测试**

Run: `python3 scripts/gen_protocol_gdscript.py && python3 -m unittest tests.protocol.test_gen_protocol_gdscript -v`
Expected: PASS（3 个测试）。

- [ ] **Step 5: 提交**

```bash
git add scripts/gen_protocol_gdscript.py protocol/generated/protocol.gd tests/protocol/test_gen_protocol_gdscript.py
git commit -m "feat(protocol): 生成 GDScript 协议常量"
```

---

### Task 4: 字段表生成器

**Files:**
- Create: `scripts/gen_protocol_docs.py`
- Create: `docs/current/technical/architecture/protocol-messages.md`
- Test: `tests/protocol/test_gen_protocol_docs.py`

**Interfaces:**
- Consumes: `validate_protocol.load_mapping()`、`_resolve()`。
- Produces: `render(schema_dir: pathlib.Path, mapping: dict) -> str`；CLI 同样支持 `--check`、`--output`。
- Produces: `protocol-messages.md`，开头注明"本文件由 `scripts/gen_protocol_docs.py` 生成，不要手改"。

- [ ] **Step 1: 写失败的测试**

```python
# tests/protocol/test_gen_protocol_docs.py
import pathlib, subprocess, sys, tempfile, unittest
ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import gen_protocol_docs as gen

class GenDocsTest(unittest.TestCase):
    def test_render_contains_every_type_and_required_fields(self):
        text = gen.render(ROOT / "protocol/schema/v1", gen.load_mapping())
        self.assertEqual(text, gen.render(ROOT / "protocol/schema/v1", gen.load_mapping()))
        for name in ("AuthRequest", "StateUpdate", "CombatResult", "Error"):
            self.assertIn(f"### {name}", text)
        self.assertIn("| 字段 | 类型 | 必填 | 说明 |", text)
        self.assertIn("clientIntentId", text)

    def test_check_mode_passes_on_committed_file(self):
        result = subprocess.run([sys.executable, str(ROOT / "scripts/gen_protocol_docs.py"), "--check"],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
```

- [ ] **Step 2: 运行确认失败**

Run: `python3 -m unittest tests.protocol.test_gen_protocol_docs -v`
Expected: FAIL。

- [ ] **Step 3: 实现生成器并产出字段表**

每个消息一节，表格列固定为「字段 / 类型 / 必填 / 说明」，说明取 schema 的 `description`；嵌套对象用缩进路径（如 `payload.intent.slot`）或独立子表，二者择一但全文件一致。类型列：`string`、`integer`、`boolean`、`object`、`array<object>`、`enum`、引用名。`$ref` 解析失败时报错退出 1。

- [ ] **Step 4: 生成并运行测试**

Run: `python3 scripts/gen_protocol_docs.py && python3 -m unittest tests.protocol.test_gen_protocol_docs -v`
Expected: PASS（2 个测试）。

- [ ] **Step 5: 提交**

```bash
git add scripts/gen_protocol_docs.py docs/current/technical/architecture/protocol-messages.md tests/protocol/test_gen_protocol_docs.py
git commit -m "feat(protocol): 生成协议字段表"
```

---

### Task 5: Make 入口与生成一致性

**Files:**
- Create: `Makefile`
- Test: `tests/protocol/test_make_targets.py`

**Interfaces:**
- Produces: `make test`、`make protocol`、`make protocol-check` 三个目标；`protocol-check` 不修改工作区。

- [ ] **Step 1: 写失败的测试**

```python
# tests/protocol/test_make_targets.py
import pathlib, subprocess, unittest
ROOT = pathlib.Path(__file__).resolve().parents[2]

class MakeTargetsTest(unittest.TestCase):
    def test_protocol_targets_exist(self):
        out = subprocess.run(["make", "-qp"], cwd=ROOT, capture_output=True, text=True).stdout
        for target in ("test:", "protocol:", "protocol-check:"):
            self.assertIn(target, out)

    def test_protocol_check_is_clean_and_read_only(self):
        before = subprocess.run(["git", "status", "--porcelain"], cwd=ROOT, capture_output=True, text=True).stdout
        result = subprocess.run(["make", "protocol-check"], cwd=ROOT, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        after = subprocess.run(["git", "status", "--porcelain"], cwd=ROOT, capture_output=True, text=True).stdout
        self.assertEqual(before, after)
```

- [ ] **Step 2: 运行确认失败**

Run: `python3 -m unittest tests.protocol.test_make_targets -v`
Expected: FAIL（没有 Makefile）。

- [ ] **Step 3: 写 Makefile**

```make
PYTHON ?= python3

.PHONY: test protocol protocol-check

test:
	$(PYTHON) -m unittest discover -s tests -v

protocol:
	$(PYTHON) scripts/gen_protocol_gdscript.py
	$(PYTHON) scripts/gen_protocol_docs.py

protocol-check:
	$(PYTHON) scripts/validate_protocol.py
	$(PYTHON) scripts/gen_protocol_gdscript.py --check
	$(PYTHON) scripts/gen_protocol_docs.py --check
```

- [ ] **Step 4: 运行确认通过**

Run: `make protocol-check && python3 -m unittest tests.protocol.test_make_targets -v`
Expected: 全部 PASS。

- [ ] **Step 5: 提交**

```bash
git add Makefile tests/protocol/test_make_targets.py
git commit -m "build: 新增 make test 与协议生成目标"
```

---

### Task 6: 人工协议文档

**Files:**
- Create: `docs/current/technical/architecture/protocol.md`
- Modify: `docs/current/technical/architecture/README.md`
- Modify: `docs/current/technical/README.md`
- Test: `tests/protocol/test_docs_links.py`

**Interfaces:**
- Consumes: 设计文档 §1–§11 的规则。
- Produces: `protocol.md` 的章节锚点，供 Task 7 的链接与索引引用。

- [ ] **Step 1: 写失败的链接测试**

```python
# tests/protocol/test_docs_links.py
import pathlib, re, unittest
ROOT = pathlib.Path(__file__).resolve().parents[2]
DOCS = ROOT / "docs"
LINK = re.compile(r"\]\(([^)#]+)(?:#[^)]*)?\)")

class DocsLinksTest(unittest.TestCase):
    def test_protocol_doc_and_index_links_resolve(self):
        for path in [DOCS / "versions/v0.1/technical/architecture/protocol.md",
                     DOCS / "versions/v0.1/technical/architecture/README.md",
                     DOCS / "versions/v0.1/technical/README.md"]:
            self.assertTrue(path.exists(), path)
            text = path.read_text()
            for target in LINK.findall(text):
                if target.startswith(("http://", "https://", "mailto:")):
                    continue
                resolved = (path.parent / target).resolve()
                self.assertTrue(resolved.exists(), f"{path.name} -> {target}")
```

- [ ] **Step 2: 运行确认失败**

Run: `python3 -m unittest tests.protocol.test_docs_links -v`
Expected: FAIL（`protocol.md` 不存在）。

- [ ] **Step 3: 写 protocol.md 并更新索引**

`protocol.md` 只写规则，不复制字段表，结构固定：

```
# 通信协议
> 状态：方案未实施
> 依赖：设计文档链接、联网与技术架构

## 1. 连接生命周期
## 2. 信封与序号
## 3. 消息与方向
## 4. 可见状态与可操作提示
## 5. 意图与幂等
## 6. 鉴权、心跳与重连
## 7. 背压、限流与消息上限
## 8. 契约来源与生成
## 9. 版本演进与兼容
## 10. 错误码
## 11. 验证
```

每节内容取自设计文档对应小节；字段细节链接 `protocol-messages.md`，不重复。两个 README 的索引表各加一行指向 `protocol.md`。

- [ ] **Step 4: 运行确认通过**

Run: `python3 -m unittest tests.protocol.test_docs_links -v`
Expected: PASS。

- [ ] **Step 5: 提交**

```bash
git add docs/current/technical/architecture/protocol.md \
        docs/current/technical/architecture/README.md \
        docs/current/technical/README.md \
        tests/protocol/test_docs_links.py
git commit -m "docs: 新增 v0.1 通信协议规则文档"
```

---

### Task 7: 基线文档改写与全局一致性

**Files:**
- Modify: `docs/current/technical/architecture/network-architecture.md`
- Modify: `docs/current/technical/architecture/server-stack.md`
- Modify: `docs/current/technical/architecture/client-stack.md`
- Modify: `docs/versions/v0.1/open-questions.md`
- Test: `tests/protocol/test_docs_consistency.py`

**Interfaces:**
- Consumes: `protocol.md` 的路径与标题。
- Produces: 全仓库文档不再出现 `StateDelta`；`network-architecture.md` 与协议文档一致。

- [ ] **Step 1: 写失败的一致性测试**

```python
# tests/protocol/test_docs_consistency.py
import pathlib, unittest
ROOT = pathlib.Path(__file__).resolve().parents[2]
DOCS = ROOT / "docs"

class DocsConsistencyTest(unittest.TestCase):
    def test_no_state_delta_anywhere(self):
        offenders = [p for p in DOCS.rglob("*.md") if "StateDelta" in p.read_text()]
        self.assertEqual(offenders, [])

    def test_network_architecture_matches_protocol(self):
        text = (DOCS / "versions/v0.1/technical/architecture/network-architecture.md").read_text()
        for needle in ('"v": 1', '"replyTo"', "StateUpdate", "Snapshot", "不递增"):
            self.assertIn(needle, text)

    def test_protocol_links_from_architecture_index(self):
        text = (DOCS / "versions/v0.1/technical/architecture/README.md").read_text()
        self.assertIn("protocol.md", text)
        self.assertIn("protocol-messages.md", text)
```

- [ ] **Step 2: 运行确认失败**

Run: `python3 -m unittest tests.protocol.test_docs_consistency -v`
Expected: FAIL，遍布 `StateDelta`，且缺 `"v": 1`。

- [ ] **Step 3: 改写文档**

- `network-architecture.md`：§5.1 信封改用 `v`/`id`/`replyTo`/RFC 3339 `ts`；§5.2 `StateDelta` → `StateUpdate` 并写明"完整可见视图 + events"；§5.3 序列示例同步；§5.4 补"重复意图回原结果、去重表随房间生命周期"；§6.2 重连改为"先认证再 `Reconnect`，直接返回 `Snapshot`，不补发增量"；§6.1 补"`Snapshot` 不递增 `seq`"。
- `server-stack.md`：工程形态补 `protocol/` IDL 目录、生成物位置与"Rust 生成在 workspace 建立后启用"。
- `client-stack.md`：补生成物 `protocol/generated/protocol.gd` 与"客户端不做完整运行时校验，只检查 type/v/关键字段"。
- `open-questions.md`：新增「GDScript 生成器长期维护成本」「观战只读视图字段裁剪」两条；把「协议字段表手写/生成」从待确认中移除（已定先手写后自动）。

- [ ] **Step 4: 运行全部验证**

Run: `make protocol-check && python3 -m unittest discover -s tests -v && python3 scripts/validate_docs.py`
Expected: 三条命令均退出 0。

- [ ] **Step 5: 提交**

```bash
git add docs/current/technical docs/versions/v0.1/open-questions.md tests/protocol/test_docs_consistency.py
git commit -m "docs: 同步协议变更到联网与技术基线"
```

---

### Task 8: 收尾自检

**Files:**
- 不新增文件；只核对前七个任务的产物。

- [ ] **Step 1: 全量验证**

Run: `make test && make protocol-check && python3 scripts/validate_docs.py`
Expected: 全部退出 0，测试数 ≥ 24。

- [ ] **Step 2: 生成物可重现**

Run: `make protocol && git status --porcelain`
Expected: 输出为空（生成后无差异）。

- [ ] **Step 3: 检查变更范围**

Run: `git diff develop --stat`
Expected: 只含 `protocol/`、`scripts/`、`tests/protocol/`、`Makefile`、`docs/current/technical/`、`docs/versions/v0.1/open-questions.md`、`docs/superpowers/specs/`；无 `server/`、`client/` 目录。

- [ ] **Step 4: 提交（若有遗漏修正）**

```bash
git add -A
git commit -m "chore(protocol): 收尾校验与生成物同步"
```
