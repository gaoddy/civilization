# 顶层 Make 入口：共享变量与验收入口。
# 各领域目标分册在 make/ 下：
#   make/client.mk   客户端构建、运行、检查与导出
#   make/content.mk  内容、协议、资产与文档生成/校验
#   make/server.mk   服务端测试、模拟器、压测与本地联调
#   make/deploy.mk   服务端部署
# 命令说明见 AGENTS.md 与 docs/current/technical/。

PYTHON ?= python3

# 独立步骤的并行度：留空按 CPU 数（上限 8），JOBS=1 退回串行。
JOBS ?=

.DEFAULT_GOAL := test

include make/client.mk
include make/content.mk
include make/server.mk
include make/deploy.mk

.PHONY: m0-check m1-check

m0-check:
	$(PYTHON) scripts/run_parallel.py $(if $(JOBS),--jobs $(JOBS),) \
		"$(MAKE) --no-print-directory data-check" \
		"$(MAKE) --no-print-directory test" \
		"$(MAKE) --no-print-directory lint" \
		"$(MAKE) --no-print-directory m0-determinism"

# M1a 收尾验收入口。e2e 由 `make test` 的 `cargo test --workspace` 覆盖，
# 不在此单独再跑：那会用真实 WebSocket 起 8 个 bot 跑两次。
m1-check:
	$(PYTHON) scripts/run_parallel.py $(if $(JOBS),--jobs $(JOBS),) \
		"$(MAKE) --no-print-directory protocol-rs-check" \
		"$(MAKE) --no-print-directory protocol-check" \
		"$(MAKE) --no-print-directory data-check" \
		"$(MAKE) --no-print-directory test" \
		"$(MAKE) --no-print-directory lint" \
		"$(MAKE) --no-print-directory m0-determinism"
