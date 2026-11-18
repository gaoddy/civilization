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
| struthioniformes | 鸵鸟目 | aves | 目 | |
| struthionidae | 鸵鸟科 | struthioniformes | 科 | 鸵鸟 |
| sphenisciformes | 企鹅目 | aves | 目 | |
| spheniscidae | 企鹅科 | sphenisciformes | 科 | 企鹅 |
| psittaciformes | 鹦形目 | aves | 目 | |
| psittacidae | 鹦鹉科 | psittaciformes | 科 | 鹦鹉 |
| galliformes | 鸡形目 | aves | 目 | |
| phasianidae | 雉科 | galliformes | 科 | 孔雀 |
| procellariiformes | 鹱形目 | aves | 目 | |
| diomedeidae | 信天翁科 | procellariiformes | 科 | 信天翁、海鸟 Token |
| cathartiformes | 美洲鹫目 | aves | 目 | |
| teratornithidae | 畸鸟科 | cathartiformes | 科 | 阿根廷巨鹰 |
| corvidae | 鸦科 | passeriformes | 科 | 乌鸦 |
| reptilia | 爬行纲 | chordata | 纲 | 玩法 reptile |
| squamata | 有鳞目 | reptilia | 目 | |
| varanidae | 巨蜥科 | squamata | 科 | |
| squamata-incertae-sedis | 未定科 | squamata | 科 | 未定科 |
| gekkonidae | 壁虎科 | squamata | 科 | 壁虎 |
| lacertidae | 蜥蜴科 | squamata | 科 | 蜥蜴 |
| elapidae | 眼镜蛇科 | squamata | 科 | 眼镜蛇 |
| pythonidae | 蟒科 | squamata | 科 | 蟒蛇 |
| testudines | 龟鳖目 | reptilia | 目 | |
| testudinidae | 陆龟科 | testudines | 科 | 陆龟 |
| cheloniidae | 海龟科 | testudines | 科 | 海龟 |
| crocodilia | 鳄目 | reptilia | 目 | |
| crocodylidae | 鳄科 | crocodilia | 科 | 鳄鱼、湾鳄 |
| crocodylomorpha | 鳄形类 | reptilia | 目 | 按目处理，含化石类群 |
| pholidosauridae | 似鳄科 | crocodylomorpha | 科 | 帝鳄 |
| actinopterygii | 鱼纲 | chordata | 纲 | 玩法 fish |
| actinopterygii-incertae-sedis | 未定科 | actinopterygii | 科 | 预留；现有卡牌无鱼纲物种 |
| chondrichthyes | 软骨鱼纲 | chordata | 纲 | |
| lamniformes | 鼠鲨目 | chondrichthyes | 目 | |
| lamnidae | 鼠鲨科 | lamniformes | 科 | 大白鲨 |
| orectolobiformes | 须鲨目 | chondrichthyes | 目 | |
| rhincodontidae | 鲸鲨科 | orectolobiformes | 科 | 鲸鲨 |
| sarcopterygii | 肉鳍鱼纲 | chordata | 纲 | |
| coelacanthiformes | 腔棘鱼目 | sarcopterygii | 目 | |
| coelacanthidae | 腔棘鱼科 | coelacanthiformes | 科 | 腔棘鱼 |
| placodermi | 盾皮鱼纲 | chordata | 纲 | |
| arthrodira | 节甲鱼目 | placodermi | 目 | |
| dunkleosteidae | 邓氏鱼科 | arthrodira | 科 | 邓氏鱼 |
| agnatha | 无颌纲 | chordata | 纲 | 并系群，按纲处理 |
| myllokunmingiida | 昆明鱼目 | agnatha | 目 | |
| haikouichthyidae | 海口鱼科 | myllokunmingiida | 科 | 海口鱼 |
| cypriniformes | 鲤形目 | actinopterygii | 目 | |
| cyprinidae | 鲤科 | cypriniformes | 科 | 鲤鱼 |
| salmoniformes | 鲑形目 | actinopterygii | 目 | |
| salmonidae | 鲑科 | salmoniformes | 科 | 鲑鱼 |
| perciformes | 鲈形目 | actinopterygii | 目 | |
| scombridae | 鲭科 | perciformes | 科 | 金枪鱼 |
| pomacentridae | 雀鲷科 | perciformes | 科 | 小丑鱼 |
| tetraodontiformes | 鲀形目 | actinopterygii | 目 | |
| tetraodontidae | 鲀科 | tetraodontiformes | 科 | 河豚 |
| syngnathiformes | 海龙目 | actinopterygii | 目 | 预留 |
| syngnathidae | 海龙科 | syngnathiformes | 科 | 预留 |
| beloniformes | 颌针鱼目 | actinopterygii | 目 | |
| exocoetidae | 飞鱼科 | beloniformes | 科 | 飞鱼 |
| anguilliformes | 鳗鲡目 | actinopterygii | 目 | |
| anguillidae | 鳗鲡科 | anguilliformes | 科 | 鳗鲡 |
| lophiiformes | 鮟鱇目 | actinopterygii | 目 | 预留 |
| lophiidae | 鮟鱇科 | lophiiformes | 科 | 预留 |
| arthropoda | 节肢动物门 | animalia | 门 | |
| insecta | 昆虫纲 | arthropoda | 纲 | 玩法 insect |
| blattodea | 蜚蠊目 | insecta | 目 | |
| termitidae | 白蚁科 | blattodea | 科 | |
| diptera | 双翅目 | insecta | 目 | |
| drosophilidae | 果蝇科 | diptera | 科 | |
| hymenoptera | 膜翅目 | insecta | 目 | |
| formicidae | 蚁科 | hymenoptera | 科 | 蚂蚁 |
| apidae | 蜜蜂科 | hymenoptera | 科 | 蜜蜂、蜜蜂皇后、工蜂 Token |
| orthoptera | 直翅目 | insecta | 目 | |
| acrididae | 蝗科 | orthoptera | 科 | 蝗虫 |
| mantodea | 螳螂目 | insecta | 目 | |
| mantidae | 螳螂科 | mantodea | 科 | 螳螂 |
| lepidoptera | 鳞翅目 | insecta | 目 | |
| papilionidae | 凤蝶科 | lepidoptera | 科 | 蝴蝶 |
| coleoptera | 鞘翅目 | insecta | 目 | |
| scarabaeidae | 金龟科 | coleoptera | 科 | 甲虫 |
| odonata | 蜻蜓目 | insecta | 目 | |
| libellulidae | 蜻科 | odonata | 科 | 蜻蜓 |
| meganisoptera | 巨蜻蜓目 | insecta | 目 | 按目处理，已灭绝 |
| meganeuridae | 巨脉蜓科 | meganisoptera | 科 | 巨脉蜻蜓 |
| mollusca | 软体动物门 | animalia | 门 | |
| cephalopoda | 头足纲 | mollusca | 纲 | 玩法 cephalopod |
| octopoda | 八腕目 | cephalopoda | 目 | |
| octopodidae | 章鱼科 | octopoda | 科 | |
| nautilida | 鹦鹉螺目 | cephalopoda | 目 | |
| nautilidae | 鹦鹉螺科 | nautilida | 科 | 鹦鹉螺 |
| sepiida | 乌贼目 | cephalopoda | 目 | |
| sepiidae | 乌贼科 | sepiida | 科 | 乌贼 |
| myopsida | 闭眼目 | cephalopoda | 目 | |
| loliginidae | 枪乌贼科 | myopsida | 科 | 枪乌贼 |
| argonautidae | 船蛸科 | octopoda | 科 | 船蛸 |
| oegopsida | 开眼目 | cephalopoda | 目 | |
| architeuthidae | 大王乌贼科 | oegopsida | 科 | 大王乌贼 |
| cranchiidae | 小头鱿科 | oegopsida | 科 | 大王酸浆鱿 |
| vampyromorphida | 幽灵蛸目 | cephalopoda | 目 | |
| vampyroteuthidae | 幽灵蛸科 | vampyromorphida | 科 | 吸血鬼乌贼 |
| ammonitida | 菊石目 | cephalopoda | 目 | |
| ammonitidae | 菊石科 | ammonitida | 科 | 菊石、菊石幼体 Token |
| cnidaria | 刺胞动物门 | animalia | 门 | |
| hydrozoa | 水螅纲 | cnidaria | 纲 | 玩法 cnidarian |
| anthoathecata | 花水螅目 | hydrozoa | 目 | |
| oceaniidae | 海螅水母科 | anthoathecata | 科 | |
| hydridae | 水螅科 | anthoathecata | 科 | 水螅 |
| siphonophorae | 管水母目 | hydrozoa | 目 | |
| physaliidae | 僧帽水母科 | siphonophorae | 科 | 僧帽水母 |
| anthozoa | 珊瑚纲 | cnidaria | 纲 | |
| scleractinia | 石珊瑚目 | anthozoa | 目 | |
| poritidae | 滨珊瑚科 | scleractinia | 科 | 珊瑚虫、珊瑚虫 Token |
| acroporidae | 鹿角珊瑚科 | scleractinia | 科 | 鹿角珊瑚 |
| alcyonacea | 软珊瑚目 | anthozoa | 目 | |
| alcyoniidae | 软珊瑚科 | alcyonacea | 科 | 软珊瑚 |
| coralliidae | 红珊瑚科 | alcyonacea | 科 | 红珊瑚 |
| actiniaria | 海葵目 | anthozoa | 目 | |
| actiniidae | 海葵科 | actiniaria | 科 | 海葵 |
| scyphozoa | 钵水母纲 | cnidaria | 纲 | |
| semaeostomeae | 旗口水母目 | scyphozoa | 目 | |
| ulmaridae | 海月水母科 | semaeostomeae | 科 | 海月水母 |
| rhizostomeae | 根口水母目 | scyphozoa | 目 | |
| rhizostomatidae | 根口水母科 | rhizostomeae | 科 | 巨型水母 |
| cubozoa | 立方水母纲 | cnidaria | 纲 | |
| chirodropida | 多手立方水母目 | cubozoa | 目 | |
| chirodropidae | 多手立方水母科 | chirodropida | 科 | 箱水母 |
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
| asparagales | 天门冬目 | liliopsida | 目 | |
| orchidaceae | 兰科 | asparagales | 科 | 兰花 |
| ginkgophyta | 银杏门 | plantae | 门 | |
| ginkgopsida | 银杏纲 | ginkgophyta | 纲 | |
| ginkgoales | 银杏目 | ginkgopsida | 目 | |
| ginkgoaceae | 银杏科 | ginkgoales | 科 | 银杏 |
| plantae-incertae-sedis | 未定科 | plantae | 科 | 虚构 / 未定科 |
| fungi | 真菌界 | | 界 | 玩法 fungus |
| fungi-incertae-sedis | 未定科 | fungi | 科 | 虚构 / 未定科 |
| ascomycota | 子囊菌门 | fungi | 门 | |
| saccharomycetes | 酵母纲 | ascomycota | 纲 | |
| saccharomycetales | 酵母目 | saccharomycetes | 目 | |
| saccharomycetaceae | 酵母科 | saccharomycetales | 科 | 酵母 |
| eurotiomycetes | 散囊菌纲 | ascomycota | 纲 | |
| eurotiales | 散囊菌目 | eurotiomycetes | 目 | |
| aspergillaceae | 曲霉科 | eurotiales | 科 | 青霉、曲霉 |
| pezizomycetes | 盘菌纲 | ascomycota | 纲 | |
| pezizales | 盘菌目 | pezizomycetes | 目 | |
| tuberaceae | 块菌科 | pezizales | 科 | 松露 |
| sordariomycetes | 粪壳菌纲 | ascomycota | 纲 | |
| hypocreales | 肉座菌目 | sordariomycetes | 目 | |
| ophiocordycipitaceae | 线虫草科 | hypocreales | 科 | 冬虫夏草 |
| clavicipitaceae | 麦角科 | hypocreales | 科 | 麦角 |
| basidiomycota | 担子菌门 | fungi | 门 | |
| agaricomycetes | 伞菌纲 | basidiomycota | 纲 | |
| agaricales | 伞菌目 | agaricomycetes | 目 | |
| agaricaceae | 蘑菇科 | agaricales | 科 | 蘑菇、马勃 |
| amanitaceae | 鹅膏科 | agaricales | 科 | 毒蝇伞 |
| omphalotaceae | 光茸菌科 | agaricales | 科 | 香菇 |
| physalacriaceae | 膨瑚菌科 | agaricales | 科 | 蜜环菌 |
| prototaxales | 原杉菌目 | agaricomycetes | 目 | 分类存疑 |
| prototaxitaceae | 原杉菌科 | prototaxales | 科 | 原杉菌 |
| mucoromycota | 毛霉门 | fungi | 门 | 预留 |
| mucoromycetes | 毛霉纲 | mucoromycota | 纲 | 预留 |
| mucorales | 毛霉目 | mucoromycetes | 目 | 预留 |
| mucoraceae | 毛霉科 | mucorales | 科 | 预留 |
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
| enterobacteriaceae | 肠杆菌科 | enterobacterales | 科 | 大肠杆菌、超级细菌、细菌 Token |
| lactobacillaceae | 乳杆菌科 | lactobacillales | 科 | 乳酸菌 |
| staphylococcaceae | 葡萄球菌科 | bacillales | 科 | 葡萄球菌 |
| clostridia | 梭菌纲 | firmicutes | 纲 | |
| clostridiales | 梭菌目 | clostridia | 目 | |
| clostridiaceae | 梭菌科 | clostridiales | 科 | 破伤风梭菌 |
| uroviricota | 有尾噬菌体门 | virus | 门 | 噬菌体按病毒归类 |
| caudoviricetes | 有尾噬菌体纲 | uroviricota | 纲 | |
| caudovirales | 有尾噬菌体目 | caudoviricetes | 目 | |
| myoviridae | 肌尾噬菌体科 | caudovirales | 科 | 噬菌体 |
| protozoa | 原生生物界 | | 界 | 玩法 protozoa |
| apicomplexa | 顶复门 | protozoa | 门 | |
| aconoidasida | 无腔纲 | apicomplexa | 纲 | |
| haemosporida | 血孢子虫目 | aconoidasida | 目 | |
| plasmodiidae | 疟原虫科 | haemosporida | 科 | |
| ciliophora | 纤毛门 | protozoa | 门 | |
| oligohymenophorea | 寡膜纲 | ciliophora | 纲 | |
| peniculida | 草履虫目 | oligohymenophorea | 目 | |
| parameciidae | 草履虫科 | peniculida | 科 | 草履虫 |
| sessilida | 固着目 | oligohymenophorea | 目 | |
| vorticellidae | 钟虫科 | sessilida | 科 | 钟虫 |
| heterotrichea | 异毛纲 | ciliophora | 纲 | |
| heterotrichida | 异毛目 | heterotrichea | 目 | |
| stentoridae | 喇叭虫科 | heterotrichida | 科 | 喇叭虫 |
| amoebozoa | 变形虫门 | protozoa | 门 | |
| tubulinea | 管状根足纲 | amoebozoa | 纲 | |
| euamoebida | 真变形虫目 | tubulinea | 目 | |
| amoebidae | 变形虫科 | euamoebida | 科 | 变形虫、大变形虫、变形虫幼体 Token |
| euglenozoa | 眼虫门 | protozoa | 门 | |
| euglenoidea | 眼虫纲 | euglenozoa | 纲 | |
| euglenida | 眼虫目 | euglenoidea | 目 | |
| euglenaceae | 眼虫科 | euglenida | 科 | 眼虫 |
| kinetoplastea | 动质体纲 | euglenozoa | 纲 | |
| trypanosomatida | 锥虫目 | kinetoplastea | 目 | |
| trypanosomatidae | 锥虫科 | trypanosomatida | 科 | 锥虫 |
| metamonada | 后滴虫门 | protozoa | 门 | |
| fornicata | 叉鞭毛纲 | metamonada | 纲 | |
| diplomonadida | 双滴虫目 | fornicata | 目 | |
| hexamitidae | 六鞭毛虫科 | diplomonadida | 科 | 贾第虫 |
| foraminifera | 有孔虫门 | protozoa | 门 | |
| globothalamea | 球房虫纲 | foraminifera | 纲 | |
| rotaliida | 轮虫目 | globothalamea | 目 | |
| rotaliidae | 轮虫科 | rotaliida | 科 | 有孔虫 |
| radiolaria | 放射虫门 | protozoa | 门 | |
| polycystinea | 多囊纲 | radiolaria | 纲 | |
| spumellarida | 泡沫虫目 | polycystinea | 目 | |
| spongodiscidae | 海绵盘虫科 | spumellarida | 科 | 放射虫 |
| chlorophyta | 绿藻门 | protozoa | 门 | 按原生生物处理 |
| chlorophyceae | 绿藻纲 | chlorophyta | 纲 | |
| volvocales | 团藻目 | chlorophyceae | 目 | |
| volvocaceae | 团藻科 | volvocales | 科 | 团藻、团藻细胞 Token |
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
| picornavirales | 小RNA病毒目 | pisoniviricetes | 目 | |
| caliciviridae | 杯状病毒科 | picornavirales | 科 | 诺如病毒 |
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
