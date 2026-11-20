# 效果牌科学分类：移除现象并归入学科分类

> 状态：已实施
> 需求归属：v0.1
> 产品结论：[product.md](product.md)

## 1. 数据与生成

- 唯一编辑源为 `docs/versions/v0.1/product/content/taxonomy.md`：删除 `phenomenon` 行，新增三个界级根与 15 个学科叶子，层级列使用「学科」。
- `scripts/gen_card_data.py` 的层级映射新增 `学科 → discipline`，删除 `现象 → phenomenon`；`taxonomy.json` 由分类树生成。
- `scripts/gen_text.py` 按分类树自动生成 `enum.taxon.<id>`，新节点名称不手工维护。
- 100 张效果牌的 front matter `taxon` 改为学科叶子或既有造物技术分类；属性表由 `make docs-render` 派生，不手工改写。
- Token 注册表「基础细胞」的 `taxon` 改为 `biology`；`science` 的 Token 不再允许留空，`parse_tokens` 对空分类报错。

## 2. 校验

- `scripts/validate_docs.py`：
  - 层级映射删除「现象」，新增「学科」；`discipline` 只允许挂在界级根下。
  - 叶子层级允许「科」「技术分类」「学科」；删除现象节点唯一性与根层级例外。
  - 效果牌不再强制 `taxon = phenomenon`，只要求为分类树叶子。
  - `is_biological` 改为判断根节点属于六个生物界，不再以「非 craft / phenomenon」推断。
- 分类树节点数校验更新为 338。

## 3. 规则引擎

- `server/crates/rule-engine/src/taxonomy.rs`：`Rank` 新增 `Discipline`，删除 `Phenomenon`。
- `server/crates/rule-engine/src/data.rs`：删除 `PHENOMENON` 常量；`Token.taxon` 改为必填字符串，`Token::taxon()` 删除。
- `CardFacts` 直接取 `token.taxon`。
- 测试夹具补齐 Token 的 `taxon`，删除「缺字段按 phenomenon 回退」用例。

## 4. 客户端

- `client/scenes/card_gallery.gd`：删除 `TAXON_PHENOMENON` 跳过逻辑；顶层分类按分类树根节点展示，最后追加「中立」「效果牌」。
- `client/tests/ui_smoke_gallery.gd`：保留「不含现象」断言，新增「科学」「人文」「社会」存在断言；既有计数不变。
- 生成物 `client/generated/cards.json`、`taxonomy.json`、`tokens.json` 与文本表由 `make data` 重新生成。

## 5. 文档同步

- `docs/versions/v0.1/product/content/card-system.md`：效果牌科学分类、属性表与校验口径。
- `docs/versions/v0.1/product/content/tokens/README.md`：Token 科学分类必填。
- `docs/current/technical/rule-engine-and-simulator.md`：Rank、Token 分类与筛选回退。
- `docs/current/technical/client.md` 与 `docs/current/product/ui/`：图鉴顶层分类与详情路径。
- `docs/current/product/rules/keywords-and-mechanics.md`：生物卡判定口径。

## 6. 验证

- `make data`、`make docs-render` 无漂移。
- `make data-check`、`make test`、`make lint`、`make client-check`、`make client-smoke`。
- 服务端加载新生成数据并完成既有测试；客户端图鉴冒烟通过。
