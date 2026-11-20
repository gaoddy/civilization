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

| id | 名称 | 攻击力 | 生命值 | 关键词 | 玩法 | 科学分类 | 生成来源 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `seedling` | 幼苗 | 1 | 1 | 无 | plant | plantae-incertae-sedis | 播种（9 张植物随从） |
| `synthetic-cell-token` | 合成细胞（幼体） | 3 | 3 | 无 | bacteria | bacteria-incertae-sedis | 人造子宫 |
| `hela-cell-token` | 海拉细胞（复制体） | 2 | 2 | 无 | mammal | hominidae | 海拉细胞 |
| `fish-fry-token` | 鱼苗 | 1 | 1 | 无 | fish | actinopterygii-incertae-sedis | 鲑鱼、邓氏鱼的分裂 |
| `spore-token-1` | 孢子 | 1 | 1 | 无 | fungus | fungi-incertae-sedis | 酵母的分裂 |
| `spore-token-2` | 孢子 | 2 | 2 | 无 | fungus | fungi-incertae-sedis | 蘑菇、蜜环菌的分裂 |
| `spore-token-3` | 孢子 | 3 | 3 | 无 | fungus | fungi-incertae-sedis | 原杉菌的分裂 |
| `amoeba-larva-token-1` | 变形虫幼体 | 1 | 1 | 无 | protozoa | amoebidae | 变形虫的分裂 |
| `amoeba-larva-token-2` | 变形虫幼体 | 2 | 2 | 无 | protozoa | amoebidae | 大变形虫的分裂 |
| `schizont-token` | 裂殖体 | 3 | 3 | 无 | protozoa | plasmodiidae | 疟原虫的分裂 |
| `coral-polyp-token` | 珊瑚虫 | 1 | 1 | 无 | cnidarian | poritidae | 珊瑚虫、鹿角珊瑚的分裂 |
| `worker-bee-token-1` | 工蜂 | 1 | 1 | 无 | insect | apidae | 蜜蜂的分裂 |
| `worker-bee-token-2` | 工蜂 | 2 | 2 | 无 | insect | apidae | 蜜蜂皇后的战吼 |
| `termite-token` | 白蚁（2/2） | 2 | 2 | 无 | insect | termitidae | 白蚁王的分裂（2/2 衍生物，区别于 1/2 白蚁卡） |
| `seabird-token` | 海鸟 | 3 | 3 | 无 | bird | diomedeidae | 信天翁的分裂 |
| `octopus-larva-token` | 章鱼幼体 | 1 | 1 | 无 | cephalopod | octopodidae | 章鱼幼体的分裂 |
| `bacteria-token-2` | 细菌 | 2 | 2 | 无 | bacteria | enterobacteriaceae | 噬菌体的转化 |
| `bacteria-token-4` | 细菌 | 4 | 4 | 无 | bacteria | enterobacteriaceae | 超级细菌的转化 |
| `cordyceps-token` | 虫草 | 3 | 3 | 无 | fungus | ophiocordycipitaceae | 冬虫夏草的分裂 |
| `ammonite-token` | 菊石幼体 | 5 | 5 | 无 | cephalopod | ammonitidae | 菊石的分裂 |
| `volvox-cell-token` | 团藻细胞 | 1 | 1 | 无 | protozoa | volvocaceae | 团藻的分裂 |
| `neutral-token-2-2` | 基础细胞 | 2 | 2 | 无 | neutral | biology | 通用效果牌「集结」「召唤」 |
| `amoeba-larva-3-3-token` | 变形虫幼体 | 3 | 3 | 无 | protozoa | amoebidae | 原生定向牌「细胞分裂」 |

「关键词」列如有取值，写英文 id（`battlecry`、`taunt`、`breed N` 等，见[关键词与机制](../../rules/keywords-and-mechanics.md) §1）；「玩法」与「科学分类」列填写玩法 id 与分类树叶子 id（见[分类树与玩法](../taxonomy.md)）；「玩法」留空时按 `neutral` 处理，「科学分类」必填；「名称」列是文本来源，运行时由文本表 `token.<id>.name` 承载（`client/text/zh_CN.csv`）。

## 相关文档

- [内容索引](../README.md)
- [核心规则](../../rules/core-rules.md)
