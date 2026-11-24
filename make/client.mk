# 客户端构建、运行、检查与导出。由根 Makefile include。
# 依赖根 Makefile 的 PYTHON、JOBS、CURDIR，以及 make/server.mk 的 server-build 目标。

GODOT ?= $(shell command -v godot 2>/dev/null || echo /Applications/Godot.app/Contents/MacOS/Godot)
# 冒烟、预览与线框图直接读仓库内生成的下发内容，不依赖真实服务端。
CONTENT_DIR := $(CURDIR)/server/data/v0.1
CLIENT_CONTENT_ENV := CIVILIZATION_CONTENT_DIR="$(CONTENT_DIR)"
# 补丁版本固定，导出模板目录与之精确一致；本机引擎不符时由 client-godot-check 拒绝执行。
GODOT_VERSION := 4.7.2
GODOT_INSTALLED_VERSION = $(shell "$(GODOT)" --version 2>/dev/null | head -n 1 | cut -d. -f1-3)
GODOT_TEMPLATE_DIR := $(HOME)/Library/Application Support/Godot/export_templates/$(GODOT_VERSION).stable
# 导入压缩策略，默认 0（headless + CPU 压缩）：auto=有图形会话时非无头导入（Betsy 优先用 GPU，失败回落 CPU）；
# 1=强制非无头。headless 下引擎不允许创建 RenderingDevice，GPU 压缩不可用。
CLIENT_IMPORT_GPU ?= 0

.PHONY: client-godot-check client-import client-cache-copy client-check client-smoke \
	client-run client-run-platform-check client-run-check client-run-ios-check _client-run-macos _client-run-ios _client-run-invalid \
	client-env-check client-export-templates client-export client-export-platform-check \
	_client-export-macos _client-export-local _client-export-steam _client-export-ios \
	_client-export-android _client-export-invalid client-export-android-check client-notarize steam-upload \
	ui-wireframe ui-preview ui-preview-check \
	run export run-iphone run-ipad run-phone run-tablet run-desktop run-ios \
	export-macos export-local export-steam export-ios export-android export-aab \
	export-macos-local export-macos-dev export-macos-prod \
	export-ios-local export-ios-dev export-ios-prod \
	export-ipad-local export-ipad-dev export-ipad-prod \
	export-android-local export-android-dev export-android-prod

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
	    $(CLIENT_CONTENT_ENV) $(GODOT) --headless --path client res://tests/ui_smoke_$$name.tscn 2>&1 | tee output/ui_smoke_$$name.log; \
	    grep -q "UI_SMOKE_OK" output/ui_smoke_$$name.log || exit 1; \
	    ! grep -q "SCRIPT ERROR" output/ui_smoke_$$name.log || exit 1; \
	  done; \
	else \
	  for scene in client/tests/ui_smoke*.tscn; do \
	    name=$$(basename $$scene .tscn); \
	    $(CLIENT_CONTENT_ENV) $(GODOT) --headless --path client res://tests/$$name.tscn 2>&1 | tee output/$$name.log; \
	    grep -q "UI_SMOKE_OK" output/$$name.log || exit 1; \
	    ! grep -q "SCRIPT ERROR" output/$$name.log || exit 1; \
	  done; \
	fi

# 客户端构建环境：local / dev / prod，默认 local。ENV 与 env 等价，同时传且值不同直接失败。
# local 写空配置，客户端运行时按平台回落本机调试地址；dev 写开发公网入口；prod 读正式地址
# （当前为空，配置前运行或导出直接失败，避免把开发包当正式包）。
# 定义放在运行参数之前：client-run 的前置依赖在解析期就要读取 CLIENT_ENV。
CLIENT_DEV_SERVER_URL ?= wss://civilization.satori.host/ws
CLIENT_PROD_SERVER_URL ?=
CLIENT_ENV = $(if $(and $(filter command line environment,$(origin ENV)),$(strip $(ENV))),$(strip $(ENV)),$(if $(and $(filter command line environment,$(origin env)),$(strip $(env))),$(strip $(env)),local))
CLIENT_SERVER_URL = $(if $(filter prod,$(CLIENT_ENV)),$(CLIENT_PROD_SERVER_URL),$(if $(filter dev,$(CLIENT_ENV)),$(CLIENT_DEV_SERVER_URL),))
CLIENT_REQUIRE_SERVER = $(if $(filter prod,$(CLIENT_ENV)),--require-server,$(if $(filter dev,$(CLIENT_ENV)),--require-url,))

# 运行参数：DEVICE 是设备档位（desktop / tablet / phone）或整机预设（iphone / ipad，
# 同时给出验收机横屏尺寸）。WIDTH/HEIGHT 显式传入时覆盖预设尺寸，两者必须同时给出。
# 显式传入 DEVICE 且环境为 local 时默认带上本地服务端与机器人；dev/prod 与不传 DEVICE 时关闭。
WIDTH ?=
HEIGHT ?=
DEVICE ?=
LOCAL_SERVER ?= $(if $(strip $(DEVICE)),$(if $(filter local,$(CLIENT_ENV)),1,0),0)
SIDECAR_DIR ?= $(CURDIR)/server/target/debug

# 平台入口：client-run 支持 macos（编辑器）与 ios（真机）；client-export 支持全部发行平台。
PLATFORM ?= macos
CLIENT_PLATFORM := $(strip $(PLATFORM))
CLIENT_RUN_PLATFORMS := macos ios
CLIENT_EXPORT_PLATFORMS := macos local steam ios android
AAB ?= 0

CLIENT_RUN_DEVICE := $(if $(strip $(DEVICE)),$(strip $(DEVICE)),desktop)
CLIENT_RUN_PROFILE := $(if $(filter iphone,$(CLIENT_RUN_DEVICE)),phone,$(if $(filter ipad,$(CLIENT_RUN_DEVICE)),tablet,$(CLIENT_RUN_DEVICE)))
CLIENT_RUN_WIDTH := $(if $(strip $(WIDTH)),$(strip $(WIDTH)),$(if $(filter iphone,$(CLIENT_RUN_DEVICE)),2622,$(if $(filter ipad,$(CLIENT_RUN_DEVICE)),2360,)))
CLIENT_RUN_HEIGHT := $(if $(strip $(HEIGHT)),$(strip $(HEIGHT)),$(if $(filter iphone,$(CLIENT_RUN_DEVICE)),1206,$(if $(filter ipad,$(CLIENT_RUN_DEVICE)),1640,)))
CLIENT_RUN_TARGET := _client-run-$(if $(filter $(CLIENT_RUN_PLATFORMS),$(CLIENT_PLATFORM)),$(CLIENT_PLATFORM),invalid)
CLIENT_EXPORT_TARGET := _client-export-$(if $(filter $(CLIENT_EXPORT_PLATFORMS),$(CLIENT_PLATFORM)),$(CLIENT_PLATFORM),invalid)

# 桌面客户端统一从这里启动：先导入刷新全局类缓存。新增或改名 class_name
# 脚本后直接跑 Godot 会沿用旧缓存，缺类的脚本编译失败，对局页可能只构建一半。
#   make client-run                                  # 默认 desktop，不启本地服务
#   make client-run DEVICE=iphone                    # 手机档位 + 验收尺寸 + 本地服务
#   make client-run DEVICE=ipad LOCAL_SERVER=0       # 平板尺寸，不启本地服务
#   make client-run WIDTH=960 HEIGHT=540             # 自定义窗口
#   make client-run PLATFORM=ios                     # 真机安装运行
#   make run-desktop-dev                             # 桌面编辑器连开发服
#   make run-ios-ipad-dev                            # 导出并安装到 iPad，连开发服
client-run: client-run-platform-check client-run-check $(CLIENT_RUN_TARGET)

client-run-platform-check:
	@$(if $(filter $(CLIENT_RUN_PLATFORMS),$(CLIENT_PLATFORM)),,$(error client-run 的 PLATFORM 只能是 macos / ios，收到：$(CLIENT_PLATFORM)))

client-run-check:
	@$(if $(filter desktop iphone ipad phone tablet,$(CLIENT_RUN_DEVICE)),,$(error DEVICE 只能是 desktop / iphone / ipad / phone / tablet，收到：$(CLIENT_RUN_DEVICE)))
	@$(if $(filter 0 1,$(LOCAL_SERVER)),,$(error LOCAL_SERVER 只能是 0 或 1，收到：$(LOCAL_SERVER)))
	@$(if $(strip $(WIDTH)),$(if $(strip $(HEIGHT)),,$(error WIDTH 与 HEIGHT 需同时传入正整数)),$(if $(strip $(HEIGHT)),$(error WIDTH 与 HEIGHT 需同时传入正整数),))

# 编辑器运行接受 ENV：local 写空配置并回落本机地址；dev/prod 在启动前临时写入对应
# BuildConfig.SERVER_URL，退出后由 EXIT trap 恢复。dev/prod 默认不启本地服务端。
_client-run-macos: client-env-check client-import $(if $(filter 1,$(LOCAL_SERVER)),server-build)
	@set -e; \
	if [ "$(CLIENT_ENV)" != "local" ]; then $(CLIENT_BACKUP_CONFIG); fi; \
	SERVER_URL="$(CLIENT_SERVER_URL)" STEAM_APP_ID="" $(PYTHON) scripts/gen_build_config.py $(CLIENT_REQUIRE_SERVER); \
	w="$(CLIENT_RUN_WIDTH)"; h="$(CLIENT_RUN_HEIGHT)"; \
	if [ -n "$$w" ] || [ -n "$$h" ]; then \
	  case "$$w" in ''|*[!0-9]*) echo "WIDTH 与 HEIGHT 需同时传入正整数：make client-run WIDTH=960 HEIGHT=540" >&2; exit 1 ;; esac; \
	  case "$$h" in ''|*[!0-9]*) echo "WIDTH 与 HEIGHT 需同时传入正整数：make client-run WIDTH=960 HEIGHT=540" >&2; exit 1 ;; esac; \
	  [ "$$w" -gt 0 ] && [ "$$h" -gt 0 ] || { echo "WIDTH 与 HEIGHT 需同时传入正整数：make client-run WIDTH=960 HEIGHT=540" >&2; exit 1; }; \
	  CIVILIZATION_LOCAL_SERVER="$(LOCAL_SERVER)" CIVILIZATION_LOCAL_SERVER_DIR="$(SIDECAR_DIR)" \
	    CIVILIZATION_WINDOW_SIZE="$(CLIENT_RUN_WIDTH)x$(CLIENT_RUN_HEIGHT)" CIVILIZATION_DEVICE="$(CLIENT_RUN_PROFILE)" $(GODOT) --path client; \
	else \
	  CIVILIZATION_LOCAL_SERVER="$(LOCAL_SERVER)" CIVILIZATION_LOCAL_SERVER_DIR="$(SIDECAR_DIR)" \
	    CIVILIZATION_DEVICE="$(CLIENT_RUN_PROFILE)" $(GODOT) --path client; \
	fi

# 真机联调默认走 local：空配置，客户端按移动端回落开发机局域网地址。
# 环境由 ENV 指定；设备类型交给 ios_run.sh 自动选已连接的同类型设备。
client-run-ios-check:
	@$(if $(filter iphone ipad,$(IOS_DEVICE_TYPE)),,$(error IOS_DEVICE_TYPE 只能是 iphone / ipad，收到：$(IOS_DEVICE_TYPE)))

_client-run-ios: client-run-ios-check client-export
	IOS_DEVICE="$(IOS_DEVICE)" IOS_DEVICE_TYPE="$(IOS_DEVICE_TYPE)" IOS_LAUNCH="$(IOS_LAUNCH)" bash scripts/ios_run.sh

_client-run-invalid:
	@exit 2

# 页面线框图：无头装载真实场景解算布局，导出 SVG 与 PNG 到 output/wireframes/。
# PAGE 取 client/pages 下的页面名（gallery、room 等）；ALL=1 生成全部；DEVICE=phone 出手机档位。
PAGE ?=

ui-wireframe: client-import
	@if [ -z "$(PAGE)" ] && [ "$(ALL)" != "1" ]; then \
	  echo "用法：make ui-wireframe PAGE=gallery [DEVICE=phone]，或 make ui-wireframe ALL=1" >&2; \
	  exit 1; \
	fi
	$(CLIENT_CONTENT_ENV) $(PYTHON) scripts/ui_wireframe.py --godot "$(GODOT)" \
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
	$(CLIENT_CONTENT_ENV) $(GODOT) --path client res://tests/view_preview.tscn -- \
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
	@$(CLIENT_CONTENT_ENV) $(GODOT) --path client --quit-after 900 res://tests/view_preview.tscn -- \
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

client-env-check:
	@$(if $(and $(filter command line environment,$(origin ENV)),$(filter command line environment,$(origin env)),$(strip $(ENV)),$(strip $(env))),$(if $(filter $(strip $(ENV)),$(strip $(env))),,$(error ENV 与 env 不能同时传不同值：ENV=$(ENV) env=$(env))),)
	@$(if $(filter local dev prod,$(CLIENT_ENV)),,$(error 客户端构建 ENV 只能是 local / dev / prod，收到：$(CLIENT_ENV)))
	@$(if $(filter dev,$(CLIENT_ENV)),$(if $(strip $(CLIENT_DEV_SERVER_URL)),,$(error ENV=dev 需要 CLIENT_DEV_SERVER_URL（ws:// 或 wss://...），当前为空)),)
	@$(if $(filter prod,$(CLIENT_ENV)),$(if $(strip $(CLIENT_PROD_SERVER_URL)),,$(error ENV=prod 需要 CLIENT_PROD_SERVER_URL=wss://...，当前为空)),)

# 构建配置的临时改写与恢复：所有导出共用，失败由 EXIT trap 兜底。
CLIENT_BACKUP_CONFIG = backup=$$(mktemp); cp client/generated/build_config.gd "$$backup"; trap 'cp "$$backup" client/generated/build_config.gd; rm -f "$$backup"' EXIT
CLIENT_RESTORE_CONFIG = cp "$$backup" client/generated/build_config.gd; rm -f "$$backup"; trap - EXIT

# 导出入口：PLATFORM 选择平台，ENV 选择服务端地址；各平台产物路径与签名方式见分目标。
#   make client-export                          # macOS .app + zip（默认 PLATFORM=macos）
#   make client-export PLATFORM=local           # 内嵌 room-service 的本地调试包
#   make client-export PLATFORM=steam SERVER_URL=wss://... STEAM_APP_ID=...
#   make client-export PLATFORM=ios ENV=dev
#   make client-export PLATFORM=android ENV=dev
#   make client-export PLATFORM=android AAB=1 ENV=dev
client-export: client-export-platform-check $(CLIENT_EXPORT_TARGET)

client-export-platform-check:
	@$(if $(filter $(CLIENT_EXPORT_PLATFORMS),$(CLIENT_PLATFORM)),,$(error client-export 的 PLATFORM 只能是 macos / local / steam / ios / android，收到：$(CLIENT_PLATFORM)))
	@$(if $(filter local steam,$(CLIENT_PLATFORM)),$(if $(filter command line environment,$(origin ENV))$(filter command line environment,$(origin env)),$(error PLATFORM=$(CLIENT_PLATFORM) 不支持 ENV：local 始终使用空配置，steam 请用 SERVER_URL),))

_client-export-invalid:
	@exit 2

# 未签名 Universal .app 与 zip 产物。Gatekeeper 放行方式见 docs/current/technical/client.md。
_client-export-macos: client-env-check client-export-templates client-import
	@[ "$$(uname -s)" = "Darwin" ] || { echo "client-export PLATFORM=macos 仅支持 macOS（当前 $$(uname -s)）" >&2; exit 1; }
	@set -e; $(CLIENT_BACKUP_CONFIG); \
	SERVER_URL="$(CLIENT_SERVER_URL)" STEAM_APP_ID="" $(PYTHON) scripts/gen_build_config.py $(CLIENT_REQUIRE_SERVER); \
	mkdir -p output/desktop; \
	rm -rf output/desktop/Civilization.app output/desktop/Civilization-macos.zip; \
	"$(GODOT)" --headless --path client --export-release "macOS" "$(CURDIR)/output/desktop/Civilization.app"; \
	$(CLIENT_RESTORE_CONFIG); \
	test -x output/desktop/Civilization.app/Contents/MacOS/Civilization; \
	ditto -c -k --sequesterRsrc --keepParent output/desktop/Civilization.app output/desktop/Civilization-macos.zip

# 本地调试包：启用 embedded_server 特性，内嵌 room-service 与 sim，只用于本机自测。
_client-export-local: client-export-templates client-import server-build
	@[ "$$(uname -s)" = "Darwin" ] || { echo "client-export PLATFORM=local 仅支持 macOS（当前 $$(uname -s)）" >&2; exit 1; }
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
_client-export-steam: client-export-templates client-import
	@[ "$$(uname -s)" = "Darwin" ] || { echo "client-export PLATFORM=steam 仅支持 macOS（当前 $$(uname -s)）" >&2; exit 1; }
	@set -e; $(CLIENT_BACKUP_CONFIG); \
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
	@[ -d "$(STEAM_OUTPUT)/Civilization.app" ] || { echo "缺少 $(STEAM_OUTPUT)/Civilization.app，先运行 make client-export PLATFORM=steam" >&2; exit 1; }
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
	@[ -d "$(STEAM_OUTPUT)/Civilization.app" ] || { echo "缺少 $(STEAM_OUTPUT)/Civilization.app，先运行 make client-export PLATFORM=steam" >&2; exit 1; }
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
_client-export-ios: client-env-check client-export-templates client-import
	@[ "$$(uname -s)" = "Darwin" ] || { echo "client-export PLATFORM=ios 仅支持 macOS（当前 $$(uname -s)）" >&2; exit 1; }
	@[ -f "$(GODOT_TEMPLATE_DIR)/ios.zip" ] || { echo "缺少 iOS 导出模板：$(GODOT_TEMPLATE_DIR)/ios.zip" >&2; exit 1; }
	@set -e; $(CLIENT_BACKUP_CONFIG); \
	SERVER_URL="$(CLIENT_SERVER_URL)" STEAM_APP_ID="" $(PYTHON) scripts/gen_build_config.py $(CLIENT_REQUIRE_SERVER); \
	mkdir -p output/ios; \
	rm -rf output/ios/Civilization output/ios/Civilization.xcodeproj output/ios/Civilization.xcframework \
	  output/ios/MoltenVK.xcframework output/ios/libgodot_camera.*.xcframework \
	  output/ios/Civilization.pck output/ios/Payload output/ios/Civilization-ios.ipa; \
	"$(GODOT)" --headless --path client --export-release "iOS" "$(CURDIR)/output/ios/Civilization.xcodeproj"; \
	$(CLIENT_RESTORE_CONFIG); \
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

# iOS 真机开发运行：导出并签名后安装到设备并启动。
# 默认取第一台已连接的 iPhone；IOS_DEVICE_TYPE 取 iphone / ipad 切换设备类型，
# IOS_DEVICE 用设备名或 UDID 指定具体设备，IOS_LAUNCH=0 只安装。
IOS_DEVICE ?=
IOS_DEVICE_TYPE ?= iphone
IOS_LAUNCH ?= 1

# Android 签名参数只从环境变量读取：keystore 与口令不入库，也不写进命令行。
ANDROID_KEYSTORE_PATH ?=
ANDROID_KEYSTORE_USER ?=
ANDROID_KEYSTORE_PASSWORD ?=
ANDROID_OUTPUT ?= output/android
export ANDROID_KEYSTORE_PATH ANDROID_KEYSTORE_USER ANDROID_KEYSTORE_PASSWORD

client-export-android-check:
	@$(if $(filter 0 1,$(AAB)),,$(error AAB 只能是 0 或 1，收到：$(AAB)))

# Android 侧载 APK（AAB=0）或商店 AAB（AAB=1）：arm64、横屏、包名 com.civilization.banner，
# release keystore 签名。缺少任一 ANDROID_KEYSTORE_* 或 keystore 文件时直接失败，不产出未签名包。
_client-export-android: client-export-android-check client-env-check client-export-templates client-import
	@if [ "$(AAB)" = "1" ]; then \
	  [ -f "$(GODOT_TEMPLATE_DIR)/android_source.zip" ] || { echo "缺少 Android 构建模板：$(GODOT_TEMPLATE_DIR)/android_source.zip" >&2; exit 1; }; \
	else \
	  [ -f "$(GODOT_TEMPLATE_DIR)/android_release.apk" ] || { echo "缺少 Android 导出模板：$(GODOT_TEMPLATE_DIR)/android_release.apk" >&2; exit 1; }; \
	fi
	@[ -n "$$ANDROID_KEYSTORE_PATH" ] || { echo "需要 ANDROID_KEYSTORE_PATH（release keystore 路径）" >&2; exit 1; }
	@[ -n "$$ANDROID_KEYSTORE_USER" ] || { echo "需要 ANDROID_KEYSTORE_USER（keystore alias）" >&2; exit 1; }
	@[ -n "$$ANDROID_KEYSTORE_PASSWORD" ] || { echo "需要 ANDROID_KEYSTORE_PASSWORD（keystore 口令）" >&2; exit 1; }
	@[ -f "$$ANDROID_KEYSTORE_PATH" ] || { echo "Android keystore 不存在：$$ANDROID_KEYSTORE_PATH" >&2; exit 1; }
	@set -e; $(CLIENT_BACKUP_CONFIG); \
	SERVER_URL="$(CLIENT_SERVER_URL)" STEAM_APP_ID="" $(PYTHON) scripts/gen_build_config.py $(CLIENT_REQUIRE_SERVER); \
	GODOT="$(GODOT)" GODOT_TEMPLATE_DIR="$(GODOT_TEMPLATE_DIR)" ANDROID_OUTPUT="$(ANDROID_OUTPUT)" \
	  bash scripts/android_export.sh $(if $(filter 1,$(AAB)),aab,apk)

# 快速别名：常用设备与平台的短入口，等价于带参数的 client-run / client-export。
# 命令行传入的 ENV、SERVER_URL、STEAM_APP_ID、WIDTH/HEIGHT、LOCAL_SERVER 等会原样传给子 make；
# 别名自身指定的 DEVICE / PLATFORM / ENV / AAB 优先。
# 基础命令 make run / make client-run 不带环境（ENV 可选，默认 local）；其余运行别名显式带环境：
# run-<设备>-<环境>（编辑器运行）与 run-ios-<设备>-<环境>（真机运行）。
run:
	@$(MAKE) --no-print-directory client-run

CLIENT_RUN_DEVICES := desktop iphone ipad phone tablet
CLIENT_RUN_ENVS := local dev prod

define client_run_device_alias
.PHONY: run-$(1)-$(2)
run-$(1)-$(2):
	@$$(MAKE) --no-print-directory client-run DEVICE=$(1) ENV=$(2)
endef

$(foreach device,$(CLIENT_RUN_DEVICES),$(foreach env,$(CLIENT_RUN_ENVS),$(eval $(call client_run_device_alias,$(device),$(env)))))

# 旧设备别名保留为 local 快捷入口，等价于对应的 run-<设备>-local。
run-iphone:
	@$(MAKE) --no-print-directory client-run DEVICE=iphone ENV=local

run-ipad:
	@$(MAKE) --no-print-directory client-run DEVICE=ipad ENV=local

run-desktop:
	@$(MAKE) --no-print-directory client-run DEVICE=desktop ENV=local

run-phone:
	@$(MAKE) --no-print-directory client-run DEVICE=phone ENV=local

run-tablet:
	@$(MAKE) --no-print-directory client-run DEVICE=tablet ENV=local

run-ios:
	@$(MAKE) --no-print-directory client-run PLATFORM=ios ENV=local

export:
	@$(MAKE) --no-print-directory client-export

export-macos:
	@$(MAKE) --no-print-directory client-export PLATFORM=macos

export-local:
	@$(MAKE) --no-print-directory client-export PLATFORM=local

export-steam:
	@$(MAKE) --no-print-directory client-export PLATFORM=steam

export-ios:
	@$(MAKE) --no-print-directory client-export PLATFORM=ios

export-android:
	@$(MAKE) --no-print-directory client-export PLATFORM=android

export-aab:
	@$(MAKE) --no-print-directory client-export PLATFORM=android AAB=1

# 带环境的导出别名：export-<平台>-<环境>，同时固定 PLATFORM 与 ENV。
# 环境取 local / dev / prod；prod 需要先配置 CLIENT_PROD_SERVER_URL，否则导出报错。
export-macos-local:
	@$(MAKE) --no-print-directory client-export PLATFORM=macos ENV=local

export-macos-dev:
	@$(MAKE) --no-print-directory client-export PLATFORM=macos ENV=dev

export-macos-prod:
	@$(MAKE) --no-print-directory client-export PLATFORM=macos ENV=prod

export-ios-local:
	@$(MAKE) --no-print-directory client-export PLATFORM=ios ENV=local

export-ios-dev:
	@$(MAKE) --no-print-directory client-export PLATFORM=ios ENV=dev

export-ios-prod:
	@$(MAKE) --no-print-directory client-export PLATFORM=ios ENV=prod

# iPad 别名：iOS 预设是 iPhone & iPad 通用包（targeted_device_family=2），产物与
# export-ios-<环境> 完全相同；这里只提供与 run-ios-ipad-<环境> 对齐的命名入口。
export-ipad-local:
	@$(MAKE) --no-print-directory client-export PLATFORM=ios ENV=local

export-ipad-dev:
	@$(MAKE) --no-print-directory client-export PLATFORM=ios ENV=dev

export-ipad-prod:
	@$(MAKE) --no-print-directory client-export PLATFORM=ios ENV=prod

export-android-local:
	@$(MAKE) --no-print-directory client-export PLATFORM=android ENV=local

export-android-dev:
	@$(MAKE) --no-print-directory client-export PLATFORM=android ENV=dev

export-android-prod:
	@$(MAKE) --no-print-directory client-export PLATFORM=android ENV=prod

# iOS 运行别名：run-ios-<设备>-<环境>，设备取 iphone / ipad，环境取 local / dev / prod。
# 同时固定 PLATFORM=ios、设备类型与 ENV；IOS_DEVICE 仍可指定具体设备并覆盖自动选择。
CLIENT_RUN_IOS_DEVICES := iphone ipad

define client_run_ios_alias
.PHONY: run-ios-$(1)-$(2)
run-ios-$(1)-$(2):
	@$$(MAKE) --no-print-directory client-run PLATFORM=ios IOS_DEVICE_TYPE=$(1) ENV=$(2)
endef

$(foreach device,$(CLIENT_RUN_IOS_DEVICES),$(foreach env,$(CLIENT_RUN_ENVS),$(eval $(call client_run_ios_alias,$(device),$(env)))))
