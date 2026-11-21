# 发行渠道与账号技术基线

> 状态：已实现（Steam 契约与适配；成就运行时除外）
> 依赖：[v0.1 客户端技术基线](../../../v0.1/technical/architecture/client-stack.md)、[v0.1 通信协议](../../../v0.1/technical/architecture/protocol.md)、[v0.2 客户端技术基线](client-stack.md)、[Steam 接入技术现状](../../../../current/technical/steam-integration.md)

本文记录 v0.2 Steam 发行与账号能力的技术目标形态：构建渠道、认证分支、账号模型、邀请 token 与成就同步。实现细节、命令与验证结果见[Steam 接入技术现状](../../../../current/technical/steam-integration.md)；产品规则见[发行渠道与账号](../../product/systems/accounts-and-distribution.md)。

## 1. 客户端

- 新增 `Platform` autoload，作为唯一接触 Steamworks 的边界；`DevBackend` 与 `SteamBackend` 两个实现，业务层只调用语义接口。
- 新增 macOS `Steam` 导出预设（`custom_features="steam"`），与现有 macOS 直发、iOS 构建共用同一工程与协议；Steam 构建只从 Steam 启动。
- 登录：Steam 渠道自动取 Web API 票据并发送 `AuthRequest{steamTicket}`；展示名取 Steam 个人资料名并截断 20 字符；失败只重试，不回落手机号表单。
- 账号页：展示展示名、玩家编号与手机号绑定状态；绑定走验证码流程；入口只在 Steam 构建的大厅出现。
- Steam 好友邀请：Steam 大厅只作为邀请传输，房间成员、座位、阶段与容量仍以服务端为准；邀请 token 60 秒、一次性。
- 成就同步：服务端下发成就 id 与 Steam API 名，客户端幂等调用 Steam 统计接口；不硬编码映射，不建立本地待同步文件。

## 2. 服务端

- 新增 `SteamVerifier`（枚举 `Http` / `Disabled` / `Stub`），`Http` 调用 `ISteamUserAuth/AuthenticateUserTicket`，超时 3 秒，不重试；票据与 publisher key 不落日志。
- 账号表从「手机号唯一」扩展为「手机号 / Steam 双身份」：`phone` 与 `steam_id` 均可空且各自唯一，至少一个非空；开发阶段直接改基线迁移。
- `AccountStore` 增加按 Steam 与账号 id 的索引；Steam 首次登录建号，手机号绑定只允许 Steam 账号绑定一次。
- 新增绑定验证码服务：5 分钟过期、按账号 60 秒重发冷却；生产短信通道未接入，见[待确认事项](../../open-questions.md)。
- 新增房间邀请 token：只存在于房间进程内，60 秒过期、一次性消费，不写 `room_invite` 表；入房复用 `JoinRoom` 的阶段、容量与视图路径。
- 成就：加载 `server/data/v0.2/achievements.json` 的 `steam_api_name` 映射，认证/重连下发 `AchievementState`，解锁下发 `AchievementUnlocked`；成就判定仍由 v0.2 成就运行时负责。

## 3. 协议

- `AuthRequest` 增加第三分支 `{steamTicket, displayName?}`；旧两个分支不变。
- 新增：`AccountView`、`RequestAccount`、`RequestPhoneCode`、`PhoneCodeSent`、`BindPhone`、`CreateRoomInvite`、`RoomInviteCode`、`RedeemRoomInvite`、`AchievementState`、`AchievementUnlocked`。
- 新增错误码：`STEAM_AUTH_FAILED`、`STEAM_UNAVAILABLE`、`STEAM_DISABLED`、`PHONE_INVALID`、`PHONE_ALREADY_BOUND`、`PHONE_CODE_UNAVAILABLE`、`CODE_INVALID`、`CODE_EXPIRED`、`ALREADY_IN_ROOM`。
- 开发阶段允许同版本增字段与增类型；样例、GDScript、Rust 与字段表同批生成。

## 4. 构建与发布

- macOS Steam 包先签 dylib 与嵌套框架、再签应用，启用 hardened runtime；公证与 stapler 由独立 make 入口完成。
- SteamPipe 上传使用 `steamcmd` 与入库的 depot/VDF；账号、App ID 密钥与 Steam Web API key 只从环境或 Helm Secret 读取。
- 客户端生产地址必须为 `wss://`，由部署层终止 TLS；非 Steam 构建保持现有手机号登录。

## 5. 未确认

- 国际短信通道、Steam 国际版域名与 TLS、Steamworks 后台配置、Apple 签名与公证账号，统一记录在[待确认事项](../../open-questions.md)。
