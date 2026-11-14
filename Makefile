PYTHON ?= python3

.PHONY: test lint audit data data-check protocol protocol-check

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
