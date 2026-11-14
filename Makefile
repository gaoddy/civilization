PYTHON ?= python3

.PHONY: test lint audit data data-check protocol protocol-check m0-determinism m0-batch m0-report

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

m0-determinism:
	cd server && cargo test -p sim --test sim_determinism replay_same_log_100_times_has_same_hash

m0-batch:
	cd server && cargo run -q -p sim -- batch --games 1000 --seed 1 --out ../output/m0-batch.json

m0-report:
	cd server && cargo run -q -p sim -- report --log ../output/m0-batch.json
