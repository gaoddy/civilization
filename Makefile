PYTHON ?= python3

.PHONY: test lint audit data data-check protocol protocol-check protocol-rs protocol-rs-check m0-check m0-determinism m0-batch m0-report m1-check

test:
	$(PYTHON) -m unittest discover -s tests -v
	cd server && cargo test --workspace

lint:
	cd server && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings

audit:
	$(PYTHON) scripts/audit_effects.py

data:
	$(PYTHON) scripts/gen_card_data.py

data-check:
	$(PYTHON) scripts/gen_card_data.py --check
	$(PYTHON) scripts/audit_effects.py

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
