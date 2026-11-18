PYTHON ?= python3
GODOT ?= $(shell command -v godot 2>/dev/null || echo /Applications/Godot.app/Contents/MacOS/Godot)

.PHONY: test lint audit data data-check balance-check protocol protocol-check protocol-rs protocol-rs-check m0-check m0-determinism m0-batch m0-report m1-check client-import client-run client-check client-smoke m1b-build m1b-e2e dev-bots

test:
	$(PYTHON) -m unittest discover -s tests -v
	cd server && cargo test --workspace

lint:
	cd server && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings

audit:
	$(PYTHON) scripts/audit_effects.py

data:
	$(PYTHON) scripts/gen_card_data.py
	$(PYTHON) scripts/gen_client_data.py
	$(PYTHON) scripts/gen_text.py

data-check:
	$(PYTHON) scripts/gen_card_data.py --check
	$(PYTHON) scripts/gen_client_data.py --check
	$(PYTHON) scripts/audit_effects.py
	$(PYTHON) scripts/gen_text.py --check

balance-check:
	$(PYTHON) scripts/balance_report.py --check

protocol:
	$(PYTHON) scripts/gen_protocol_gdscript.py
	$(PYTHON) scripts/gen_protocol_docs.py

protocol-check:
	$(PYTHON) scripts/validate_protocol.py
	$(PYTHON) scripts/gen_protocol_gdscript.py --check
	$(PYTHON) scripts/gen_protocol_docs.py --check

protocol-rs:
	cd server && cargo run -q -p protocol-gen

protocol-rs-check:
	cd server && cargo run -q -p protocol-gen -- --check

m0-check:
	$(MAKE) data-check
	$(MAKE) test
	$(MAKE) lint
	$(MAKE) m0-determinism

# M1a 收尾验收入口。e2e 由 `make test` 的 `cargo test --workspace` 覆盖，
# 不在此单独再跑：那会用真实 WebSocket 起 8 个 bot 跑两次。
m1-check:
	$(MAKE) protocol-rs-check
	$(MAKE) protocol-check
	$(MAKE) data-check
	$(MAKE) test
	$(MAKE) lint
	$(MAKE) m0-determinism

# 客户端工程导入与静态边界检查。
client-import:
	$(GODOT) --headless --path client --import

# 桌面客户端统一从这里启动：先导入刷新全局类缓存。新增或改名 class_name
# 脚本后直接跑 Godot 会沿用旧缓存，缺类的脚本编译失败，对局页可能只构建一半。
client-run: client-import
	$(GODOT) --path client

client-check:
	$(MAKE) protocol-check
	$(MAKE) data-check
	$(PYTHON) -m unittest tests.test_client_boundary -v
	$(PYTHON) -m unittest tests.test_i18n -v
	$(MAKE) client-import

client-smoke: client-import
	@mkdir -p output
	@for scene in client/tests/ui_smoke*.tscn; do \
	  name=$$(basename $$scene .tscn); \
	  $(GODOT) --headless --path client res://tests/$$name.tscn 2>&1 | tee output/$$name.log; \
	  grep -q "UI_SMOKE_OK" output/$$name.log || exit 1; \
	  ! grep -q "SCRIPT ERROR" output/$$name.log || exit 1; \
	done

# 1 个 Godot 客户端 + 7 个机器人的无头端到端。
m1b-build:
	cd server && cargo build -q -p room-service -p sim

m1b-e2e: client-import m1b-build
	GODOT="$(GODOT)" bash scripts/m1b_e2e.sh

# 本地联调：起服务端与 7 个机器人，打印房间码后等开发者在大厅加入。
DEV_PORT ?= 8080

dev-bots: client-import m1b-build
	DEV_PORT=$(DEV_PORT) bash scripts/dev_bots.sh

m0-determinism:
	cd server && cargo test -p sim --test sim_determinism replay_same_log_100_times_has_same_hash

m0-batch:
	cd server && cargo run -q -p sim -- batch --games 1000 --seed 1 --out ../output/m0-batch.json

M0_GAMES ?= 1000
M0_BATCH_LOG ?= ../output/m0-batch.json

m0-report:
	@mkdir -p ../output
	cd server && cargo run -q -p sim -- check --log $(M0_BATCH_LOG) --expect-games $(M0_GAMES)
	cd server && cargo run -q -p sim -- report --log $(M0_BATCH_LOG) 2>&1 | tee ../output/m0-report.log
