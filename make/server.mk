# 服务端测试、静态检查、模拟器、压测与本地联调。由根 Makefile include。
# 依赖根 Makefile 的 PYTHON、JOBS；m1b-e2e / dev-bots 依赖 make/client.mk 的 client-import。

DATABASE_URL ?= postgres://postgres:postgres@127.0.0.1:5432/civilization
DEV_PORT ?= 8080

.PHONY: test lint db-up db-down db-reset db-test m0-determinism m0-batch m0-report \
	m1b-build m1b-e2e dev-bots load-build loadtest loadtest-ramp loadtest-smoke

test:
	$(PYTHON) scripts/run_parallel.py $(if $(JOBS),--jobs $(JOBS),) \
		"$(PYTHON) -m unittest discover -s tests -v" \
		"cd server && cargo test --workspace"

lint:
	cd server && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings

db-up:
	docker compose up -d --wait db

db-down:
	docker compose down

db-reset:
	docker compose down -v
	$(MAKE) db-up

# 持久化测试需要真实 PostgreSQL：账号、好友、最近对局与邀请跨重启恢复。
db-test: db-up
	cd server && TEST_DATABASE_URL=$(DATABASE_URL) cargo test -p room-service --test persistence

m0-determinism:
	cd server && cargo test -p sim --test sim_determinism replay_same_log_100_times_has_same_hash

m0-batch:
	cd server && cargo run -q --release -p sim -- batch --games 1000 --seed 1 --out ../output/m0-batch.json

M0_GAMES ?= 1000
M0_BATCH_LOG ?= ../output/m0-batch.json

m0-report:
	@mkdir -p ../output
	cd server && cargo run -q --release -p sim -- check --log $(M0_BATCH_LOG) --expect-games $(M0_GAMES)
	cd server && cargo run -q --release -p sim -- report --log $(M0_BATCH_LOG) 2>&1 | tee ../output/m0-report.log

# 1 个 Godot 客户端 + 7 个机器人的无头端到端。
m1b-build:
	cd server && cargo build -q -p room-service -p sim

m1b-e2e: client-import m1b-build
	GODOT="$(GODOT)" bash scripts/m1b_e2e.sh

# 本地联调：起服务端与 7 个机器人，打印房间码后等开发者在大厅加入。
dev-bots: client-import m1b-build
	DEV_PORT=$(DEV_PORT) bash scripts/dev_bots.sh

# 并发对局压测：release 产物 + 本机 room-service 自动起停；报告写 output/（忽略目录）。
LOAD_SERVER_BIN ?= server/target/release/room-service
LOAD_SIM_BIN ?= server/target/release/sim
LOAD_ROOMS ?= 10
LOAD_DURATION_MS ?= 30000
LOAD_ROOMS_MAX ?= 100
LOAD_STEP ?= 10
LOAD_STEP_MS ?= 20000

load-build:
	cd server && cargo build --release -p room-service -p sim

loadtest: load-build
	@mkdir -p output
	$(LOAD_SIM_BIN) load --serve-room-service $(LOAD_SERVER_BIN) \
		--scenario steady --rooms $(LOAD_ROOMS) --duration-ms $(LOAD_DURATION_MS) \
		--out output/loadtest-steady.json

loadtest-ramp: load-build
	@mkdir -p output
	$(LOAD_SIM_BIN) load --serve-room-service $(LOAD_SERVER_BIN) \
		--scenario ramp --rooms $(LOAD_ROOMS) --rooms-max $(LOAD_ROOMS_MAX) \
		--step $(LOAD_STEP) --step-ms $(LOAD_STEP_MS) \
		--out output/loadtest-ramp.json

loadtest-smoke: load-build
	@mkdir -p output
	$(LOAD_SIM_BIN) load --serve-room-service $(LOAD_SERVER_BIN) \
		--scenario steady --rooms 2 --duration-ms 5000 \
		--out output/loadtest-smoke.json
