# 英雄与卡牌 id 语义化、居里夫人改本名 — 短设计

日期：2026-11-14
范围：`docs/`（纯文档与 CSV 数据）

## 意图

1. 英雄 `h01`–`h10`、卡牌 `s01`–`u18` 都是纯序号 id，不含语义；程序与数据引用时要反查表。
2. 「居里夫人」是俗称，人物全名是「玛丽·居里」；其他英雄均用全名，仅此一位不统一。

## 目标 / 非目标

- 目标：10 位英雄、120 张卡牌的 id 改为语义 slug；「居里夫人」统一改为「玛丽·居里」。
- 非目标：不改数值、关键词、技能效果；不改写 `docs/superpowers/plans/` 下的历史记录；不新增或删除卡牌。

## 命名规则

- 中文人物与中文概念：拼音，如 `baiyi`、`qishi`、`zhugeliang`。
- 外国人名与国际通用术语：英文，如 `marie_curie`、`einstein`、`darwin`、`smallpox`、`quantum_computer`。
- 格式：`^[a-z][a-z0-9_]*$`，全局唯一，不再带时代前缀；时代由 `era` 字段表达（与英雄 id 风格一致）。
- 英雄与同名随从保持同一 slug（`einstein`、`darwin`、`mendel`、`marie_curie`），二者是不同表、不同命名空间，沿用文档既有「同名保留、UI 区分」的结论。

## 英雄 id 映射

| 原名 | 新 id | | 原名 | 新 id |
| --- | --- | --- | --- | --- |
| 鲁迅 | `luxun` | | 迈克尔·法拉第 | `faraday` |
| 高德纳 | `knuth` | | 列奥纳多·达·芬奇 | `davinci` |
| 阿尔伯特·爱因斯坦 | `einstein` | | 格雷戈尔·约翰·孟德尔 | `mendel` |
| 玛丽·居里 | `marie_curie` | | 查尔斯·罗伯特·达尔文 | `darwin` |
| 尼古拉·特斯拉 | `tesla` | | 伽利略·伽利莱 | `galileo` |

## 卡牌 id 映射

| 旧 id | 卡名 | 新 id | 旧 id | 卡名 | 新 id |
| --- | --- | --- | --- | --- | --- |
| s01 | 白蚁 | `baiyi` | o13 | 石轮车 | `shilunche` |
| s02 | 始祖鸟 | `shizuniao` | o14 | 天花 | `smallpox` |
| s03 | 实验小鼠 | `shiyanxiaoshu` | o15 | 亚里士多德 | `aristotle` |
| s04 | 果蝇 | `guoying` | o16 | 阿基米德 | `archimedes` |
| s05 | 野狼 | `yelang` | o17 | 张衡 | `zhangheng` |
| s06 | 巨鹰 | `juying` | o18 | 希波克拉底 | `hippocrates` |
| s07 | 猛犸象 | `mengmaxiang` | o19 | 猛虎 | `menghu` |
| s08 | 剑齿虎 | `jianchihu` | o20 | 始祖兽 | `shizushou` |
| s09 | 毒蛇 | `dushe` | o21 | 巫医 | `wuyi` |
| s10 | 拟态章鱼 | `nitaizhangyu` | o22 | 疟疾 | `malaria` |
| s11 | 豌豆 | `wandou` | o23 | 原始机械傀儡 | `yuanshijixiekuilei` |
| s12 | 橘子 | `juzi` | o24 | 炼金术士 | `lianjinshushi` |
| s13 | 原始真菌 | `yuanshizhenjun` | f01 | 骑士 | `qishi` |
| s14 | 蕨类 | `juelei` | f02 | 弩手 | `nushou` |
| s15 | 屠夫 | `tufu` | f03 | 水车 | `shuiche` |
| s16 | 弓箭手 | `gongjianshou` | f04 | 蔡伦 | `cailun` |
| s17 | 斯巴达勇士 | `sibadayongshi` | f05 | 火枪手 | `huoqiangshou` |
| s18 | 燧石矛兵 | `suishimaobing` | f06 | 忍者 | `renzhe` |
| s19 | 洞穴画家 | `dongxuehuajia` | f07 | 侠客 | `xiake` |
| s20 | 驯火者 | `xunhuozhe` | f08 | 诸葛亮 | `zhugeliang` |
| s21 | 石斧匠 | `shifujiang` | f09 | 罗马军团兵 | `luomajuntuanbing` |
| s22 | 猛犸猎人 | `mengmalieren` | f10 | 投石机 | `toushiji` |
| s23 | 噬菌体 | `shijunti` | f11 | 风车 | `fengche` |
| s24 | 病毒始祖 | `bingdushizu` | f12 | 毕昇 | `bisheng` |
| o01 | 欧几里得 | `euclid` | f13 | 孔子 | `kongzi` |
| o02 | 毕达哥拉斯 | `pythagoras` | f14 | 李耳 | `lier` |
| o03 | 岩画师 | `yanhuashi` | f15 | 十字军 | `shizijun` |
| o04 | 野猪 | `yezhu` | f16 | 攻城塔 | `gongchengta` |
| o05 | 藤蔓 | `tengman` | f17 | 柏拉图 | `plato` |
| o06 | 部落战士 | `buluozhanshi` | f18 | 米开朗基罗 | `michelangelo` |
| o07 | 狼群首领 | `langqunshouling` | f19 | 黑死病 | `black_death` |
| o08 | 巨蜥 | `juxi` | f20 | 霍乱 | `cholera` |
| o09 | 巨花 | `juhua` | f21 | 麦田 | `maitian` |
| o10 | 长矛手 | `changmaoshou` | f22 | 战马 | `zhanma` |
| o11 | 萨满 | `saman` | f23 | 古腾堡 | `gutenberg` |
| o12 | 兽皮匠 | `shoupijiang` | f24 | 摩西 | `moses` |
| m01 | 爱因斯坦冰箱 | `einstein_fridge` | u01 | 自动驾驶汽车 | `self_driving_car` |
| m02 | 机器狗 | `robot_dog` | u02 | 克隆羊多莉 | `dolly` |
| m03 | 电饭煲 | `rice_cooker` | u03 | 量子计算机 | `quantum_computer` |
| m04 | 黑猩猩坎兹 | `kanzi` | u04 | 纳米机器人 | `nanobot` |
| m05 | 流感病毒 | `influenza` | u05 | 合成细胞 | `synthetic_cell` |
| m06 | 拳击手 | `boxer` | u06 | 人造子宫 | `artificial_womb` |
| m07 | X射线机 | `xray_machine` | u07 | 太空水稻 | `space_rice` |
| m08 | 蒸汽机 | `steam_engine` | u08 | 固氮作物 | `nitrogen_crop` |
| m09 | 电话机 | `telephone` | u09 | 冯·诺依曼 | `von_neumann` |
| m10 | 留声机 | `phonograph` | u10 | 克劳德·香农 | `shannon` |
| m11 | 海拉细胞 | `hela_cell` | u11 | 太空电梯 | `space_elevator` |
| m12 | 带粉色头套的猩猩 | `pink_ape` | u12 | 聚变反应堆 | `fusion_reactor` |
| m13 | 杂交水稻 | `hybrid_rice` | u13 | 复活猛犸 | `revived_mammoth` |
| m14 | 艾滋病毒 | `hiv` | u14 | 垂直农场 | `vertical_farm` |
| m15 | 孟德尔 | `mendel` | u15 | 基因工程病毒 | `engineered_virus` |
| m16 | 李小龙 | `bruce_lee` | u16 | 库尔特·哥德尔 | `godel` |
| m17 | 飞机 | `airplane` | u17 | 通用人工智能 | `agi` |
| m18 | 巡逻机甲 | `patrol_mech` | u18 | 卡尔·萨根 | `sagan` |
| m19 | 达尔文 | `darwin` | | | |
| m20 | 玛丽·居里 | `marie_curie` | | | |
| m21 | 爱因斯坦 | `einstein` | | | |
| m22 | 牛顿 | `newton` | | | |
| m23 | 青蒿 | `qinghao` | | | |
| m24 | 埃博拉 | `ebola` | | | |
| m25 | 穆罕默德·阿里 | `muhammad_ali` | | | |
| m26 | 大猩猩科科 | `koko` | | | |
| m27 | 图灵机 | `turing_machine` | | | |
| m28 | 艾伦·图灵 | `alan_turing` | | | |
| m29 | 袁隆平 | `yuanlongping` | | | |
| m30 | 无线电报 | `wireless_telegraph` | | | |

## 改动落点

| 文件 | 改动 |
| --- | --- |
| `docs/data/英雄数据.csv` | 10 行 id；h04 行 name 改「玛丽·居里」 |
| `docs/data/卡牌数据.csv` | 120 行 id；m20 行 name 改「玛丽·居里」 |
| `docs/06-英雄与技能.md` | §1 总表、§3 详解标题、素材合并说明、§6 校验期望、§7 同名备注 |
| `docs/05-卡牌规范与数据.md` | §1 `id` 字段取值域说明；§4.2 合并表 id 引用；§6 全量卡表 120 行 id |
| `docs/04-时代与场景.md` | §3.4 出场流派括注人名 |

## 校验

- `docs/05` 校验脚本增加：id 唯一、`^[a-z][a-z0-9_]*$`；`docs/06` 同样增加英雄 id 唯一与格式断言。
- 运行 05 §5 与 06 §6 的全部 python 校验脚本，期望值与文档现有期望一致（120 张、时代 24/24/24/30/18 等；英雄 10 位）。
- 全库检索确认 `h01`–`h10`、`s01`–`u18`、`居里夫人` 除 `docs/superpowers/plans/` 历史记录外无残留。
