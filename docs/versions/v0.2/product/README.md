# 产品基线增量

本文索引 v0.2 相对 v0.1 的产品增量文档；已实现的 v0.1 内容不在此重复。

| 文档 | 内容 |
| --- | --- |
| [Android 端适配](ui/android-adaptation.md) | Android 手机与平板横屏适配、返回键与键盘规则、APK 与 Google Play AAB 分发 |
| [天梯与匹配](systems/ranked-and-matchmaking.md) | 自动匹配、动态积分、段位、全服榜与个人主页 |
| [房间等待区与对局内选英雄](systems/room-waiting-and-hero-pick.md) | 房间页准备/开始/机器人补位，选英雄移到对局页独立阶段 |
| [成就](systems/achievements.md) | 6 个局内成就的接入、启用方式与设计原则 |
| [单客户端登录与对局接管](systems/single-client-login.md) | 同账号新登录顶掉旧端、旧令牌失效、新端接管房间与对局 |
| [成就索引](content/achievements/README.md) | 6 个成就实体文档 |

状态：[发行渠道与账号](../../../current/product/accounts-and-distribution.md) 已实现 Steam 契约与适配（含 6 个成就运行时），不再保留版本基线；[Android 端适配](ui/android-adaptation.md) 已实现导出、返回路由与设备判定，真机对局未验证；[天梯与匹配](systems/ranked-and-matchmaking.md) 已实现；[房间等待区与对局内选英雄](systems/room-waiting-and-hero-pick.md) 已实现，规则已回写至[房间与邀请](../../../current/product/systems/rooms-and-invites.md)与[房间等待区页面](../../../current/product/ui/pages/room.md)；[单客户端登录与对局接管](systems/single-client-login.md) 已实现，规则已回写至[发行渠道与账号](../../../current/product/accounts-and-distribution.md)与[协议架构](../../../current/technical/architecture/protocol.md)。
