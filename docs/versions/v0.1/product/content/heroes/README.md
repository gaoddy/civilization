# 英雄卡索引

本文索引 v0.1 的 32 张英雄卡，一行一张；字段规范与校验规则见[卡牌规范与数据](../card-system.md) §2.1。英雄卡位于 `content/heroes/`，`card_type: hero`，只用于开局选英雄，不进商店、牌池、三连与发现。每张英雄卡与 `content/cards/` 中同 id、同名的随从卡共享 `name`、`playstyle`、`specialty` 与 `skills`，两卡分属英雄池与随从池。「玩法」为英雄的玩法子叶；「关联」为 `play:` 关联玩法；「技能」为英雄引用的技能名。

| 英雄 | 玩法 | 关联 | 技能 |
| --- | --- | --- | --- |
| [亚当·斯密](adam-smith.md) | 哺乳纲 | 无 | 分工 |
| [贝多芬](beethoven.md) | 哺乳纲 | 无 | 技艺 |
| [达尔文](darwin.md) | 中立 | 哺乳纲、鸟纲、爬行纲、鱼纲、昆虫纲、头足纲、水螅纲 | aura-biology-plus-1-plus-1；进化 |
| [达·芬奇](davinci.md) | 中立 | 头足纲、哺乳纲 | 万能手稿 |
| [爱迪生](edison.md) | 机械 | 无 | 发明工厂 |
| [爱因斯坦](einstein.md) | 中立 | 鸟纲、哺乳纲 | battlecry-all-friendly-growth-once；光速出击 |
| [法拉第](faraday.md) | 中立 | 爬行纲、机械 | keyword-divine-shield；法拉第笼 |
| [弗莱明](fleming.md) | 病毒 | 无 | 培养 |
| [伽利略](galileo.md) | 中立 | 哺乳纲 | 观测 |
| [高斯](gauss.md) | 中立 | 哺乳纲 | 推演 |
| [高德纳](knuth.md) | 中立 | 机械 | 高效算法 |
| [拉马克](lamarck.md) | 中立 | 哺乳纲、鸟纲、爬行纲、鱼纲、昆虫纲、头足纲、水螅纲 | 获得性状 |
| [拉瓦锡](lavoisier.md) | 中立 | 爬行纲 | aura-other-plus-0-plus-1 |
| [林奈](linnaeus.md) | 中立 | 哺乳纲、鸟纲、爬行纲、鱼纲、昆虫纲、头足纲、水螅纲 | battlecry-all-biology-growth-once |
| [李时珍](lisshi.md) | 植物界 | 无 | battlecry-plant-growth-twice |
| [鲁班](luban.md) | 机械 | 无 | battlecry-friendly-mech-growth-once |
| [鲁迅](luxun.md) | 中立 | 哺乳纲 | 呐喊 |
| [玛丽·居里](marie-curie.md) | 中立 | 哺乳纲、机械 | battlecry-friendly-growth-twice；放射 |
| [孟德尔](mendel.md) | 中立 | 植物界 | battlecry-friendly-plant-growth-once；遗传 |
| [摩尔根](morgan.md) | 中立 | 昆虫纲、哺乳纲、鸟纲、爬行纲、鱼纲、头足纲、水螅纲 | aura-biology-plus-0-plus-1 |
| [墨子](mozi.md) | 哺乳纲 | 无 | 非攻 |
| [巴斯德](pasteur.md) | 病毒 | 无 | 微生物学 |
| [屈原](quyuan.md) | 中立 | 哺乳纲 | 呐喊 |
| [卢瑟福](rutherford.md) | 中立 | 哺乳纲、机械 | combat-random-enemy-damage-3 |
| [莎士比亚](shakespeare.md) | 哺乳纲 | 无 | battlecry-all-friendly-growth-once |
| [沈括](shenkuo.md) | 中立 | 哺乳纲 | 观测 |
| [司马迁](simaqian.md) | 哺乳纲 | 无 | aura-all-plus-0-plus-1 |
| [史蒂芬孙](stephenson.md) | 机械 | 无 | aura-other-mech-plus-1-plus-1 |
| [特斯拉](tesla.md) | 中立 | 鸟纲、机械 | 交流电 |
| [莱特兄弟](wright-brothers.md) | 机械 | 无 | 航线 |
| [张仲景](zhangzhongjing.md) | 哺乳纲 | 无 | battlecry-all-friendly-plus-0-plus-1 |
| [郑和](zhenghe.md) | 哺乳纲 | 无 | 启航 |
