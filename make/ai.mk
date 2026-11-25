.PHONY: ai-test ai-check ai-data train-c0 train-c1 train-c2 ai-eval

# 训练产物根目录；targets 在 server/ 下执行，因此按仓库根相对路径写。
AI_MODELS ?= ../output/models

# 各阶段共用的随机种子。
SEED ?= 0

ai-test:
	cd server && cargo test -p ai-core -p ai-model -p ai-train

ai-check:
	cd server && cargo check -p ai-train --features train

# C0 棋盘对：train-c0 只读不生成，重训前先跑这一步。
ai-data:
	cd server && cargo run --release -p ai-train --features train -- data-c0 --out $(AI_MODELS)/c0/pairs.jsonl --seed $(SEED) --games $(or $(GAMES),2000)

train-c0:
	cd server && cargo run -p ai-train --features train -- train-c0 --out-dir $(AI_MODELS)/c0 --seed $(SEED)

train-c1:
	cd server && cargo run -p ai-train --features train -- train-c1 --out-dir $(AI_MODELS)/c1 --seed $(SEED)

train-c2:
	cd server && cargo run --release -p ai-train --features train -- train-c2 --out-dir $(AI_MODELS)/c2 --seed $(SEED) --episodes $(or $(EPISODES),1000)

ai-eval:
	cd server && cargo run --release -p ai-train --features train -- eval --seeds $(or $(SEEDS),2000) $(if $(RUN),--run $(RUN),)
