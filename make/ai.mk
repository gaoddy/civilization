.PHONY: ai-test ai-check train-c0 train-c1 train-c2 ai-eval

ai-test:
	cd server && cargo test -p ai-core -p ai-model -p ai-train

ai-check:
	cd server && cargo check -p ai-train --features train

train-c0:
	cd server && cargo run -p ai-train --features train -- train-c0

train-c1:
	cd server && cargo run -p ai-train --features train -- train-c1

train-c2:
	cd server && cargo run -p ai-train --features train -- train-c2

ai-eval:
	cd server && cargo run -p ai-train --features train -- eval
