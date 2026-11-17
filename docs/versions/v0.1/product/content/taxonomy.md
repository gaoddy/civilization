# 分类树与玩法

> 依赖：[卡牌规范与数据](card-system.md)、[技能体系](skill-system.md)
> 状态：方案未实施
> 数据：本文件即分类树、玩法表与关联分类的唯一编辑源

本文件是科学分类树与玩法表的唯一编辑源。卡牌的 `taxon` 取本文件 §1 的叶子 id，
`playstyle` 取 §2 的玩法 id（或 `neutral`），`specialty` 取 §3 的关联分类。

## 1. 科学分类树

| id | 名称 | 上级 | 层级 | 备注 |
| --- | --- | --- | --- | --- |
| animalia | 动物界 | | 界 | |
| chordata | 脊索动物门 | animalia | 门 | |
| mammalia | 哺乳纲 | chordata | 纲 | 玩法 mammal |
| primates | 灵长目 | mammalia | 目 | |
| hominidae | 人科 | primates | 科 | 含人类与猩猩、大猩猩 |
| carnivora | 食肉目 | mammalia | 目 | |
| felidae | 猫科 | carnivora | 科 | |
| canidae | 犬科 | carnivora | 科 | |
| artiodactyla | 偶蹄目 | mammalia | 目 | |
| suidae | 猪科 | artiodactyla | 科 | |
| bovidae | 牛科 | artiodactyla | 科 | |
| perissodactyla | 奇蹄目 | mammalia | 目 | |
| equidae | 马科 | perissodactyla | 科 | |
| proboscidea | 长鼻目 | mammalia | 目 | |
| elephantidae | 象科 | proboscidea | 科 | |
| rodentia | 啮齿目 | mammalia | 目 | |
| muridae | 鼠科 | rodentia | 科 | |
| mammalia-incertae-sedis | 未定科 | mammalia | 科 | 虚构 / 未定科 |
| aves | 鸟纲 | chordata | 纲 | 玩法 bird |
| accipitriformes | 鹰形目 | aves | 目 | |
| accipitridae | 鹰科 | accipitriformes | 科 | |
| columbiformes | 鸽形目 | aves | 目 | |
| columbidae | 鸠鸽科 | columbiformes | 科 | |
| passeriformes | 雀形目 | aves | 目 | |
| fringillidae | 燕雀科 | passeriformes | 科 | |
| archaeopterygiformes | 始祖鸟目 | aves | 目 | |
| archaeopterygidae | 始祖鸟科 | archaeopterygiformes | 科 | |
| reptilia | 爬行纲 | chordata | 纲 | 玩法 reptile |
| squamata | 有鳞目 | reptilia | 目 | |
| varanidae | 巨蜥科 | squamata | 科 | |
| squamata-incertae-sedis | 未定科 | squamata | 科 | 未定科 |
| actinopterygii | 鱼纲 | chordata | 纲 | 玩法 fish |
| actinopterygii-incertae-sedis | 未定科 | actinopterygii | 科 | 预留；现有卡牌无鱼纲物种 |
| arthropoda | 节肢动物门 | animalia | 门 | |
| insecta | 昆虫纲 | arthropoda | 纲 | 玩法 insect |
| blattodea | 蜚蠊目 | insecta | 目 | |
| termitidae | 白蚁科 | blattodea | 科 | |
| diptera | 双翅目 | insecta | 目 | |
| drosophilidae | 果蝇科 | diptera | 科 | |
| mollusca | 软体动物门 | animalia | 门 | |
| cephalopoda | 头足纲 | mollusca | 纲 | 玩法 cephalopod |
| octopoda | 八腕目 | cephalopoda | 目 | |
| octopodidae | 章鱼科 | octopoda | 科 | |
| cnidaria | 刺胞动物门 | animalia | 门 | |
| hydrozoa | 水螅纲 | cnidaria | 纲 | 玩法 cnidarian |
| anthoathecata | 花水螅目 | hydrozoa | 目 | |
| oceaniidae | 海螅水母科 | anthoathecata | 科 | |
| plantae | 植物界 | | 界 | 玩法 plant |
| bryophyta | 苔藓植物门 | plantae | 门 | |
| bryopsida | 藓纲 | bryophyta | 纲 | |
| bryales | 真藓目 | bryopsida | 目 | |
| bryaceae | 真藓科 | bryales | 科 | |
| pteridophyta | 蕨类植物门 | plantae | 门 | |
| polypodiopsida | 水龙骨纲 | pteridophyta | 纲 | |
| polypodiales | 水龙骨目 | polypodiopsida | 目 | |
| polypodiaceae | 水龙骨科 | polypodiales | 科 | |
| salviniales | 槐叶苹目 | polypodiopsida | 目 | |
| salviniaceae | 槐叶苹科 | salviniales | 科 | |
| angiospermae | 被子植物门 | plantae | 门 | |
| liliopsida | 单子叶植物纲 | angiospermae | 纲 | |
| poales | 禾本目 | liliopsida | 目 | |
| poaceae | 禾本科 | poales | 科 | |
| magnoliopsida | 双子叶植物纲 | angiospermae | 纲 | |
| fabales | 豆目 | magnoliopsida | 目 | |
| fabaceae | 豆科 | fabales | 科 | |
| malvales | 锦葵目 | magnoliopsida | 目 | |
| malvaceae | 锦葵科 | malvales | 科 | |
| sapindales | 无患子目 | magnoliopsida | 目 | |
| rutaceae | 芸香科 | sapindales | 科 | |
| asterales | 菊目 | magnoliopsida | 目 | |
| asteraceae | 菊科 | asterales | 科 | |
| lamiales | 唇形目 | magnoliopsida | 目 | |
| oleaceae | 木犀科 | lamiales | 科 | |
| malpighiales | 金虎尾目 | magnoliopsida | 目 | |
| rhizophoraceae | 红树科 | malpighiales | 科 | |
| plantae-incertae-sedis | 未定科 | plantae | 科 | 虚构 / 未定科 |
| fungi | 真菌界 | | 界 | 玩法 fungus |
| fungi-incertae-sedis | 未定科 | fungi | 科 | 虚构 / 未定科 |
| bacteria | 细菌域 | | 界 | 玩法 bacteria |
| proteobacteria | 变形菌门 | bacteria | 门 | |
| gammaproteobacteria | γ-变形菌纲 | proteobacteria | 纲 | |
| enterobacterales | 肠杆菌目 | gammaproteobacteria | 目 | |
| vibrionaceae | 弧菌科 | enterobacterales | 科 | |
| yersiniaceae | 耶尔森菌科 | enterobacterales | 科 | |
| alphaproteobacteria | α-变形菌纲 | proteobacteria | 纲 | |
| rickettsiales | 立克次体目 | alphaproteobacteria | 目 | |
| rickettsiaceae | 立克次体科 | rickettsiales | 科 | |
| firmicutes | 厚壁菌门 | bacteria | 门 | |
| bacilli | 芽孢杆菌纲 | firmicutes | 纲 | |
| bacillales | 芽孢杆菌目 | bacilli | 目 | |
| bacillaceae | 芽孢杆菌科 | bacillales | 科 | |
| lactobacillales | 乳杆菌目 | bacilli | 目 | |
| streptococcaceae | 链球菌科 | lactobacillales | 科 | |
| actinobacteria | 放线菌门 | bacteria | 门 | |
| actinomycetia | 放线菌纲 | actinobacteria | 纲 | |
| mycobacteriales | 分枝杆菌目 | actinomycetia | 目 | |
| mycobacteriaceae | 分枝杆菌科 | mycobacteriales | 科 | |
| bacteria-incertae-sedis | 未定科 | bacteria | 科 | 未定科 |
| protozoa | 原生生物界 | | 界 | 玩法 protozoa |
| apicomplexa | 顶复门 | protozoa | 门 | |
| aconoidasida | 无腔纲 | apicomplexa | 纲 | |
| haemosporida | 血孢子虫目 | aconoidasida | 目 | |
| plasmodiidae | 疟原虫科 | haemosporida | 科 | |
| virus | 病毒 | | 界 | 玩法 virus |
| riboviria | 核糖病毒门 | virus | 门 | |
| pisoniviricetes | 小核糖病毒纲 | riboviria | 纲 | |
| nidovirales | 网巢病毒目 | pisoniviricetes | 目 | |
| coronaviridae | 冠状病毒科 | nidovirales | 科 | |
| monjiviricetes | 单股负链病毒纲 | riboviria | 纲 | |
| mononegavirales | 单股负链病毒目 | monjiviricetes | 目 | |
| filoviridae | 丝状病毒科 | mononegavirales | 科 | |
| paramyxoviridae | 副黏病毒科 | mononegavirales | 科 | |
| rhabdoviridae | 弹状病毒科 | mononegavirales | 科 | |
| insthoviricetes | 流感病毒纲 | riboviria | 纲 | |
| articulavirales | 节段病毒目 | insthoviricetes | 目 | |
| orthomyxoviridae | 正黏病毒科 | articulavirales | 科 | |
| revtraviricetes | 逆转录病毒纲 | riboviria | 纲 | |
| ortervirales | 逆转录病毒目 | revtraviricetes | 目 | |
| retroviridae | 逆转录病毒科 | ortervirales | 科 | |
| flasuviricetes | 黄病毒纲 | riboviria | 纲 | |
| amarillovirales | 黄病毒目 | flasuviricetes | 目 | |
| flaviviridae | 黄病毒科 | amarillovirales | 科 | |
| nucleocytoviricota | 核质巨 DNA 病毒门 | virus | 门 | |
| pokkesviricetes | 痘病毒纲 | nucleocytoviricota | 纲 | |
| chitovirales | 痘病毒目 | pokkesviricetes | 目 | |
| poxviridae | 痘病毒科 | chitovirales | 科 | |
| virus-incertae-sedis | 未定科 | virus | 科 | 虚构 / 未定科 |
| craft | 造物 | | 界 | |
| power-energy | 动力与能源 | craft | 技术分类 | |
| transport | 交通与运载 | craft | 技术分类 | |
| weapon-siege | 武器与攻城 | craft | 技术分类 | |
| production-construction | 生产与建筑 | craft | 技术分类 | |
| information-computing | 信息与计算 | craft | 技术分类 | |
| life-medical | 生活与医疗 | craft | 技术分类 | |
| automation | 自动机械 | craft | 技术分类 | 玩法 machine |
| phenomenon | 现象 | | 现象 | 效果牌叶子 |

## 2. 玩法

| id | 名称 | 科学分类对应 | 核心循环 |
| --- | --- | --- | --- |
| mammal | 哺乳纲 | 哺乳纲 | 少量高质量随从站场成长，相邻同族互相加成 |
| bird | 鸟纲 | 鸟纲 | 高攻低血，攻击无视嘲讽，反复突进 |
| reptile | 爬行纲 | 爬行纲 | 低攻高血，靠减伤和毒换命 |
| fish | 鱼纲 | 鱼纲 | 同类越多越强，死亡时分裂鱼群 |
| insect | 昆虫纲 | 昆虫纲 | 快速铺场，亡语继续召唤 |
| cephalopod | 头足纲 | 头足纲 | 复制敌方随从的属性与关键词，潜行突袭 |
| cnidarian | 水螅纲 | 水螅纲 | 死亡回手或复活，每次复活更强 |
| plant | 植物界 | 植物界 | 播种 token，延迟收益、持续成长 |
| fungus | 真菌界 | 真菌界 | 亡语散播孢子，击杀后分解生命值 |
| bacteria | 细菌域 | 细菌域 | 给敌方叠感染层，满层转化或消灭 |
| protozoa | 原生生物界 | 原生生物界 | 吞噬低攻随从获得属性，分裂成更小个体 |
| virus | 病毒 | 病毒 | 传染扩散，潜伏数回合后爆发 |
| machine | 机械 | 自动机械 | 装配层数与机器人联动，零件回收 |

## 3. 关联分类

关联分类写在卡牌 `specialty` 列表中，每条为 `前缀:取值`，可零到多条。

- `play:<玩法 id>`：玩法关联。学者卡表示**擅长玩法**，中立非学者卡表示**关联玩法**；
  取值域为 §2 的 13 个玩法 id，不含 `neutral`。其他卡不得使用。
- `prey:<分类节点 id>`：捕食分类。表示这张卡捕食的对象分类，取 §1 的任一节点；
  按祖先关系匹配，写上级节点即覆盖其全部下级。只允许生物卡使用（`taxon` 不在
  `craft` / `phenomenon` 下）。
- `host:<分类节点 id>`：寄生分类。表示这张卡寄生的宿主分类，取值域与匹配规则同 `prey:`；
  只允许生物卡使用。

效果牌的三类关联分类均为空。非生物卡不得使用 `prey:` / `host:`。

## 相关文档

- [卡牌文档索引](cards/README.md)
- [技能体系](skill-system.md)
