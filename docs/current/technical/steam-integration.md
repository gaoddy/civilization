# Steam 接入

> 状态：已实现（Steam 契约与适配、6 个成就运行时）；签名/公证/上传与真实验票待外部配置
> 关联：[发行渠道与账号](../product/accounts-and-distribution.md)、[Godot 客户端](client.md)、[房间服务与联网](room-service/README.md)、[部署与接入](deployment/README.md)

本文记录 Steam 渠道在客户端、协议、服务端与构建发布上的落点、边界和验证方式。

## 1. 客户端

| 位置 | 职责 |
| --- | --- |
| `client/addons/godotsteam/` | GodotSteam 4.24 GDExtension（Steamworks 1.65）；只保留 macOS 二进制，版本与来源记录在 `UPSTREAM.md` |
| `client/autoload/platform.gd` | 渠道判定与语义接口：`channel()`、`sign_in()`、大厅、邀请、成就；向上不暴露 Steam 类型 |
| `client/platform/dev_backend.gd` | 非 Steam 构建与测试用的空实现 |
| `client/platform/steam_backend.gd` | 唯一调用 Steam 单例的位置；票据、大厅与统计均在此封装 |
| `client/autoload/game_store.gd` | 账号视图、Steam 登录、绑定、邀请 token、成就状态与幂等记录 |
| `client/autoload/ws_client.gd` | `BuildConfig.SERVER_URL` 优先；Steam 构建回落到桌面地址 |
| `client/pages/login/` | Steam 渠道自动登录、失败文案与重试/退出；非 Steam 保持手机号表单 |
| `client/pages/account/` | 账号页与手机号绑定 |
| `client/pages/lobby/lobby_bottom_bar.gd` | Steam 渠道的大厅「账号」入口 |
| `client/pages/room/room_action_bar.gd` | Steam 渠道的「邀请 Steam 好友」入口 |
| `client/generated/build_config.gd` | 由 `scripts/gen_build_config.py` 生成；编辑器与本地调试为空配置，release 导出写入构建环境地址 |

- 渠道由导出预设的 `custom_features="steam"` 决定（`OS.has_feature("steam")`）；GDExtension 通过 `Engine.has_singleton("Steam")` 动态获取，不注册 autoload。
- 票据：`getAuthTicketForWebApi("civilization")` 后等待信号，10 秒超时；登录整体 12 秒超时。票据无效或 Steam 不可用只重试，不切换手机号。
- 大厅：私有大厅最多 8 人，metadata 写 `room_code`、`invite_token`、`invite_expires_at`、`inviter_name`；创建与加入均以“请求 + 轮询结果 + 超时”实现，失败时邀请入口置灰。
- 邀请接收：`join_requested` / `+connect_lobby` 统一转成 `GameStore.accept_steam_invite(token)`；未认证先缓存，已在房间只提示。
- 成就：收到 `AchievementState` / `AchievementUnlocked` 后按 `achievementId` 去重，Steam 渠道调用 `getAchievement`，未解锁才 `setAchievement` + `storeStats`；非 Steam 渠道只记录状态。

## 2. 协议

新增消息与错误码已写入 `protocol/schema/v1/` 并生成两端代码：

| 方向 | type | 载荷 |
| --- | --- | --- |
| C→S | `AuthRequest` | `{steamTicket, displayName?}`（第三分支） |
| S→C | `AccountView` | `{playerId, playerCode, displayName, steamBound, phoneBound, maskedPhone?}` |
| C→S | `RequestAccount` / `RequestPhoneCode` / `BindPhone` | 账号刷新与绑定 |
| S→C | `PhoneCodeSent` | `{expiresAt}` |
| C→S | `CreateRoomInvite` / `RedeemRoomInvite` | 房间邀请 token |
| S→C | `RoomInviteCode` | `{token, roomCode, expiresAt}` |
| S→C | `AchievementState` / `AchievementUnlocked` | 成就 id 与 Steam API 名 |

新增错误码：`STEAM_AUTH_FAILED`、`STEAM_UNAVAILABLE`、`STEAM_DISABLED`、`PHONE_INVALID`、`PHONE_ALREADY_BOUND`、`PHONE_CODE_UNAVAILABLE`、`CODE_INVALID`、`CODE_EXPIRED`、`ALREADY_IN_ROOM`。

认证成功后下行顺序为 `AuthResult` → `AccountView` → `AchievementState`（账号已解锁的成就 id 列表，按 id 升序），随后推送在线状态；旧客户端按未知类型忽略。

## 3. 服务端

| 位置 | 职责 |
| --- | --- |
| `server/crates/room-service/src/steam.rs` | `SteamVerifier`：`Http` / `Disabled` / `Stub`；调用 `ISteamUserAuth/AuthenticateUserTicket`，超时 3 秒，不重试 |
| `server/crates/room-service/src/phone_code.rs` | 绑定验证码：5 分钟过期、按账号 60 秒冷却；开发验证码固定 `123456` |
| `server/crates/room-service/src/account.rs` | 手机号 / Steam 双身份登录、绑定与 `AccountView`；展示名截断 20 字符 |
| `server/crates/room-service/src/room.rs` | 房间内一次性邀请 token（60 秒）；不落库 |
| `server/crates/room-service/src/net.rs` | 认证分支、账号、验证码、绑定、邀请 token 与成就下发 |
| `server/migrations/0001_baseline.sql` | `phone` 可空、新增 `steam_id`；两者各自唯一，至少一个非空 |

- 验票条件：`response.params.result == "OK"` 且 `steamid` 非空；`ownersteamid` 不参与建号。票据先做十六进制与长度校验，非法不发外部请求。
- 配置：`STEAM_WEB_API_KEY` 与 `STEAM_APP_ID` 同时存在才启用验票；任一缺失为 `STEAM_DISABLED`，手机号登录不受影响。日志不记录票据、publisher key、验证码与完整手机号。
- 绑定：只允许 Steam 账号绑定一次；同号重复请求幂等，占用他人手机号返回 `PHONE_ALREADY_BOUND`；事务内 `SELECT ... FOR UPDATE` 后检查并更新。
- 邀请：`CreateRoomInvite` 要求已入房且处于大厅；`RedeemRoomInvite` 复用 `JoinRoom` 的阶段、容量与 `RoomUpdate` 路径；已在房间返回 `ALREADY_IN_ROOM`。
- 成就：`server/data/v0.2/achievements.json` 由 `scripts/gen_achievement_data.py` 从成就实体文档生成，`AchievementCatalog` 建立 `achievement_id → steam_api_name` 映射；认证/重连下发账号已解锁状态，局内首次解锁下发 `AchievementUnlocked`，账号级记录存于 `account_achievement` 表，同一账号只同步一次。

## 4. 构建与发布

| 命令 | 行为 |
| --- | --- |
| `make client-export PLATFORM=steam SERVER_URL=wss://… STEAM_APP_ID=…` | 临时写入发布配置后导出 `output/steam/Civilization.app`，校验 `libsteam_api.dylib` 同时含 `x86_64` 与 `arm64`，退出时恢复空配置 |
| `make client-notarize CODESIGN_IDENTITY=… NOTARY_PROFILE=…` | 先签 `Contents/Frameworks/*.dylib`，再以 hardened runtime 与 `deploy/steam/entitlements.plist` 签 `.app`；验证后提交公证、staple 并用 `spctl` 复核 |
| `make steam-upload STEAM_APP_ID=… STEAM_DEPOT_ID=… STEAM_USER=…` | 要求产物已签名；渲染 `deploy/steam/*.vdf` 到输出目录后调用 `steamcmd`，密码交互输入 |

- 签名启用 `com.apple.security.cs.disable-library-validation`，允许 Steam 客户端注入覆盖层。
- `.gitignore` 忽略开发用 `steam_appid.txt`；depot 配置同时排除该文件。
- Helm：`networkPolicy.enabled` 默认关闭；打开后允许 DNS 与 443 出网（`networkPolicy.steamEgress`），域名限制由集群 DNS / 出网防火墙承担。`STEAM_WEB_API_KEY` 与 `STEAM_APP_ID` 由现有 `existingSecretName` 以 `envFrom` 注入。

## 5. 验证

| 命令 | 结果 |
| --- | --- |
| `make data-check` | 退出 0；成就数据无漂移 |
| `make test` | 退出 0；含 `steam_auth` 13 项、成就映射 3 项单测 |
| `make client-check` | 退出 0 |
| `make client-smoke` | 17 个场景全部 `UI_SMOKE_OK`，无 `SCRIPT ERROR` |
| `make client-export-steam`（测试参数） | 产物生成，`libsteam_api.dylib` 为 `x86_64 arm64` |
| `make client-notarize` / `make steam-upload`（无证书/未签名） | 分别在缺证书与未签名处给出明确阻塞提示 |

## 6. 未完成与外部依赖

真正接入 Steam 时的后台、证书、上传与验收步骤见 [Steam 上线操作清单](../../versions/v0.2/technical/steam-onboarding.md)。

- 6 个局内成就的运行时（触发、奖励、下发与账号级解锁）已实现；真实 Steam 同步仍待 Steamworks 后台与票据配置。本版不做成就弹窗、成就列表或结算页展示。
- 真实 Steam 票据校验需要 Steamworks 后台 App ID 与 Web API key；CI 不访问 Steam。
- Apple Developer ID、公证凭据、Steamworks depot 与商店页未配置；签名、公证与上传脚本已就绪但未实测。
- 生产短信通道未接入，绑定验证码在生产不可用。
