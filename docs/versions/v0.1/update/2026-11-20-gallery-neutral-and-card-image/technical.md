# 图鉴：中立分类、随从/效果筛选与卡图预览：技术方案

> 状态：已实施
> 需求归属：v0.1
> 产品依据：[product.md](product.md)

## 1. 改动边界

- 只改客户端图鉴、客户端展示数据与文档；服务端、协议、规则、卡牌数值与 `taxonomy` 源数据不变。
- 不修改 `taxonomy.md` 与 `server/data/v0.1/` 生成物；「现象」只在图鉴目录层被跳过。
- 不新增图片资源；详情预览复用已有 `client/assets/cards/faces/` 卡面美术。

## 2. 分类与筛选

### 2.1 分类目录

- `CardGallery._categories()` 遍历 `CardDB.taxonomy` 根节点，跳过 `phenomenon`，并在该位置注入 `{id: "neutral", name: "中立"}` 与 `{id: "effect", name: "效果牌"}`。
- `_category_keys(entry)` 返回卡牌命中的分类：
  - 界级根节点（`phenomenon` 除外）；
  - `playstyle == neutral` → `neutral`；
  - `card_type == effect` → `effect`。
- 一张卡可同时命中界级根与横切分类。

### 2.2 二级筛选

- `GalleryToc.set_state(categories, counts, selected, kinds, kind_counts, selected_kind)`：顶层项选中后，在其下方插入 `KindAll` / `KindMinion` / `KindEffect` 三个缩进按钮，使用独立 `ButtonGroup`。
- 新增 `kind_selected(String)` 信号；`CardGallery._on_taxon_selected` 切换顶层分类时把 `_kind` 重置为空串。
- `_kind_bucket(entry)`：`card_type == effect` → `effect`，其余（含 Token）→ `minion`。

### 2.3 计数

- `_count_by_era`：按分类 + 二级筛选换算。
- `_count_by_category`：按时代 + 二级筛选换算。
- `_count_by_kind`：按时代 + 分类换算。
- 零计数且未选中的项禁用，与既有规则一致。

## 3. Token 展示数据

- `scripts/gen_client_data.py` 的 `_display_token` 为 Token 补 `card_type = "minion"`。
- 只影响 `client/generated/tokens.json`；服务端 `Token` 结构带 `deny_unknown_fields`，`server/data/v0.1/tokens.json` 不增加该字段。
- 图鉴详情仍通过 `CardDB.tokens` 判定显示「Token」，不改为「随从」。

## 4. 详情卡图

- `GalleryDetail._build_preview` 取代 `GalleryCardFace.build`：
  - 外层 `CenterContainer`（`GalleryDetailPreview`）与圆角 `PanelContainer`（`GalleryDetailArtFrame`）。
  - 有 `CardDB.art_path(id, "card_face")` 且资源存在时，放 `TextureRect`（`GalleryDetailArt`），`STRETCH_KEEP_ASPECT_COVERED`。
  - 无图时放 `Label`（`GalleryDetailArtPlaceholder`），底色取 `UITheme.playstyle_cover`，文字取 `CardText.initial`。
- 新增令牌：`gallery-preview-width` 200、`gallery-preview-height` 260，手机 160 / 208。
- 名称与属性行、节点名 `GalleryDetailName` / `GalleryDetailEffect` 保持不变。

## 5. 文案

`client/text/zh_CN.csv` 新增人工 key：

- `ui.gallery.category_effect`：效果牌
- `ui.gallery.kind_all`：全部
- `ui.gallery.kind_minion`：随从
- `ui.gallery.kind_effect`：效果

## 6. 测试

`client/tests/ui_smoke_gallery.gd` 新增：

- 目录存在 `Taxonneutral`、`Taxoneffect`，不存在 `Taxonphenomenon`。
- 中立 178 张；二级随从 78、效果 100；效果牌顶层 100 张；切换顶层后二级回到全部。
- `zhenghe` 显示 `GalleryDetailArt`；`adam-smith` 显示 `GalleryDetailArtPlaceholder`。

## 7. 验证记录

- `make client-check`：退出码 0。
- `make client-smoke`：退出码 0，十三个场景均输出 `UI_SMOKE_OK`。
