# 待确认事项

本文只保留 v0.2 尚未确认的事项；结论确认后写入对应文档，并从本文移除。

## 天梯与匹配

| 问题 | 影响范围 | 待确认 | 来源 |
| --- | --- | --- | --- |
| 天梯 K=200 与段位区间是否让高分段过快收敛、段位分布失衡 | 天梯 | 上线前用模拟对局校准 | [天梯与匹配](product/systems/ranked-and-matchmaking.md) |
| 匹配放宽 ±150/±300/±500 在 8 人局是否导致等待过长或对局强弱悬殊 | 天梯 | 内测数据校准 | [天梯与匹配](product/systems/ranked-and-matchmaking.md) |
| 好友小队 2–4 人、队内分差 400 是否足以限制同局操纵名次 | 天梯 | 观察内测对局 | [好友](../../current/product/systems/friends.md) |
| 赛季长度、软重置幅度与卡牌轮换比例 | 赛季 | 商业化方案确定后校准 | [版本路线](../../version-roadmap.md) |

## 商业化

| 问题 | 影响范围 | 待确认 | 来源 |
| --- | --- | --- | --- |
| 外观品类、英雄卡选择位收费方式与定价 | 商业化 | 需求澄清 | [版本路线](../../version-roadmap.md) |
| 是否引入赛季通行证，以及通行证与段位奖励的关系 | 商业化 | 需求澄清 | [版本路线](../../version-roadmap.md) |

## 发行与账号

本节各项的准备与上线顺序见 [Steam 上线操作清单](technical/steam-onboarding.md)。

| 问题 | 影响范围 | 待确认 | 来源 |
| --- | --- | --- | --- |
| 国际手机号绑定需要真实短信通道，覆盖地区、费用与供应商未定 | 账号绑定 | 上线前接入短信服务 | [发行渠道与账号](../../current/product/accounts-and-distribution.md) |
| Steam 国际版客户端连接的服务端域名、TLS 证书与部署环境未定 | 发行与服务端 | 部署方案确定 | [发行渠道与账号](../../current/product/accounts-and-distribution.md) |
| Steamworks 后台 App ID、6 个成就的 API 名与图标、depot 与商店页配置未建 | 发行与成就同步 | 发布前配置 | [发行渠道与账号](../../current/product/accounts-and-distribution.md) |
| Apple Developer ID、公证账号与 macOS 签名证书未准备 | 发行 | 发布前准备 | [发行渠道与账号](../../current/product/accounts-and-distribution.md) |

## 移动端与版本范围

| 问题 | 影响范围 | 待确认 | 来源 |
| --- | --- | --- | --- |
| 移动端目标设备、断点与安全区处理 | UI 与适配 | UI 设计确定 | [客户端技术基线](technical/architecture/client-stack.md) |

## 观战与视图

| 问题 | 影响范围 | 待确认 | 来源 |
| --- | --- | --- | --- |
| 观战只读视图的字段裁剪 | 实现 | v0.1 不做观战，视图按可被第三方只读消费设计，裁剪规则待观战时确定 | [通信协议](../../current/technical/architecture/protocol.md) |

## 文档结构

| 问题 | 影响范围 | 待确认 | 来源 |
| --- | --- | --- | --- |
| v0.2 的场景、赛季、商业化与大决战尚未建立产品文档 | 文档结构 | 按版本开发流程逐项补齐 | 版本入口 |
