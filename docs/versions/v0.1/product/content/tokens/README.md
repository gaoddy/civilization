# Token 注册表

> 依赖：[核心规则](../../rules/core-rules.md) §6、[关键词与机制](../../rules/keywords-and-mechanics.md) §9
> 状态：方案未实施
> 数据：本文件即注册表，不为单个 Token 建实体文档

本文登记战斗中由效果生成、不进牌池的衍生随从；不描述具体卡牌数值与实现。

## 1. 规则

1. Token 有独立 `id`，与卡牌 id 命名空间分离；
2. Token 不进牌池、不可购买、不参与三连，死亡即消失；
3. Token 不写入 `cards/`，召唤来源写在生成它的卡牌效果文本中；
4. 召唤已有卡（如实验小鼠召唤 1/2 白蚁）不是 Token，按 [核心规则](../../rules/core-rules.md) §6 第 7 条从牌池召唤。

## 2. 注册表

| id | 名称 | 攻击力 | 生命值 | 关键词 | 生成来源 |
| --- | --- | --- | --- | --- | --- |
| `seedling` | 幼苗 | 1 | 1 | 无 | 播种（9 张植物随从） |
| `synthetic-cell-token` | 合成细胞（幼体） | 3 | 3 | 无 | 人造子宫 |
| `hela-cell-token` | 海拉细胞（复制体） | 2 | 2 | 无 | 海拉细胞 |

「关键词」列如有取值，写英文 id（`battlecry`、`taunt`、`breed N` 等，见[关键词与机制](../../rules/keywords-and-mechanics.md) §1）；「名称」列是文本来源，运行时由文本表 `token.<id>.name` 承载（`client/text/zh_CN.csv`）。

## 相关文档

- [内容索引](../README.md)
- [核心规则](../../rules/core-rules.md)
