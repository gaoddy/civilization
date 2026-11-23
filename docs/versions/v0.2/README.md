# v0.2

> 状态：规划中

本文是 v0.2 的版本入口，描述版本主题、相对 v0.1 的能力增量与文档导航；不重复已实现的 v0.1 产品基线。

## 主题

在 v0.1 的桌面端与 iOS 房间制对局之上完成 Android 端适配、服务端上线与首版上线，并加入成就、自动匹配与天梯。

## 相对 v0.1 的增量

| 内容 | 范围 |
| --- | --- |
| Android 端适配 | Android 手机与平板横屏适配；缩放、触控、安全区、返回键与键盘规则沿用 iOS；APK 侧载与 Google Play AAB 并存，见[Android 端适配](product/ui/android-adaptation.md) |
| 服务端上线 | prod 环境部署、域名与 TLS、数据库迁移与健康检查；沿用 `make deploy` 发布通道 |
| 发行渠道与账号 | macOS Steam 国际版构建、Steam 自动登录、手机号绑定、Steam 成就同步与 Steam 好友邀请；契约与适配已实现，见[当前发行渠道与账号](../../current/product/accounts-and-distribution.md) |
| 成就 | 6 个局内成就接入运行时 |
| 自动匹配 | 8 人按积分匹配，等待时间逐级放宽；房间局仍不计分 |
| 天梯 | 会战自动匹配局计分，动态积分差、8 段位与全服榜；房间局仍不计分 |

## 文档导航

- 产品增量：[product/README.md](product/README.md)
- 技术增量：[客户端技术基线](technical/architecture/client-stack.md)、[Steam 接入技术现状](../../current/technical/steam-integration.md)、[服务端部署与接入](../../current/technical/deployment/README.md)
- Steam 上线操作：[Steam 上线操作清单](technical/steam-onboarding.md)
- 未确认事项：[open-questions.md](open-questions.md)
- 跨版本路线：[版本路线与风险](../../version-roadmap.md)

## 数据与校验

卡牌与技能实体文档以已实现产品状态（`docs/current/product/content/`）为唯一数据源；6 个成就实体文档随本版本位于 `product/content/achievements/`。两类数据均由 `python3 scripts/validate_docs.py` 校验。
