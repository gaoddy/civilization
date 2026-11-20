# 图鉴分类目录改为玩法分类

> 状态：已实施
> 需求归属：v0.1
> 产品依据：[product.md](product.md)

## 1. 数据

- `client/autoload/card_db.gd` 读取 `taxonomy.json` 时保存 `playstyles` 数组，顺序与 `taxonomy.md` §2 一致。
- 图鉴只读取客户端生成数据，服务端数据与协议不变。

## 2. 分类逻辑

- `card_gallery.gd` 的顶层分类改为 `CardDB.playstyles` + 「中立」，删除科学分类根、学科根与「效果牌」顶层项。
- `_category_keys`：加入非 `neutral` 的 `playstyle`；遍历 `specialty` 的 `play:` 项加入关联玩法；`playstyle = neutral` 时加入 `neutral`。
- 删除 `_root_taxon`、`CATEGORY_EFFECT`、`DISCIPLINE_ROOTS` 与 `ui.gallery.category_effect` 文本键。
- 二级筛选 `KIND_MINION` / `KIND_EFFECT` 保持不变。

## 3. 测试

- `client/tests/ui_smoke_gallery.gd`：
  - 目录断言改为哺乳纲、机械存在，效果牌与科学分类根不存在。
  - 时代筛选改为未来下鱼纲禁用、未来 + 哺乳纲 11 张。
  - 保留中立 178、随从 78、效果 100；新增哺乳纲 100 与切换顶层后二级回到全部。

## 4. 文档

- `docs/current/product/ui/pages/gallery.md`、`components/card-gallery.md`、`components/gallery-toc.md`、`interactions.md`。
- `docs/current/technical/client.md`。
- 原 `2026-11-20-effect-science-taxonomy` 的图鉴目录部分改为指向本需求。

## 5. 验证

- `make client-check`、`make client-smoke`。
- `make data-check` 无漂移。
