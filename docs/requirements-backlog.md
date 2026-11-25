# 产品需求总表

> 状态：跨版本文档，未排期项待决策
> 作用：汇总当前代码与文档中尚未完成的产品需求，登记状态、现有版本归属与来源，作为版本排期的输入
> 不描述：规则正文、技术实现方案、排期细节；规则以链接的承载文档为准

本文是跨版本的产品需求登记表。已实现状态以 `docs/current/` 与代码为准，版本归属只记录现有文档已声明的计划，不代表已重新排期。v0.2 的天梯与匹配、Android 导出与返回路由等已实现项不登记为需求；登记项只覆盖尚未完成、待确认或待决策的部分。内容素材库是参考材料，登记为素材需求时不代表已承诺上线。

## 1. 口径

| 状态 | 含义 |
| --- | --- |
| 未开始 | 尚无代码实现；产品规则或技术方案已存在 |
| 部分实现 | 已有实现，但缺真机验证、外部配置、上线步骤或观测能力 |
| 待确认 | 产品规则未确认，不能进入实施 |
| 待决策 | 需要决定是否立项、是否进入版本或改动已锁定规则 |
| 未排期 | 已登记、无版本归属，也未承诺实施 |
| 未解决 | 已知问题仍存在，解决方向或约束已记录 |
| 未验证 | 已有实现或修复，缺少环境、真机或数据验收 |
| 待修正 | 文档或状态标注与代码现状不一致 |
| 持续 | 不绑定单一版本的平衡、质量或观测工作 |

| 现有归属 | 含义 |
| --- | --- |
| v0.2 | [版本路线](version-roadmap.md) §2 的 M4 / M5 范围 |
| v0.3 | 版本路线 §2 的 M6 范围，或 v0.3 文档已声明 |
| v0.4 | 版本路线 §3 的第四版内容，尚未建立版本目录 |
| 待定 | 无现有归属，需在排期时决定 |

## 2. 总览

| 分类 | 条目数 | 归属分布 |
| --- | --- | --- |
| 平台与发行 | 10 | v0.2：9；待定：1 |
| 内容 | 8 | v0.2：1；v0.3：1；v0.4：1；未排期：4；待定：1 |
| 玩法与系统 | 14 | v0.3：5；v0.4：2；未排期：2；待决策：1；待定：4 |
| 社交与商业化 | 2 | v0.3：2 |
| 平衡与质量 | 9 | 持续：5；v0.2：4 |
| 工程能力 | 4 | 待定：3；v0.2：1 |
| 文档修正 | 4 | v0.2：1；v0.3：1；v0.4：1；待定：1 |
| 发行运营（非产品功能） | 5 | 未排期：5 |

## 3. 平台与发行

| 编号 | 需求 | 范围要点 | 当前状态 | 现有归属 | 来源 |
| --- | --- | --- | --- | --- | --- |
| PLAT-01 | prod 服务端上线 | prod namespace、数据库与配置；`make deploy ENV=prod`；prod 域名与 TLS；`/healthz`、`/readyz`、`/ws`；客户端经公网连接 prod 完成完整对局 | 部分实现：部署入口与 dev 首发已执行，prod 未执行，prod 域名未定，`CLIENT_PROD_SERVER_URL` 为空 | v0.2 | [部署与接入](current/technical/deployment/README.md) §8、[版本路线](version-roadmap.md) M4 |
| PLAT-02 | Android 真机验收 | 手机与平板各完成完整对局；系统返回键与返回手势；软键盘让位；安全区与命中尺寸；APK 与 AAB 均可安装 | 部分实现：导出、返回路由与设备判定已实现，真机对局未验证 | v0.2 | [Android 端适配](versions/v0.2/product/ui/android-adaptation.md)、[客户端技术基线](versions/v0.2/technical/architecture/client-stack.md) §4–§5 |
| PLAT-03 | Android 的 godotsteam 扩展处理 | 扩展声明 Android / Linux / Windows 库路径，仓库只有 macOS 二进制；Android 包内含 0 字节 `libgodotsteam*.so` 与 `libsteam_api.so`，启动加载失败；业务代码不取用，实际影响未验证 | 待决策：移除非 macOS 声明或按平台排除该扩展 | v0.2 | [客户端技术基线](versions/v0.2/technical/architecture/client-stack.md) §5 |
| PLAT-04 | 生产短信通道 | 国际手机号覆盖地区、费用与供应商选型；生产验证码替换开发固定值 `123456` | 未开始 | v0.2 | [v0.2 待确认事项](versions/v0.2/open-questions.md)、[发行渠道与账号](current/product/accounts-and-distribution.md) §3 |
| PLAT-05 | Steamworks 后台与商店页 | App ID；6 个成就的 API 名与图标；depot；商店页、截图与 5 秒预告；按[Steam 上线操作清单](versions/v0.2/technical/steam-onboarding.md)执行 | 未开始 | v0.2 | [v0.2 待确认事项](versions/v0.2/open-questions.md)、[销量预测](sales-forecast.md) §7.1 |
| PLAT-06 | Steam 成就真实同步验收 | 真实 Steamworks 票据、端到端解锁与幂等补同步 | 部分实现：触发、奖励、账号级幂等与消息下发已验证，真实同步待外部配置 | v0.2 | [发行渠道与账号](current/product/accounts-and-distribution.md) §8、[Steam 接入](current/technical/steam-integration.md) |
| PLAT-07 | Apple 签名与公证 | Apple Developer ID；公证账号；macOS 签名证书 | 未开始 | v0.2 | [v0.2 待确认事项](versions/v0.2/open-questions.md) |
| PLAT-08 | Google Play 发布配置 | 开发者账号；应用条目；AAB 上传配置 | 未开始 | v0.2 | [v0.2 待确认事项](versions/v0.2/open-questions.md)、[Android 端适配](versions/v0.2/product/ui/android-adaptation.md) |
| PLAT-09 | Windows 构建与 Steam depot | Windows 导出预设；Steam depot；Steam 成就与云存档路径 | 未开始：`client/export_presets.cfg` 只有 macOS、iOS、Steam(macOS)、macOS Local Debug、Android 与 Android AAB | 待定 | [销量预测](sales-forecast.md) §7.1（列为 P0） |
| PLAT-10 | 上线数据记录与查看 | 崩溃率、意图拒绝率与单局时长在客户端与服务端记录并可查看；不设通过阈值 | 部分实现：服务端每局输出 `game_over` 观察日志，含拒绝率与时长原始值；无持久化与查看端，客户端崩溃率未采集 | v0.2 | [房间服务与联网](current/technical/room-service/README.md) §15、[版本路线](version-roadmap.md) M4 |

## 4. 内容

| 编号 | 需求 | 范围要点 | 当前状态 | 现有归属 | 来源 |
| --- | --- | --- | --- | --- | --- |
| CONT-01 | v0.2 卡池增量 | +20 随从、+2 英雄卡；素材首批另含 4 张效果牌与 1 张衍生物 | 未开始：当前 332 随从、100 效果牌、32 英雄卡，首批卡未进入卡池 | v0.2 | [版本路线](version-roadmap.md) §3、[内容素材库存](current/product/content/reference/content-inventory.md) §7.1–§7.4 |
| CONT-02 | v0.3 卡池增量 | +20 随从（高血人物与植物补强）、+2 英雄卡 | 未开始：尚未建立卡牌设计 | v0.3 | [版本路线](version-roadmap.md) §3 |
| CONT-03 | v0.4 卡池增量 | +20 随从、+2 英雄卡 | 未开始：版本目录未建立 | v0.4 | [版本路线](version-roadmap.md) §3 |
| CONT-04 | 素材库存 152 张精选卡落地 | 77 张 A 类、45 张 B 类、30 张 C 类；9 项新机制、39 个模板、69 个技能；按 0–5 阶段执行 | 未排期：参考材料，不代表已承诺上线 | 未排期 | [内容素材库存](current/product/content/reference/content-inventory.md) §9.1 |
| CONT-05 | 诺奖一人一卡全量名册 | 1901–2026 全部诺奖得主与机构，1028 位一人一卡 | 未排期：参考材料 | 未排期 | [内容素材库存](current/product/content/reference/content-inventory.md) 第二部分 |
| CONT-06 | 未采用素材再评估 | 未进入首版卡池的素材与再评估方向，约 33 项 | 未排期 | 未排期 | [内容素材库](content-backlog.md) |
| CONT-07 | 逐卡音效 | 当前 9 个默认合成占位音效，全部卡牌回落默认；逐卡 `cards/<cardId>/{buy,attack,death}.wav` 未制作 | 未开始 | 未排期 | [Godot 客户端](current/technical/client.md) §11、[对局页](current/product/ui/pages/game.md) §8 |
| CONT-08 | 内容多语言 | 432 张卡、262 个技能、32 张英雄卡与全部 UI 字符串；内容 schema 先定，分批翻译 | 未开始：仅 `client/text/zh_CN.zh_CN.translation` | 待定 | [销量预测](sales-forecast.md) §7.1（列为 P0） |

## 5. 玩法与系统

| 编号 | 需求 | 范围要点 | 当前状态 | 现有归属 | 来源 |
| --- | --- | --- | --- | --- | --- |
| SYS-01 | 场景默认开启 | 8 个战场从扩展层转为默认开启；可单独开关；关闭时与 v0.2 行为一致 | 未开始：规则已写，首版默认关闭 | v0.3 | [时代与场景](current/product/rules/eras-and-scenarios.md) §4、[v0.3 版本入口](versions/v0.3/README.md) |
| SYS-02 | 大决战模式 | 10 卡位；复用会战商店概率与牌池份数；可单独开关 | 未开始：只有配置参数 | v0.3 | [模式](current/product/systems/modes.md) §3、[v0.3 版本入口](versions/v0.3/README.md) |
| SYS-03 | 英雄坐镇 | 每个时代一张英雄卡坐镇，提供该时代期间的全队加成；不占每回合技能次数 | 未开始：只有配置项 | v0.3 | [模式](current/product/systems/modes.md) §5、[v0.3 版本入口](versions/v0.3/README.md) |
| SYS-04 | 陨落与遗嘱 | 星级 ≥5 的人科随从在场 3 回合后消失；消失前触发遗嘱，完成给永久奖励 | 未开始：只有设计 | v0.3 | [设计总纲](current/product/overview/design-overview.md) §6.9、[v0.3 版本入口](versions/v0.3/README.md) |
| SYS-05 | 赛季 | 赛季周期、积分软重置、卡牌轮换 | 待确认：长度、重置幅度与轮换比例未定 | v0.3 | [v0.3 待确认事项](versions/v0.3/open-questions.md)、[版本路线](version-roadmap.md) M6 |
| SYS-06 | 剧情模式第一章 | 剧情第一章的关卡与叙事内容；进度系统 | 未开始：首版明确非目标，第四版列出第一章 | v0.4 | [模式](current/product/systems/modes.md) §4、[版本路线](version-roadmap.md) §3 |
| SYS-07 | 关卡编辑器与剧情文本 | 支撑剧情模式的编辑与文本工具 | 未开始 | v0.4 | [版本路线](version-roadmap.md) §3 |
| SYS-08 | 观战与赛事 | 观战入口、只读快照消费方与赛事规则 | 未排期：首版不做，赛事规则留到后续版本再定；v0.1 待确认事项提到的「观战字段裁剪」未进入 v0.2 清单 | 未排期 | [模式](current/product/systems/modes.md) §1、[v0.1 待确认事项](versions/v0.1/open-questions.md) |
| SYS-09 | 断线 / AFK 的 AI 托管 | 设计总纲把它列为首版目标；当前实现与房间文档明确「对局中不做托管」；需确认产品规则 | 待决策 | 待决策 | [设计总纲](current/product/overview/design-overview.md) §3、[房间与邀请](current/product/systems/rooms-and-invites.md) §8 |
| SYS-10 | 每日种子局与排行榜 | 固定种子的每日对局与排行榜 | 未排期：销量预测 P2 候选 | 待定 | [销量预测](sales-forecast.md) §7.3 |
| SYS-11 | 单人模式与难度递进 | 单人入口；3–10 档难度；复用现有卡与规则引擎 | 未排期：销量预测列为 L1 / P2 候选，未立项 | 待定 | [销量预测](sales-forecast.md) §7.3 |
| SYS-12 | 局外解锁与教学局 | 局外解锁；用简单难度机器人局承接新手教学 | 未排期：现有首局非阻塞引导不等于教学局 | 待定 | [销量预测](sales-forecast.md) §7.3 |
| SYS-13 | 连续 run | 单局循环拉长为连续 run；依赖单人模式 D7 回访达标 | 待决策：前置指标未验证，不立项 | 待定 | [销量预测](sales-forecast.md) §7.3、§8.1 |
| SYS-14 | 违规与消极比赛处理 | 举报、消极比赛判定、挂机处罚与禁赛 | 未排期：v0.2 明确不新增，未承诺版本 | 未排期 | [天梯与匹配](versions/v0.2/product/systems/ranked-and-matchmaking.md) §9 |

## 6. 社交与商业化

| 编号 | 需求 | 范围要点 | 当前状态 | 现有归属 | 来源 |
| --- | --- | --- | --- | --- | --- |
| SOC-01 | 好友榜与好友小队 | 好友内积分榜；2–4 人小队排队，队内分差 ≤400，按平均积分匹配；一局最多一个小队 | 未开始：产品文档已有，规则未实施 | v0.3 | [好友榜与好友小队](versions/v0.3/product/systems/friends-ranked.md) |
| SOC-02 | 商业化 | 外观与英雄卡选择位；不卖战力；外观品类、收费方式与定价；是否引入赛季通行证 | 待确认：品类、定价与通行证关系未定 | v0.3 | [v0.3 待确认事项](versions/v0.3/open-questions.md)、[版本路线](version-roadmap.md) M6 |

## 7. 平衡与质量

| 编号 | 需求 | 范围要点 | 当前状态 | 现有归属 | 来源 |
| --- | --- | --- | --- | --- | --- |
| BAL-01 | 13 个玩法夺冠率收敛 | 目标区间 4%–12%；当前实测 3‰–361‰，主因是玩法卡池厚度差；收敛需要允许调整卡牌玩法归属、牌池份数或 AI 招募策略 | 未解决 | 持续 | [平衡问题](current/product/content/balance-questions.md) §1、[平衡测算](current/product/content/balance-analysis.md) |
| BAL-02 | 平均回合数回落到 16–19 | 当前 1000 局 `avg_turns` 为 20，且 1000 局全部打满第 20 回合；需真人局确认单局时长后再决定是否上调伤害曲线 | 未解决 | 持续 | [平衡问题](current/product/content/balance-questions.md) §1 |
| BAL-03 | 单局 20–30 分钟目标验证 | 设计目标 20–30 分钟；目前只有 AI 对局回合数，没有真人单局时长 | 未验证 | 持续 | [版本路线](version-roadmap.md) §4、[设计总纲](current/product/overview/design-overview.md) §6.10 |
| BAL-04 | 其余待确认平衡项 | 随从统一 3 金币、卖血升级、攻 0 随从占比、关键词密度、七下西洋经济与平局、单卡强度等待观察项 | 待确认 | 持续 | [平衡问题](current/product/content/balance-questions.md) §1 |
| BAL-05 | 模拟报告字段补齐 | 缺「流派进前四率」、升满时代时点、空星级重抽与空槽统计；补齐后才能完成相应判定 | 未开始 | 持续 | [平衡问题](current/product/content/balance-questions.md) §1、§3 |
| BAL-06 | 天梯积分与段位校准 | K=200 与段位区间是否让高分段过快收敛、段位分布失衡；上线后用模拟对局校准 | 待确认 | v0.2 | [v0.2 待确认事项](versions/v0.2/open-questions.md) |
| BAL-07 | 已修复缺陷的环境复验 | 整桌断线对局收束待 dev 部署后验证压测样本消失；大厅排队计时待开发环境客户端复验 | 待验证 | v0.2 | [整桌断线缺陷](versions/v0.1/bugfix/2026-11-24-abandoned-game-room/bugfix.md)、[排队计时缺陷](versions/v0.1/bugfix/2026-11-24-lobby-queue-timer/bugfix.md) |
| BAL-08 | AI 补位规则文档冲突 | 设计总纲 §3 与版本路线 §4 写「不设 AI 补位 / 不引入 AI 补位」，但代码默认 `bot_fill: true`，房间文档已写成已实现规则 | 待修正：改写锁定规则并补决策记录 | v0.2 | [销量预测](sales-forecast.md) §7.1、[房间与邀请](current/product/systems/rooms-and-invites.md) §4.1 |
| BAL-09 | 过期状态标注修正 | v0.2 产品入口仍写天梯与 Android「方案未实施」；`current` 移动端适配仍写 Android 未实现 | 待修正 | v0.2 | [v0.2 产品增量](versions/v0.2/product/README.md)、[天梯与匹配](versions/v0.2/product/systems/ranked-and-matchmaking.md)、[移动端适配](current/product/ui/mobile-adaptation.md) |

## 8. 工程能力

| 编号 | 需求 | 范围要点 | 当前状态 | 现有归属 | 来源 |
| --- | --- | --- | --- | --- | --- |
| ENG-01 | 房间、对局状态、意图日志与遥测持久化 | 崩溃恢复所需的状态持久化与恢复；当前只落账号、成就与社交数据，其余在进程内或未实现 | 未开始 | 待定 | [房间服务与联网](current/technical/room-service/README.md) §14、[服务端技术基线](current/technical/architecture/server-stack.md) |
| ENG-02 | 多副本、多区部署与容量 | 当前单实例、`Recreate`、进程内房间状态；多副本前需把迁移改为独立 Job，并定扩容、限流与分区方案 | 未开始 | 待定 | [部署与接入](current/technical/deployment/README.md) §6、[销量预测](sales-forecast.md) §7.1 |
| ENG-03 | 远端 `wss://` 压测 | 压测工具只支持 `ws://`，压远端需先做端口转发 | 未开始 | 待定 | [规则引擎与模拟器](current/technical/rule-engine-and-simulator.md) §11、[房间服务与联网](current/technical/room-service/README.md) |
| ENG-04 | 客户端崩溃率采集 | 上线数据要求的客户端崩溃率；当前无采集与上报 | 未开始 | v0.2 | [版本路线](version-roadmap.md) M4、[房间服务与联网](current/technical/room-service/README.md) §15 |

## 9. 文档修正

| 编号 | 需求 | 范围要点 | 当前状态 | 现有归属 | 来源 |
| --- | --- | --- | --- | --- | --- |
| DOC-01 | v0.3 扩展项产品文档 | 场景、大决战、赛季、商业化、陨落、遗嘱与英雄坐镇尚未建立产品文档 | 未开始 | v0.3 | [v0.3 待确认事项](versions/v0.3/open-questions.md)、[v0.3 产品增量](versions/v0.3/product/README.md) |
| DOC-02 | 第四版版本文档 | 版本路线已列第四版内容，但 `docs/versions/` 未建立 v0.4 目录 | 未开始 | v0.4 | [版本路线](version-roadmap.md) §3 |
| DOC-03 | 登录页文档 | 登录页场景已实现，未单独成文 | 未开始 | 待定 | [UI 页面索引](current/product/ui/pages/README.md) |
| DOC-04 | 观战字段裁剪待确认项 | v0.1 待确认事项记录该条转入 v0.2，但 v0.2 清单没有该条 | 待修正 | v0.2 | [v0.1 待确认事项](versions/v0.1/open-questions.md)、[v0.2 待确认事项](versions/v0.2/open-questions.md) |

## 10. 明确排除（当前范围）

以下事项在当前版本或首版范围中明确排除；其中部分已被销量预测列为后续候选，重新提出需求时才进入排期。

| 事项 | 依据 |
| --- | --- |
| 私聊、语音、表情、拉黑、赠送、关注、战队与社区 | [好友](current/product/systems/friends.md) §7 |
| 公开房间列表、随机加入、客户端展示或手动输入房间码、邀请链接与二维码 | [房间与邀请](current/product/systems/rooms-and-invites.md) §1、§10 |
| 对局中补位、对局中移除成员、机器人难度设置 | [房间与邀请](current/product/systems/rooms-and-invites.md) §10 |
| 竖屏布局、浅色主题与主题切换 | [移动端适配](current/product/ui/mobile-adaptation.md) §1、[客户端 UI](current/product/ui/README.md) §1 |
| 首版教学局 | [设计总纲](current/product/overview/design-overview.md) §3 |
| 付费变强 | [设计总纲](current/product/overview/design-overview.md) §3 |
| iOS App Store、TestFlight 与公证分发 | [移动端适配](current/product/ui/mobile-adaptation.md) §7 |
| 中国区发行（版号、未成年人防沉迷实名、教育类备案） | [销量预测](sales-forecast.md) §9 |
| 手机号换绑、解绑；Steam 账号与已有手机号账号合并；展示名修改 | [发行渠道与账号](current/product/accounts-and-distribution.md) §2–§3 |

## 11. 版本归属现状

| 版本 | 已登记的未完成项 |
| --- | --- |
| v0.2 | PLAT-01–PLAT-08、PLAT-10、CONT-01、BAL-06–BAL-09、ENG-04、DOC-04；PLAT-09 与 CONT-08 未定 |
| v0.3 | SYS-01–SYS-05、SOC-01、SOC-02、CONT-02、DOC-01 |
| v0.4 | SYS-06、SYS-07、CONT-03、DOC-02 |
| 待决策 | SYS-09 |
| 未排期 / 待定 | CONT-04–CONT-08、SYS-08、SYS-10–SYS-14、BAL-01–BAL-05、ENG-01–ENG-03、PLAT-09、DOC-03、OPS-01–OPS-05 |

## 12. 待决策事项

| 编号 | 决策 | 判断依据 | 建议 |
| --- | --- | --- | --- |
| D-01 | 扩展层是否提前到 v0.2 | v0.2 的 M4 未列场景；场景与陨落都会扩大平衡与测试面，版本路线明确要求核心循环稳定后再加入 | 保持 v0.3，不提前 |
| D-02 | Windows 构建是否立项 | 销量预测列为 P0；当前导出预设与 Steam depot 均无 Windows | 在 v0.2 上线前评估，最迟与 Steam 商店页同批 |
| D-03 | 内容多语言是否立项 | 销量预测列为 P0；卡面文本全中文，只有 UI 走 `tr()` | 先定内容 schema 与翻译批次，再决定版本；不晚于商店页上线 |
| D-04 | 单人模式是否立项 | 销量预测认为单人可玩与纯联机差一个量级，但会引入独立平衡面与验证面 | 先做难度递进与入口评估，不直接承诺连续 run |
| D-05 | AI 托管与 AI 补位的规则口径 | 设计总纲把 AI 托管列为首版目标、版本路线写不引入 AI 补位，但代码默认 `bot_fill: true`，当前实现也不做托管 | 一起确认：保留机器人补位并改写版本路线与设计总纲、确认不托管，补决策记录 |
| D-06 | 观战与赛事是否立项 | 规则只留口，未定版本；观战字段裁剪待确认项已丢失 | 先补观战字段裁剪结论，再决定版本 |
| D-07 | 新需求的版本归属 | 本表未排期项与新增需求需要统一排期 | 按本表逐项决定，结论回写版本 `README.md` 与 `open-questions.md` |

## 13. 发行运营项（非产品功能）

以下事项影响上线与曝光，但不是产品功能需求，不进入版本功能清单；按销量预测的日历关键路径单独跟踪。

| 编号 | 事项 | 范围要点 | 当前状态 | 现有归属 | 来源 |
| --- | --- | --- | --- | --- | --- |
| OPS-01 | Demo | 支持加入付费玩家房间，或开放 1–2 个完整时代；与新品节档期绑定 | 未排期 | 未排期 | [销量预测](sales-forecast.md) §7.2 |
| OPS-02 | Steam 新品节 | 至少两次（预热与首发前）；需提前 3–4 个月报名 | 未排期 | 未排期 | [销量预测](sales-forecast.md) §7.2 |
| OPS-03 | 愿望单 | 上线前 ≥30,000；首周销量约为愿望单的 10%–20% | 未排期：当前愿望单数为 0 或未知 | 未排期 | [销量预测](sales-forecast.md) §7.2、§8.1 |
| OPS-04 | 宣发账号 | 冷启动需 3–6 个月；上线前持续运营 | 未排期 | 未排期 | [销量预测](sales-forecast.md) §7.2、§7.4 |
| OPS-05 | 家长 / 教育渠道与 Apple Arcade | 需短局、教学目标与渠道关系；Apple Arcade 有排他代价；均未计入主预测 | 未排期：评估项 | 未排期 | [销量预测](sales-forecast.md) §9.1、§9.3 |

## 相关文档

- [版本路线与风险](version-roadmap.md)
- [版本总览](versions/README.md)
- [内容素材库](content-backlog.md)
- [销量预测](sales-forecast.md)
- [v0.2 待确认事项](versions/v0.2/open-questions.md)
- [v0.3 待确认事项](versions/v0.3/open-questions.md)
- [平衡问题](current/product/content/balance-questions.md)
