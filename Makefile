PYTHON ?= python3
GODOT ?= $(shell command -v godot 2>/dev/null || echo /Applications/Godot.app/Contents/MacOS/Godot)
# 补丁版本固定，导出模板目录与之精确一致；本机引擎不符时由 client-godot-check 拒绝执行。
GODOT_VERSION := 4.7.2
# 导入压缩策略，默认 0（headless + CPU 压缩）：auto=有图形会话时非无头导入（Betsy 优先用 GPU，失败回落 CPU）；
# 1=强制非无头。headless 下引擎不允许创建 RenderingDevice，GPU 压缩不可用。
CLIENT_IMPORT_GPU ?= 0
# 纹理产物（PNG → BPTC/ASTC + .import/.md5）由独立工具生成，Godot 只做扫描；
# 工具与质量门禁说明见 AGENTS.md「仓库与入口」。
ASSET_COMPRESS_DIR := tools/asset-compress
ASSET_COMPRESS_QUALITY ?= standard
ASSET_COMPRESS_BACKEND ?= auto
GODOT_INSTALLED_VERSION = $(shell "$(GODOT)" --version 2>/dev/null | head -n 1 | cut -d. -f1-3)
GODOT_TEMPLATE_DIR := $(HOME)/Library/Application Support/Godot/export_templates/$(GODOT_VERSION).stable
DATABASE_URL ?= postgres://postgres:postgres@127.0.0.1:5432/civilization
ASSET_MAP ?= client/assets/characters/portraits/asset_map.json
ASSET_COVERS ?= output/effect-covers/generated
FONT_SOURCE ?= output/fonts/NotoSansSC[wght].ttf

# 独立步骤的并行度：留空按 CPU 数（上限 8），JOBS=1 退回串行。
JOBS ?=

.PHONY: test lint audit data data-check docs-render assets assets-covers sfx font-subset balance-check protocol protocol-check protocol-rs protocol-rs-check m0-check m0-determinism m0-batch m0-report load-build loadtest loadtest-ramp loadtest-smoke m1-check client-godot-check client-import client-cache-copy client-run client-run-on-iphone client-run-on-ipad client-run-on-desktop client-check client-smoke client-env-check asset-compress asset-compress-check ui-wireframe ui-preview ui-preview-check client-export-templates client-export client-export-local client-export-steam client-notarize steam-upload client-export-ios client-run-ios client-export-android client-export-android-aab m1b-build m1b-e2e dev-bots db-up db-down db-reset db-test deploy-render deploy-lint deploy-check-env deploy-image deploy-load deploy-preflight deploy-apply deploy-rollout deploy-status deploy-smoke deploy

test:
	$(PYTHON) scripts/run_parallel.py $(if $(JOBS),--jobs $(JOBS),) \
		"$(PYTHON) -m unittest discover -s tests -v" \
		"cd server && cargo test --workspace"

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
	$(PYTHON) scripts/run_parallel.py $(if $(JOBS),--jobs $(JOBS),) \
		"$(PYTHON) scripts/gen_card_data.py" \
		"$(PYTHON) scripts/gen_client_data.py" \
		"$(PYTHON) scripts/gen_achievement_data.py" \
		"$(PYTHON) scripts/gen_text.py"

data-check:
	$(PYTHON) scripts/run_parallel.py $(if $(JOBS),--jobs $(JOBS),) \
		"$(PYTHON) scripts/validate_docs.py" \
		"$(PYTHON) scripts/gen_card_data.py --check" \
		"$(PYTHON) scripts/gen_client_data.py --check" \
		"$(PYTHON) scripts/gen_achievement_data.py --check" \
		"$(PYTHON) scripts/audit_effects.py" \
		"$(PYTHON) scripts/gen_text.py --check" \
		"$(PYTHON) scripts/render_docs.py --check"

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
		--manifest client/assets/asset_manifest.json \
		$(if $(JOBS),--jobs $(JOBS),)

# 效果牌封面后处理：对 <card_id>/*.png 去水印，缩放到 448×584 写入客户端卡面。
assets-covers:
	@[ -d "$(ASSET_COVERS)" ] || { echo "assets-covers 需要封面目录 $(ASSET_COVERS)（<card_id>/*.png）" >&2; exit 1; }
	$(PYTHON) scripts/process_assets.py --covers "$(ASSET_COVERS)" \
		--faces client/assets/cards/faces \
		--manifest client/assets/asset_manifest.json \
		$(if $(JOBS),--jobs $(JOBS),)

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
	$(PYTHON) scripts/run_parallel.py $(if $(JOBS),--jobs $(JOBS),) \
		"$(PYTHON) scripts/gen_protocol_gdscript.py" \
		"$(PYTHON) scripts/gen_protocol_docs.py"

protocol-check:
	$(PYTHON) scripts/run_parallel.py $(if $(JOBS),--jobs $(JOBS),) \
		"$(PYTHON) scripts/validate_protocol.py" \
		"$(PYTHON) scripts/gen_protocol_gdscript.py --check" \
		"$(PYTHON) scripts/gen_protocol_docs.py --check"

protocol-rs:
	cd server && cargo run -q -p protocol-gen

protocol-rs-check:
	cd server && cargo run -q -p protocol-gen -- --check

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

# 本机引擎必须与固定的补丁版本一致，避免用错版本导入或导出。
client-godot-check:
	@[ -n "$(GODOT_INSTALLED_VERSION)" ] || { echo "无法读取 Godot 版本：$(GODOT) --version" >&2; exit 1; }
	@[ "$(GODOT_INSTALLED_VERSION)" = "$(GODOT_VERSION)" ] || { echo "Godot 版本不符：需要 $(GODOT_VERSION)，当前 $(GODOT_INSTALLED_VERSION)" >&2; exit 1; }

# 客户端工程导入与静态边界检查。
# 默认 headless（CPU 压缩）：当前卡面是 high_quality=true 的 BC7，而 Godot 只对
# S3TC/BC6H 走 Betsy GPU 压缩，BC7 只能 CPU，非无头导入没有收益还会弹出编辑器窗口。
# CLIENT_IMPORT_GPU=1 手动切非无头（改用 S3TC/HDR 纹理后才有 GPU 加速）；
# auto 按图形会话自动判断，CI 与无 GUI 环境回落到 headless。
client-import: client-godot-check
	@mode="$(CLIENT_IMPORT_GPU)"; \
	case "$$mode" in \
	  auto|1|0) ;; \
	  *) echo "CLIENT_IMPORT_GPU 取值应为 auto、1 或 0" >&2; exit 2 ;; \
	esac; \
	if [ "$$mode" = "auto" ]; then \
	  if [ -n "$$DISPLAY" ] || [ -n "$$WAYLAND_DISPLAY" ] || { command -v launchctl >/dev/null 2>&1 && launchctl print "gui/$$(id -u)" >/dev/null 2>&1; }; then \
	    mode=1; \
	  else \
	    mode=0; \
	  fi; \
	fi; \
	if [ "$$mode" = "1" ]; then \
	  echo "client-import：非无头导入（仅 S3TC/HDR 纹理走 GPU 压缩）"; \
	  snapshot="$$(mktemp)"; cp client/project.godot "$$snapshot" 2>/dev/null || true; \
	  $(GODOT) --path client --import || { echo "非无头导入失败，回退 headless" >&2; $(GODOT) --headless --path client --import; }; \
	  if [ -f "$$snapshot" ] && ! cmp -s "$$snapshot" client/project.godot; then \
	    echo "提示：非无头导入按引擎格式重写了 client/project.godot，提交前请检查 diff" >&2; \
	  fi; \
	  rm -f "$$snapshot"; \
	else \
	  echo "client-import：headless 导入（CPU 压缩）"; \
	  $(GODOT) --headless --path client --import; \
	fi

# 新 worktree 的 Godot 导入缓存不跨目录共享，冷导入会重编全部 BC7 纹理。
# 从主检出复制已导入的 .godot/imported 与 *.import 旁车，随后 make client-import
# 只补翻译、类缓存等增量产物。缓存按源图 md5 校验，worktree 分支改过的资源仍会重导。
client-cache-copy:
	@set -eu; \
	main="$$(dirname "$$(git rev-parse --path-format=absolute --git-common-dir)")"; \
	here="$$(pwd -P)"; \
	if [ "$$here" = "$$main" ]; then echo "client-cache-copy：当前是主检出，无需复制缓存"; exit 0; fi; \
	[ -d "$$main/client/.godot/imported" ] || { echo "client-cache-copy：主检出缺少导入缓存，请先执行：cd $$main && make client-import" >&2; exit 1; }; \
	( cd "$$main/client" && find .godot/imported -type f -print0 && \
	  find . -name '*.import' -not -path './.godot/*' -print0 ) | \
	while IFS= read -r -d '' f; do \
	  mkdir -p "client/$$(dirname "$$f")"; \
	  cp -p "$$main/client/$$f" "client/$$f"; \
	done; \
	echo "client-cache-copy：已从 $$main 复制导入缓存"

# 重新生成 client/assets 下全部 PNG 的 VRAM 压缩产物（BPTC + ASTC）。
# 源图未变时按 source_md5 跳过，因此重复执行是增量空跑。
asset-compress:
	cd $(ASSET_COMPRESS_DIR) && cargo run --release -- --input ../../client/assets \
	  --quality $(ASSET_COMPRESS_QUALITY) --backend $(ASSET_COMPRESS_BACKEND)

# 工具自身的测试、静态检查与 60 张卡面的 BC7 质量门禁。
asset-compress-check:
	cd $(ASSET_COMPRESS_DIR) && cargo test --release && cargo clippy -- -D warnings
	cd $(ASSET_COMPRESS_DIR) && cargo run --release -- --input ../../client/assets/cards/faces \
	  --quality standard --backend gpu --check

# 桌面客户端统一从这里启动：先导入刷新全局类缓存。新增或改名 class_name
# 脚本后直接跑 Godot 会沿用旧缓存，缺类的脚本编译失败，对局页可能只构建一半。
# 模拟不同尺寸设备：make client-run WIDTH=960 HEIGHT=540；两者需同时为正整数，缺省按屏幕自动收拢。
# 模拟设备档位：make client-run CLIENT_DEVICE=phone；phone / tablet / desktop，其余值按未指定处理。
# 本地服务：LOCAL_SERVER=1 时由客户端内嵌启动 room-service 与 7 个机器人（sidecar 取
# server/target/debug），服务端就绪后自动登录并自动入房，退出客户端时一并回收。
WIDTH ?=
HEIGHT ?=
CLIENT_DEVICE ?=
LOCAL_SERVER ?= 0
SIDECAR_DIR ?= $(CURDIR)/server/target/debug

client-run: client-import $(if $(filter 1,$(LOCAL_SERVER)),m1b-build)
	@if [ -z "$(WIDTH)" ] && [ -z "$(HEIGHT)" ]; then \
	  CIVILIZATION_LOCAL_SERVER="$(LOCAL_SERVER)" CIVILIZATION_LOCAL_SERVER_DIR="$(SIDECAR_DIR)" CIVILIZATION_DEVICE="$(CLIENT_DEVICE)" $(GODOT) --path client; \
	elif [ "$(WIDTH)" -gt 0 ] 2>/dev/null && [ "$(HEIGHT)" -gt 0 ] 2>/dev/null; then \
	  CIVILIZATION_LOCAL_SERVER="$(LOCAL_SERVER)" CIVILIZATION_LOCAL_SERVER_DIR="$(SIDECAR_DIR)" CIVILIZATION_WINDOW_SIZE="$(WIDTH)x$(HEIGHT)" CIVILIZATION_DEVICE="$(CLIENT_DEVICE)" $(GODOT) --path client; \
	else \
	  echo "WIDTH 与 HEIGHT 需同时传入正整数：make client-run WIDTH=960 HEIGHT=540" >&2; \
	  exit 1; \
	fi

# 设备档别名：同时切换窗口尺寸、设备档案（布局基准、控件尺寸、触控输入）并带上本地服务端与机器人。
# 目标名可被 shell 自带的 make 补全列出，`make client-run-on-<Tab>` 即可选择。
# 尺寸取项目验收用机的横屏物理分辨率：iPhone 17 Pro、iPad Air 11。
client-run-on-iphone:
	$(MAKE) client-run WIDTH=2622 HEIGHT=1206 CLIENT_DEVICE=phone LOCAL_SERVER=1

client-run-on-ipad:
	$(MAKE) client-run WIDTH=2360 HEIGHT=1640 CLIENT_DEVICE=tablet LOCAL_SERVER=1

client-run-on-desktop:
	$(MAKE) client-run CLIENT_DEVICE=desktop LOCAL_SERVER=1

# 单元测试筛选：TESTS=模块名（逗号或空格分隔）只跑对应 unittest，
# 如 make client-check TESTS=test_i18n；指定时跳过 protocol-check、data-check 与 client-import。
TESTS ?=
client-check:
	@if [ -n "$(TESTS)" ]; then \
	  for m in $$(echo "$(TESTS)" | tr ',' ' '); do \
	    case "$$m" in tests.*) ;; *) m="tests.$$m" ;; esac; \
	    $(PYTHON) -m unittest "$$m" -v || exit 1; \
	  done; \
	else \
	  $(PYTHON) scripts/run_parallel.py $(if $(JOBS),--jobs $(JOBS),) \
	    "$(MAKE) --no-print-directory protocol-check" \
	    "$(MAKE) --no-print-directory data-check" \
	    "$(PYTHON) -m unittest tests.test_client_boundary -v" \
	    "$(PYTHON) -m unittest tests.test_client_style -v" \
	    "$(PYTHON) -m unittest tests.test_i18n -v" && \
	  $(MAKE) --no-print-directory client-import; \
	fi

# 冒烟场景筛选：SMOKE=场景名（逗号或空格分隔，省略 ui_smoke_ 前缀）只跑对应场景，
# 如 make client-smoke SMOKE=combat；缺省跑 client/tests/ui_smoke*.tscn 全部。
SMOKE ?=
client-smoke: client-import
	@mkdir -p output
	@if [ -n "$(SMOKE)" ]; then \
	  for name in $$(echo "$(SMOKE)" | tr ',' ' '); do \
	    scene="client/tests/ui_smoke_$$name.tscn"; \
	    [ -f "$$scene" ] || { echo "未找到冒烟场景：$$scene" >&2; exit 1; }; \
	    $(GODOT) --headless --path client res://tests/ui_smoke_$$name.tscn 2>&1 | tee output/ui_smoke_$$name.log; \
	    grep -q "UI_SMOKE_OK" output/ui_smoke_$$name.log || exit 1; \
	    ! grep -q "SCRIPT ERROR" output/ui_smoke_$$name.log || exit 1; \
	  done; \
	else \
	  for scene in client/tests/ui_smoke*.tscn; do \
	    name=$$(basename $$scene .tscn); \
	    $(GODOT) --headless --path client res://tests/$$name.tscn 2>&1 | tee output/$$name.log; \
	    grep -q "UI_SMOKE_OK" output/$$name.log || exit 1; \
	    ! grep -q "SCRIPT ERROR" output/$$name.log || exit 1; \
	  done; \
	fi

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

# 运行态视图预览：真实窗口装载页面/场景并注入夹具；不截图，窗口保持打开。
# PAGE 取 client/pages 下的页面名；SCENE 直接给 res:// 场景；FOCUS 高亮同名子节点；
# FOCUS_PATH 指定节点路径；FIXTURE=game 注入八人对局视图；SETUP 指定自定义构建脚本；
# DEVICE=phone 切手机档位；EXPECT=SeatRow=8 断言节点数量（配合 ui-preview-check）。
SCENE ?=
FOCUS ?=
FOCUS_PATH ?=
FIXTURE ?=
SETUP ?=
EXPECT ?=
PREVIEW_FRAMES ?= 12

ui-preview: client-import
	@if [ -z "$(PAGE)" ] && [ -z "$(SCENE)" ] && [ -z "$(SETUP)" ]; then \
	  echo "用法：make ui-preview PAGE=game [FOCUS=PlayersPanel] [FIXTURE=game] [DEVICE=phone]" >&2; \
	  echo "      make ui-preview SCENE=res://pages/game/game.tscn；组件用 SETUP=res://tests/previews/xxx.gd" >&2; \
	  exit 1; \
	fi
	$(GODOT) --path client res://tests/view_preview.tscn -- \
		$(if $(PAGE),--preview-page=$(PAGE),) \
		$(if $(SCENE),--preview-scene=$(SCENE),) \
		$(if $(FOCUS),--preview-focus=$(FOCUS),) \
		$(if $(FOCUS_PATH),--preview-focus-path=$(FOCUS_PATH),) \
		$(if $(FIXTURE),--preview-fixture=$(FIXTURE),) \
		$(if $(SETUP),--preview-setup=$(SETUP),) \
		$(if $(DEVICE),--preview-device=$(DEVICE),) \
		$(if $(EXPECT),--preview-expect=$(EXPECT),) \
		--preview-frames=$(PREVIEW_FRAMES)

# 预览自检：窗口自动退出，按 VIEW_PREVIEW_* 日志判定；只用于本机 GUI 冒烟，不接入 CI。
ui-preview-check: client-import
	@if [ -z "$(PAGE)" ] && [ -z "$(SCENE)" ] && [ -z "$(SETUP)" ]; then \
	  echo "用法：make ui-preview-check PAGE=game FOCUS=PlayersPanel EXPECT=SeatRow=8" >&2; \
	  exit 1; \
	fi
	@mkdir -p output
	@$(GODOT) --path client --quit-after 900 res://tests/view_preview.tscn -- \
		$(if $(PAGE),--preview-page=$(PAGE),) \
		$(if $(SCENE),--preview-scene=$(SCENE),) \
		$(if $(FOCUS),--preview-focus=$(FOCUS),) \
		$(if $(FOCUS_PATH),--preview-focus-path=$(FOCUS_PATH),) \
		$(if $(FIXTURE),--preview-fixture=$(FIXTURE),) \
		$(if $(SETUP),--preview-setup=$(SETUP),) \
		$(if $(DEVICE),--preview-device=$(DEVICE),) \
		$(if $(EXPECT),--preview-expect=$(EXPECT),) \
		--preview-frames=$(PREVIEW_FRAMES) --preview-quit 2>&1 | tee output/ui_preview.log
	@grep -q "VIEW_PREVIEW_OK" output/ui_preview.log
	@! grep -qE "VIEW_PREVIEW_FAIL|SCRIPT ERROR" output/ui_preview.log

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

# 客户端导出环境：local / dev / prod，默认 local。ENV 与 env 等价，同时传且值不同直接失败。
# local 写空配置，客户端运行时按平台回落本机调试地址；dev 写开发公网入口；prod 读正式地址
# （当前为空，配置前导出直接失败，避免把开发包当正式包）。
CLIENT_DEV_SERVER_URL ?= wss://civilization.satori.host/ws
CLIENT_PROD_SERVER_URL ?=
CLIENT_ENV = $(if $(and $(filter command line environment,$(origin ENV)),$(strip $(ENV))),$(strip $(ENV)),$(if $(and $(filter command line environment,$(origin env)),$(strip $(env))),$(strip $(env)),local))
CLIENT_SERVER_URL = $(if $(filter prod,$(CLIENT_ENV)),$(CLIENT_PROD_SERVER_URL),$(if $(filter dev,$(CLIENT_ENV)),$(CLIENT_DEV_SERVER_URL),))
CLIENT_REQUIRE_SERVER = $(if $(filter prod,$(CLIENT_ENV)),--require-server,$(if $(filter dev,$(CLIENT_ENV)),--require-url,))

client-env-check:
	@$(if $(and $(filter command line environment,$(origin ENV)),$(filter command line environment,$(origin env)),$(strip $(ENV)),$(strip $(env))),$(if $(filter $(strip $(ENV)),$(strip $(env))),,$(error ENV 与 env 不能同时传不同值：ENV=$(ENV) env=$(env))),)
	@$(if $(filter local dev prod,$(CLIENT_ENV)),,$(error 客户端构建 ENV 只能是 local / dev / prod，收到：$(CLIENT_ENV)))
	@$(if $(filter dev,$(CLIENT_ENV)),$(if $(strip $(CLIENT_DEV_SERVER_URL)),,$(error ENV=dev 需要 CLIENT_DEV_SERVER_URL（ws:// 或 wss://...），当前为空)),)
	@$(if $(filter prod,$(CLIENT_ENV)),$(if $(strip $(CLIENT_PROD_SERVER_URL)),,$(error ENV=prod 需要 CLIENT_PROD_SERVER_URL=wss://...，当前为空)),)

# 未签名 Universal .app 与 zip 产物。Gatekeeper 放行方式见 docs/current/technical/client.md。
client-export: client-env-check client-export-templates client-import
	@[ "$$(uname -s)" = "Darwin" ] || { echo "client-export 仅支持 macOS（当前 $$(uname -s)）" >&2; exit 1; }
	@set -e; \
	backup=$$(mktemp); \
	cp client/generated/build_config.gd "$$backup"; \
	trap 'cp "$$backup" client/generated/build_config.gd; rm -f "$$backup"' EXIT; \
	SERVER_URL="$(CLIENT_SERVER_URL)" STEAM_APP_ID="" $(PYTHON) scripts/gen_build_config.py $(CLIENT_REQUIRE_SERVER); \
	mkdir -p output/desktop; \
	rm -rf output/desktop/Civilization.app output/desktop/Civilization-macos.zip; \
	"$(GODOT)" --headless --path client --export-release "macOS" "$(CURDIR)/output/desktop/Civilization.app"; \
	cp "$$backup" client/generated/build_config.gd; \
	rm -f "$$backup"; \
	trap - EXIT; \
	test -x output/desktop/Civilization.app/Contents/MacOS/Civilization; \
	ditto -c -k --sequesterRsrc --keepParent output/desktop/Civilization.app output/desktop/Civilization-macos.zip

# 本地调试包：启用 embedded_server 特性，内嵌 room-service 与 sim，只用于本机自测。
client-export-local: client-export-templates client-import m1b-build
	@[ "$$(uname -s)" = "Darwin" ] || { echo "client-export-local 仅支持 macOS（当前 $$(uname -s)）" >&2; exit 1; }
	rm -rf output/desktop-local
	mkdir -p output/desktop-local
	"$(GODOT)" --headless --path client --export-debug "macOS Local Debug" "$(CURDIR)/output/desktop-local/Civilization.app"
	test -x output/desktop-local/Civilization.app/Contents/MacOS/Civilization
	cp server/target/debug/room-service server/target/debug/sim output/desktop-local/Civilization.app/Contents/MacOS/
	chmod +x output/desktop-local/Civilization.app/Contents/MacOS/room-service output/desktop-local/Civilization.app/Contents/MacOS/sim
	test -x output/desktop-local/Civilization.app/Contents/MacOS/room-service
	test -x output/desktop-local/Civilization.app/Contents/MacOS/sim
	@echo "本地调试产物：output/desktop-local/Civilization.app"

# Steam 发行构建参数：SERVER_URL 必须是 wss://，STEAM_APP_ID 为 Steamworks App ID。
SERVER_URL ?=
STEAM_APP_ID ?=
STEAM_DEPOT_ID ?=
STEAM_USER ?=
STEAM_OUTPUT ?= output/steam
STEAM_CMD ?= steamcmd
CODESIGN_IDENTITY ?=
NOTARY_PROFILE ?=

# Steam 发行构建：临时写入发布配置后导出，退出时恢复仓库内的空配置。
# 校验产物内的 libsteam_api.dylib 同时包含 x86_64 与 arm64，避免产出不可用的 universal 包。
client-export-steam: client-export-templates client-import
	@[ "$$(uname -s)" = "Darwin" ] || { echo "client-export-steam 仅支持 macOS（当前 $$(uname -s)）" >&2; exit 1; }
	@set -e; \
	backup=$$(mktemp); \
	cp client/generated/build_config.gd "$$backup"; \
	trap 'cp "$$backup" client/generated/build_config.gd; rm -f "$$backup"' EXIT; \
	SERVER_URL="$(SERVER_URL)" STEAM_APP_ID="$(STEAM_APP_ID)" $(PYTHON) scripts/gen_build_config.py --require; \
	mkdir -p "$(STEAM_OUTPUT)"; \
	rm -rf "$(STEAM_OUTPUT)/Civilization.app"; \
	"$(GODOT)" --headless --path client --export-release "Steam" "$(CURDIR)/$(STEAM_OUTPUT)/Civilization.app"; \
	test -x "$(STEAM_OUTPUT)/Civilization.app/Contents/MacOS/Civilization"; \
	lib="$(STEAM_OUTPUT)/Civilization.app/Contents/Frameworks/libsteam_api.dylib"; \
	[ -f "$$lib" ] || { echo "缺少 libsteam_api.dylib：$$lib" >&2; exit 1; }; \
	archs="$$(lipo -info "$$lib")"; \
	echo "$$archs"; \
	case "$$archs" in \
	  *x86_64*arm64*|*arm64*x86_64*) ;; \
	  *) echo "libsteam_api.dylib 必须是 universal（x86_64 + arm64）" >&2; exit 1 ;; \
	esac; \
	echo "Steam 产物：$(STEAM_OUTPUT)/Civilization.app"

# 签名与公证：先签嵌套 dylib（Steam 覆盖层由 Steam 客户端注入，需关闭库校验），
# 再签 .app；公证通过后 staple 票据，最后交给 Gatekeeper 复核。
client-notarize:
	@[ "$$(uname -s)" = "Darwin" ] || { echo "client-notarize 仅支持 macOS（当前 $$(uname -s)）" >&2; exit 1; }
	@[ -d "$(STEAM_OUTPUT)/Civilization.app" ] || { echo "缺少 $(STEAM_OUTPUT)/Civilization.app，先运行 make client-export-steam" >&2; exit 1; }
	@[ -n "$(CODESIGN_IDENTITY)" ] || { echo "需要 CODESIGN_IDENTITY，例如 'Developer ID Application: 名称 (TEAMID)'" >&2; exit 1; }
	@[ -n "$(NOTARY_PROFILE)" ] || { echo "需要 NOTARY_PROFILE（用 xcrun notarytool store-credentials 创建）" >&2; exit 1; }
	@set -e; \
	app="$(STEAM_OUTPUT)/Civilization.app"; \
	for lib in "$$app/Contents/Frameworks/"*.dylib; do \
	  codesign --force --timestamp --options runtime --sign "$(CODESIGN_IDENTITY)" "$$lib"; \
	done; \
	codesign --force --timestamp --options runtime \
	  --entitlements deploy/steam/entitlements.plist \
	  --sign "$(CODESIGN_IDENTITY)" "$$app"; \
	codesign --verify --deep --strict --verbose=2 "$$app"; \
	rm -f "$(STEAM_OUTPUT)/Civilization-steam.zip"; \
	ditto -c -k --sequesterRsrc --keepParent "$$app" "$(STEAM_OUTPUT)/Civilization-steam.zip"; \
	xcrun notarytool submit "$(STEAM_OUTPUT)/Civilization-steam.zip" --keychain-profile "$(NOTARY_PROFILE)" --wait; \
	xcrun stapler staple "$$app"; \
	xcrun stapler validate "$$app"; \
	spctl -a -vv -t exec "$$app"; \
	echo "签名与公证完成：$$app"

# 上传到 Steam：用 deploy/steam 的 VDF 模板渲染到输出目录，内容根指向已签名的 .app。
# steamcmd 的密码由交互输入，不写入仓库或命令行。
steam-upload:
	@[ -n "$(STEAM_APP_ID)" ] || { echo "需要 STEAM_APP_ID" >&2; exit 1; }
	@[ -n "$(STEAM_DEPOT_ID)" ] || { echo "需要 STEAM_DEPOT_ID" >&2; exit 1; }
	@[ -n "$(STEAM_USER)" ] || { echo "需要 STEAM_USER" >&2; exit 1; }
	@[ -d "$(STEAM_OUTPUT)/Civilization.app" ] || { echo "缺少 $(STEAM_OUTPUT)/Civilization.app，先运行 make client-export-steam" >&2; exit 1; }
	@codesign --verify --deep --strict "$(STEAM_OUTPUT)/Civilization.app" 2>/dev/null || { echo "产物未签名或签名无效，先运行 make client-notarize" >&2; exit 1; }
	@mkdir -p "$(STEAM_OUTPUT)/build"
	sed -e 's|__APP_ID__|$(STEAM_APP_ID)|g' -e 's|__DEPOT_ID__|$(STEAM_DEPOT_ID)|g' \
	  -e 's|__CONTENT_ROOT__|$(CURDIR)/$(STEAM_OUTPUT)|g' \
	  -e 's|__BUILD_OUTPUT__|$(CURDIR)/$(STEAM_OUTPUT)/build|g' \
	  deploy/steam/app_build.vdf > "$(STEAM_OUTPUT)/app_build.vdf"
	sed -e 's|__DEPOT_ID__|$(STEAM_DEPOT_ID)|g' \
	  -e 's|__CONTENT_ROOT__|$(CURDIR)/$(STEAM_OUTPUT)|g' \
	  deploy/steam/depot_build.vdf > "$(STEAM_OUTPUT)/depot_build.vdf"
	$(STEAM_CMD) +login "$(STEAM_USER)" +run_app_build "$(CURDIR)/$(STEAM_OUTPUT)/app_build.vdf" +quit

# iOS 真机开发签名包：先导出 Xcode 工程，再由 xcodebuild 自动签名并打包 .ipa。
# 需要 Xcode 已登录 Apple 团队 Q62EFJPV3C，且目标真机已登记；否则 xcodebuild 失败。
client-export-ios: client-env-check client-export-templates client-import
	@[ "$$(uname -s)" = "Darwin" ] || { echo "client-export-ios 仅支持 macOS（当前 $$(uname -s)）" >&2; exit 1; }
	@[ -f "$(GODOT_TEMPLATE_DIR)/ios.zip" ] || { echo "缺少 iOS 导出模板：$(GODOT_TEMPLATE_DIR)/ios.zip" >&2; exit 1; }
	@set -e; \
	backup=$$(mktemp); \
	cp client/generated/build_config.gd "$$backup"; \
	trap 'cp "$$backup" client/generated/build_config.gd; rm -f "$$backup"' EXIT; \
	SERVER_URL="$(CLIENT_SERVER_URL)" STEAM_APP_ID="" $(PYTHON) scripts/gen_build_config.py $(CLIENT_REQUIRE_SERVER); \
	mkdir -p output/ios; \
	rm -rf output/ios/Civilization output/ios/Civilization.xcodeproj output/ios/Civilization.xcframework \
	  output/ios/MoltenVK.xcframework output/ios/libgodot_camera.*.xcframework \
	  output/ios/Civilization.pck output/ios/Payload output/ios/Civilization-ios.ipa; \
	"$(GODOT)" --headless --path client --export-release "iOS" "$(CURDIR)/output/ios/Civilization.xcodeproj"; \
	cp "$$backup" client/generated/build_config.gd; \
	rm -f "$$backup"; \
	trap - EXIT; \
	test -d output/ios/Civilization.xcodeproj; \
	xcodebuild -project output/ios/Civilization.xcodeproj -scheme Civilization -configuration Release \
	  -sdk iphoneos -destination 'generic/platform=iOS' -allowProvisioningUpdates \
	  -derivedDataPath output/ios/build build; \
	test -d output/ios/build/Build/Products/Release-iphoneos/Civilization.app; \
	mkdir -p output/ios/Payload; \
	ditto output/ios/build/Build/Products/Release-iphoneos/Civilization.app output/ios/Payload/Civilization.app; \
	ditto -c -k --sequesterRsrc --keepParent output/ios/Payload output/ios/Civilization-ios.ipa; \
	test -f output/ios/Civilization-ios.ipa; \
	echo "iOS 产物：output/ios/Civilization-ios.ipa"

# iOS 真机开发运行：导出并签名后安装到 iPhone 并启动。
# 默认取第一台已连接的 iPhone；IOS_DEVICE 用设备名或 UDID 指定，IOS_LAUNCH=0 只安装。
IOS_DEVICE ?=
IOS_LAUNCH ?= 1

# 真机联调默认走 local：空配置，客户端按移动端回落开发机局域网地址。
client-run-ios: client-export-ios
	IOS_DEVICE="$(IOS_DEVICE)" IOS_LAUNCH="$(IOS_LAUNCH)" bash scripts/ios_run.sh

# Android 签名参数只从环境变量读取：keystore 与口令不入库，也不写进命令行。
ANDROID_KEYSTORE_PATH ?=
ANDROID_KEYSTORE_USER ?=
ANDROID_KEYSTORE_PASSWORD ?=
ANDROID_OUTPUT ?= output/android
export ANDROID_KEYSTORE_PATH ANDROID_KEYSTORE_USER ANDROID_KEYSTORE_PASSWORD

# Android 侧载 APK：arm64、横屏、包名 com.civilization.banner，release keystore 签名。
# 缺少任一 ANDROID_KEYSTORE_* 或 keystore 文件时直接失败，不产出未签名包。
client-export-android: client-env-check client-export-templates client-import
	@[ -f "$(GODOT_TEMPLATE_DIR)/android_release.apk" ] || { echo "缺少 Android 导出模板：$(GODOT_TEMPLATE_DIR)/android_release.apk" >&2; exit 1; }
	@[ -n "$$ANDROID_KEYSTORE_PATH" ] || { echo "需要 ANDROID_KEYSTORE_PATH（release keystore 路径）" >&2; exit 1; }
	@[ -n "$$ANDROID_KEYSTORE_USER" ] || { echo "需要 ANDROID_KEYSTORE_USER（keystore alias）" >&2; exit 1; }
	@[ -n "$$ANDROID_KEYSTORE_PASSWORD" ] || { echo "需要 ANDROID_KEYSTORE_PASSWORD（keystore 口令）" >&2; exit 1; }
	@[ -f "$$ANDROID_KEYSTORE_PATH" ] || { echo "Android keystore 不存在：$$ANDROID_KEYSTORE_PATH" >&2; exit 1; }
	@set -e; \
	backup=$$(mktemp); \
	cp client/generated/build_config.gd "$$backup"; \
	trap 'cp "$$backup" client/generated/build_config.gd; rm -f "$$backup"' EXIT; \
	SERVER_URL="$(CLIENT_SERVER_URL)" STEAM_APP_ID="" $(PYTHON) scripts/gen_build_config.py $(CLIENT_REQUIRE_SERVER); \
	GODOT="$(GODOT)" GODOT_TEMPLATE_DIR="$(GODOT_TEMPLATE_DIR)" ANDROID_OUTPUT="$(ANDROID_OUTPUT)" \
	  bash scripts/android_export.sh apk

# Android 商店 AAB：与 APK 同一 keystore 与 arm64 预设，走 Gradle 构建。
# 首次导出会在 client/android/build 安装构建模板，导出后由脚本清理。
client-export-android-aab: client-env-check client-export-templates client-import
	@[ -f "$(GODOT_TEMPLATE_DIR)/android_source.zip" ] || { echo "缺少 Android 构建模板：$(GODOT_TEMPLATE_DIR)/android_source.zip" >&2; exit 1; }
	@[ -n "$$ANDROID_KEYSTORE_PATH" ] || { echo "需要 ANDROID_KEYSTORE_PATH（release keystore 路径）" >&2; exit 1; }
	@[ -n "$$ANDROID_KEYSTORE_USER" ] || { echo "需要 ANDROID_KEYSTORE_USER（keystore alias）" >&2; exit 1; }
	@[ -n "$$ANDROID_KEYSTORE_PASSWORD" ] || { echo "需要 ANDROID_KEYSTORE_PASSWORD（keystore 口令）" >&2; exit 1; }
	@[ -f "$$ANDROID_KEYSTORE_PATH" ] || { echo "Android keystore 不存在：$$ANDROID_KEYSTORE_PATH" >&2; exit 1; }
	@set -e; \
	backup=$$(mktemp); \
	cp client/generated/build_config.gd "$$backup"; \
	trap 'cp "$$backup" client/generated/build_config.gd; rm -f "$$backup"' EXIT; \
	SERVER_URL="$(CLIENT_SERVER_URL)" STEAM_APP_ID="" $(PYTHON) scripts/gen_build_config.py $(CLIENT_REQUIRE_SERVER); \
	GODOT="$(GODOT)" GODOT_TEMPLATE_DIR="$(GODOT_TEMPLATE_DIR)" ANDROID_OUTPUT="$(ANDROID_OUTPUT)" \
	  bash scripts/android_export.sh aab

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
