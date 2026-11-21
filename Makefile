PYTHON ?= python3
GODOT ?= $(shell command -v godot 2>/dev/null || echo /Applications/Godot.app/Contents/MacOS/Godot)
# 补丁版本固定，导出模板目录与之精确一致；本机引擎不符时由 client-godot-check 拒绝执行。
GODOT_VERSION := 4.7.2
GODOT_INSTALLED_VERSION = $(shell "$(GODOT)" --version 2>/dev/null | head -n 1 | cut -d. -f1-3)
GODOT_TEMPLATE_DIR := $(HOME)/Library/Application Support/Godot/export_templates/$(GODOT_VERSION).stable
DATABASE_URL ?= postgres://postgres:postgres@127.0.0.1:5432/civilization
ASSET_MAP ?= client/assets/characters/portraits/asset_map.json
FONT_SOURCE ?= output/fonts/NotoSansSC[wght].ttf

.PHONY: test lint audit data data-check docs-render assets sfx font-subset balance-check protocol protocol-check protocol-rs protocol-rs-check m0-check m0-determinism m0-batch m0-report m1-check client-godot-check client-import client-run client-run-on-iphone client-run-on-ipad client-run-on-desktop client-check client-smoke ui-wireframe client-export-templates client-export client-export-ios client-run-ios m1b-build m1b-e2e dev-bots db-up db-down db-reset db-test deploy-render deploy-lint deploy-check-env deploy-image deploy-load deploy-preflight deploy-apply deploy-rollout deploy-status deploy-smoke deploy

test:
	$(PYTHON) -m unittest discover -s tests -v
	cd server && cargo test --workspace

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

lint:
	cd server && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings

audit:
	$(PYTHON) scripts/audit_effects.py

data:
	$(PYTHON) scripts/gen_card_data.py
	$(PYTHON) scripts/gen_client_data.py
	$(PYTHON) scripts/gen_text.py

data-check:
	$(PYTHON) scripts/validate_docs.py
	$(PYTHON) scripts/gen_card_data.py --check
	$(PYTHON) scripts/gen_client_data.py --check
	$(PYTHON) scripts/audit_effects.py
	$(PYTHON) scripts/gen_text.py --check
	$(PYTHON) scripts/render_docs.py --check

# 重写技能实例与卡牌的派生小节（效果正文、属性派生行、技能索引文本列）。
docs-render:
	$(PYTHON) scripts/render_docs.py

# 美术素材后处理：对映射内的源立绘去水印，派生头像与卡面并重写清单。
assets:
	@[ -f "$(ASSET_MAP)" ] || { echo "assets 需要映射文件 $(ASSET_MAP)（card_id → 源立绘文件名）" >&2; exit 1; }
	$(PYTHON) scripts/process_assets.py --map "$(ASSET_MAP)" \
		--portraits client/assets/characters/portraits \
		--avatars client/assets/characters/avatars \
		--faces client/assets/cards/faces \
		--manifest client/assets/asset_manifest.json

# 默认占位音效：合成波形写入 client/assets/audio/，逐卡覆盖不在此生成。
sfx:
	$(PYTHON) scripts/gen_sfx.py

# 内置中文字体子集：字符集取文本表 + ASCII + GB2312，需先装 fontTools（见脚本说明）。
font-subset:
	@[ -f "$(FONT_SOURCE)" ] || { echo "font-subset 需要源字体 $(FONT_SOURCE)（Google Fonts 的 Noto Sans SC 变量字体）" >&2; exit 1; }
	$(PYTHON) scripts/subset_font.py --csv client/text/zh_CN.csv \
		--source "$(FONT_SOURCE)" --output client/assets/fonts/NotoSansSC-Regular-subset.ttf

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

# 本机引擎必须与固定的补丁版本一致，避免用错版本导入或导出。
client-godot-check:
	@[ -n "$(GODOT_INSTALLED_VERSION)" ] || { echo "无法读取 Godot 版本：$(GODOT) --version" >&2; exit 1; }
	@[ "$(GODOT_INSTALLED_VERSION)" = "$(GODOT_VERSION)" ] || { echo "Godot 版本不符：需要 $(GODOT_VERSION)，当前 $(GODOT_INSTALLED_VERSION)" >&2; exit 1; }

# 客户端工程导入与静态边界检查。
client-import: client-godot-check
	$(GODOT) --headless --path client --import

# 桌面客户端统一从这里启动：先导入刷新全局类缓存。新增或改名 class_name
# 脚本后直接跑 Godot 会沿用旧缓存，缺类的脚本编译失败，对局页可能只构建一半。
# 模拟不同尺寸设备：make client-run WIDTH=960 HEIGHT=540；两者需同时为正整数，缺省按屏幕自动收拢。
WIDTH ?=
HEIGHT ?=

client-run: client-import
	@if [ -z "$(WIDTH)" ] && [ -z "$(HEIGHT)" ]; then \
	  $(GODOT) --path client; \
	elif [ "$(WIDTH)" -gt 0 ] 2>/dev/null && [ "$(HEIGHT)" -gt 0 ] 2>/dev/null; then \
	  CIVILIZATION_WINDOW_SIZE="$(WIDTH)x$(HEIGHT)" $(GODOT) --path client; \
	else \
	  echo "WIDTH 与 HEIGHT 需同时传入正整数：make client-run WIDTH=960 HEIGHT=540" >&2; \
	  exit 1; \
	fi

# 设备档别名：等价于直接传 WIDTH/HEIGHT，只改窗口尺寸，设备档案仍按桌面。
# 目标名可被 shell 自带的 make 补全列出，`make client-run-on-<Tab>` 即可选择。
# 尺寸取项目验收用机的横屏物理分辨率：iPhone 17 Pro、iPad Air 11。
client-run-on-iphone:
	$(MAKE) client-run WIDTH=2622 HEIGHT=1206

client-run-on-ipad:
	$(MAKE) client-run WIDTH=2360 HEIGHT=1640

client-run-on-desktop:
	$(MAKE) client-run

client-check:
	$(MAKE) protocol-check
	$(MAKE) data-check
	$(PYTHON) -m unittest tests.test_client_boundary -v
	$(PYTHON) -m unittest tests.test_client_style -v
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

# 页面线框图：无头装载真实场景解算布局，导出 SVG 与 PNG 到 output/wireframes/。
# PAGE 取 client/pages 下的页面名（gallery、room 等）；ALL=1 生成全部；DEVICE=phone 出手机档位。
PAGE ?=
DEVICE ?=

ui-wireframe: client-import
	@if [ -z "$(PAGE)" ] && [ "$(ALL)" != "1" ]; then \
	  echo "用法：make ui-wireframe PAGE=gallery [DEVICE=phone]，或 make ui-wireframe ALL=1" >&2; \
	  exit 1; \
	fi
	$(PYTHON) scripts/ui_wireframe.py --godot "$(GODOT)" \
		$(if $(PAGE),--page $(PAGE),) $(if $(ALL),--all,) $(if $(DEVICE),--device $(DEVICE),)

# macOS 导出模板：目录按固定的 Godot 版本隔离，缺失时下载官方 tpz 并只拷贝其中的 templates/。
client-export-templates: client-godot-check
	@[ "$$(uname -s)" = "Darwin" ] || { echo "client-export-templates 仅支持 macOS（当前 $$(uname -s)）" >&2; exit 1; }
	@if [ -d "$(GODOT_TEMPLATE_DIR)" ] && [ -n "$$(ls -A "$(GODOT_TEMPLATE_DIR)" 2>/dev/null)" ]; then \
	  echo "导出模板已存在：$(GODOT_TEMPLATE_DIR)"; \
	else \
	  url="https://github.com/godotengine/godot/releases/download/$(GODOT_VERSION)-stable/Godot_v$(GODOT_VERSION)-stable_export_templates.tpz"; \
	  tmp="$$(mktemp -d)"; \
	  trap 'rm -rf "$$tmp"' EXIT; \
	  echo "下载导出模板：$$url"; \
	  curl -fL --retry 3 --progress-bar -o "$$tmp/templates.tpz" "$$url" || { echo "导出模板下载失败：$$url" >&2; exit 1; }; \
	  mkdir -p "$$tmp/unpacked" "$(GODOT_TEMPLATE_DIR)"; \
	  ditto -x -k "$$tmp/templates.tpz" "$$tmp/unpacked" || { echo "导出模板解压失败" >&2; exit 1; }; \
	  cp -R "$$tmp/unpacked/templates/." "$(GODOT_TEMPLATE_DIR)/" || { echo "导出模板复制失败" >&2; exit 1; }; \
	  echo "导出模板已安装：$(GODOT_TEMPLATE_DIR)"; \
	fi

# 未签名 Universal .app 与 zip 产物。Gatekeeper 放行方式见 docs/current/technical/client.md。
client-export: client-export-templates client-import
	@[ "$$(uname -s)" = "Darwin" ] || { echo "client-export 仅支持 macOS（当前 $$(uname -s)）" >&2; exit 1; }
	@mkdir -p output/desktop
	rm -rf output/desktop/Civilization.app output/desktop/Civilization-macos.zip
	"$(GODOT)" --headless --path client --export-release "macOS" "$(CURDIR)/output/desktop/Civilization.app"
	test -x output/desktop/Civilization.app/Contents/MacOS/Civilization
	ditto -c -k --sequesterRsrc --keepParent output/desktop/Civilization.app output/desktop/Civilization-macos.zip

# iOS 真机开发签名包：先导出 Xcode 工程，再由 xcodebuild 自动签名并打包 .ipa。
# 需要 Xcode 已登录 Apple 团队 Q62EFJPV3C，且目标真机已登记；否则 xcodebuild 失败。
client-export-ios: client-export-templates client-import
	@[ "$$(uname -s)" = "Darwin" ] || { echo "client-export-ios 仅支持 macOS（当前 $$(uname -s)）" >&2; exit 1; }
	@[ -f "$(GODOT_TEMPLATE_DIR)/ios.zip" ] || { echo "缺少 iOS 导出模板：$(GODOT_TEMPLATE_DIR)/ios.zip" >&2; exit 1; }
	@mkdir -p output/ios
	rm -rf output/ios/Civilization output/ios/Civilization.xcodeproj output/ios/Civilization.xcframework \
	  output/ios/MoltenVK.xcframework output/ios/libgodot_camera.*.xcframework \
	  output/ios/Civilization.pck output/ios/Payload output/ios/Civilization-ios.ipa
	"$(GODOT)" --headless --path client --export-release "iOS" "$(CURDIR)/output/ios/Civilization.xcodeproj"
	test -d output/ios/Civilization.xcodeproj
	xcodebuild -project output/ios/Civilization.xcodeproj -scheme Civilization -configuration Release \
	  -sdk iphoneos -destination 'generic/platform=iOS' -allowProvisioningUpdates \
	  -derivedDataPath output/ios/build build
	test -d output/ios/build/Build/Products/Release-iphoneos/Civilization.app
	mkdir -p output/ios/Payload
	ditto output/ios/build/Build/Products/Release-iphoneos/Civilization.app output/ios/Payload/Civilization.app
	ditto -c -k --sequesterRsrc --keepParent output/ios/Payload output/ios/Civilization-ios.ipa
	test -f output/ios/Civilization-ios.ipa
	@echo "iOS 产物：output/ios/Civilization-ios.ipa"

# iOS 真机开发运行：导出并签名后安装到 iPhone 并启动。
# 默认取第一台已连接的 iPhone；IOS_DEVICE 用设备名或 UDID 指定，IOS_LAUNCH=0 只安装。
IOS_DEVICE ?=
IOS_LAUNCH ?= 1

client-run-ios: client-export-ios
	IOS_DEVICE="$(IOS_DEVICE)" IOS_LAUNCH="$(IOS_LAUNCH)" bash scripts/ios_run.sh

# 1 个 Godot 客户端 + 7 个机器人的无头端到端。
m1b-build:
	cd server && cargo build -q -p room-service -p sim

m1b-e2e: client-import m1b-build
	GODOT="$(GODOT)" bash scripts/m1b_e2e.sh

# 本地联调：起服务端与 7 个机器人，打印房间码后等开发者在大厅加入。
DEV_PORT ?= 8080

dev-bots: client-import m1b-build
	DEV_PORT=$(DEV_PORT) bash scripts/dev_bots.sh

# platform 共享 Kubernetes 发布入口：本机渲染，远端只执行 kubectl/ctr。
ENV ?= dev
SERVER ?= root@dell.local
DEPLOY_CHART ?= deploy/helm/civilization-server
DEPLOY_VALUES ?= deploy/helm/values.$(ENV).yaml
DEPLOY_NAMESPACE ?= $(if $(filter prod,$(ENV)),civilization,civilization-dev)
DEPLOY_HOST ?= $(if $(filter prod,$(ENV)),civilization.platform.internal,civilization.dev.platform.internal)
DEPLOY_IMAGE ?= civilization-server:$(ENV)

# 输出就是清单本身：命令回显会混进管道，被 kubectl 当成清单解析失败。
deploy-render:
	@helm template civilization-server $(DEPLOY_CHART) --namespace $(DEPLOY_NAMESPACE) -f $(DEPLOY_VALUES)

deploy-lint:
	helm lint $(DEPLOY_CHART) -f deploy/helm/values.dev.yaml
	helm lint $(DEPLOY_CHART) -f deploy/helm/values.prod.yaml
	$(MAKE) deploy-render ENV=dev >/dev/null
	$(MAKE) deploy-render ENV=prod >/dev/null

deploy-image:
	docker build --platform linux/amd64 -t $(DEPLOY_IMAGE) -f server/Dockerfile .

deploy-load:
	docker save $(DEPLOY_IMAGE) | ssh $(SERVER) "ctr -n k8s.io images import -"

# ENV 拼错时不能静默回落到 dev 的 namespace 与域名。
deploy-check-env:
	@$(if $(filter dev prod,$(ENV)),,$(error ENV 只能是 dev 或 prod，收到：$(ENV)))

# 子 make 必须静默：它回显的命令行会混进管道，被 kubectl 当成清单解析失败。
deploy-preflight: deploy-check-env
	$(MAKE) -s deploy-render ENV=$(ENV) | ssh $(SERVER) "kubectl apply --dry-run=server -f -"

deploy-apply: deploy-check-env
	$(MAKE) -s deploy-render ENV=$(ENV) | ssh $(SERVER) "kubectl apply -f -"

deploy-rollout: deploy-check-env
	ssh $(SERVER) "kubectl rollout restart deployment/civilization-server -n $(DEPLOY_NAMESPACE) && kubectl rollout status deployment/civilization-server -n $(DEPLOY_NAMESPACE) --timeout=180s"

deploy-status: deploy-check-env
	ssh $(SERVER) "kubectl get pods,service,httproute -n $(DEPLOY_NAMESPACE) -l app.kubernetes.io/name=civilization-server"

deploy-smoke: deploy-check-env
	ssh $(SERVER) "curl -fsS https://$(DEPLOY_HOST)/healthz && curl -fsS https://$(DEPLOY_HOST)/readyz"

deploy: deploy-check-env deploy-image deploy-load deploy-preflight deploy-apply deploy-rollout deploy-smoke

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
