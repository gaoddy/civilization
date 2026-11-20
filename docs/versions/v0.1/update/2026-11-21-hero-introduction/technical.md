# 英雄介绍技术方案

> 状态：方案未实施
> 需求归属：v0.1
> 产品规则：[product.md](product.md)（已确认）

## 1. 改动边界

- 包含：32 份英雄文档、文档校验、文本生成、客户端英雄展示、文本表与客户端测试。
- 不包含：协议 schema、服务端生成数据字段、房间与对局逻辑、卡牌（非英雄）详情。

## 2. 内容文档

- 32 份 `docs/versions/v0.1/product/content/heroes/<id>.md`：
  - front matter 增加 `introduction: <文本>`，位于 `skills` 之后；
  - 正文在「## 属性」与「## 效果」之间新增「## 简介」小节，内容与字段一致；
  - 文本 40–80 个汉字，基于「## 出处」写人物身份与核心贡献，不新增无法查证的细节。
- `heroes/README.md` 不增加介绍列，索引结构不变。

## 3. 校验与文本

- `scripts/validate_docs.py`：
  - `HERO_REQUIRED_FIELDS` 增加 `introduction`；
  - 校验 `introduction` 非空、长度 40–80 个汉字；
  - 校验「## 简介」存在、非空且与 front matter 一致；
  - 英雄属性表不增加介绍行。
- `scripts/gen_text.py`：英雄循环增加 `card.<id>.introduction`，沿用现有 `card.<id>.name` 前缀。
- 重新生成 `client/text/zh_CN.csv`。

## 4. 客户端

- `client/autoload/card_db.gd` 增加 `hero_introduction(id)`：读 `card.<id>.introduction`，缺键返回空串。
- `client/scenes/room.gd` 的 `_show_skill`：标题之后、技能列表之前插入介绍标签；仅非空时插入，使用 `UI.label` 自动换行与 `color-text-dim`。
- `client/scenes/gallery_detail.gd` 的英雄分支：名称之后插入介绍标签，再显示类型、玩法与技能。
- `client/scenes/game_detail_layer.gd` 的英雄详情：名称之后插入介绍文本块，保持与图鉴英雄详情同一口径。
- 不需要改 `heroes.json`，介绍文本只走文本表。

## 5. 测试

- 文档校验：32 张英雄均有介绍，长度与正文一致。
- 文本表：`card.<id>.introduction` 32 条，无缺失或重复。
- 客户端 UI 冒烟：英雄详情包含介绍文本；缺键时不出现空白标签。
- 验证命令：`make data-check`、`make client-check`、`make client-smoke`。

## 6. 文档回写

- `docs/versions/v0.1/product/content/card-system.md` §2.1 补充英雄卡 `introduction` 字段与展示位置。
- `docs/current/technical/client.md` 补充 `card.<id>.introduction` 文本 key 与两处展示入口。
