# 内容、协议、资产与文档生成/校验。由根 Makefile include。
# 依赖根 Makefile 的 PYTHON、JOBS。

ASSET_COMPRESS_DIR := tools/asset-compress
ASSET_COMPRESS_QUALITY ?= standard
ASSET_COMPRESS_BACKEND ?= auto
ASSET_MAP ?= client/assets/characters/portraits/asset_map.json
ASSET_COVERS ?= output/effect-covers/generated
FONT_SOURCE ?= output/fonts/NotoSansSC[wght].ttf

.PHONY: audit data data-check docs-render assets assets-covers sfx font-subset balance-check \
	protocol protocol-check protocol-rs protocol-rs-check asset-compress asset-compress-check

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
