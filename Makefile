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
