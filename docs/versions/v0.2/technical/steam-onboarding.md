# Steam 上线操作清单

> 状态：待执行（Steamworks、Apple 证书与生产环境就绪后按本文操作）
> 关联：[Steam 接入技术现状](../../../current/technical/steam-integration.md)、[发行渠道与账号](../../../current/product/accounts-and-distribution.md)、[R-PLAT-REL-005 Steamworks 后台与商店页](../../../requirements/steamworks-store.md)

代码侧的 Steam 契约与适配已完成。本文是真正接入 Steam 时的一次性操作清单：外部后台配置、服务端配置、客户端构建签名、SteamPipe 上传与端到端验收。各命令的实际行为见[Steam 接入技术现状](../../../current/technical/steam-integration.md) §4。

## 1. 一次性准备

### 1.1 Steamworks

需要 Steamworks 合作伙伴账号与已支付 App Credit 的应用。

| 项 | 要求 |
| --- | --- |
| App ID | 自有应用 ID；客户端、服务端与 Steamworks 三处必须一致 |
| Depot ID | macOS 内容 depot；与 App ID 不同，单独记录 |
| Publisher Web API Key | 服务端验票使用；只放 Helm Secret，不入库、不入日志 |
| 成就 | 后台逐个创建 6 个成就，API 名与下表完全一致（区分大小写） |
| 启动项 | macOS 启动项指向 `Civilization.app/Contents/MacOS/Civilization` |
| 安装目录 | depot 解包后 `Civilization.app` 位于安装根目录 |
| 商店页与定价 | 本仓库不涉及；上线前单独完成 |

6 个成就的 API 名以 `server/data/v0.2/achievements.json` 为准：

| 成就 id | 中文名 | Steam API 名 |
| --- | --- | --- |
| `biological-evolution` | 生物进化 | `BIOLOGICAL_EVOLUTION` |
| `education-for-all` | 有教无类 | `EDUCATION_FOR_ALL` |
| `genetics` | 遗传学 | `GENETICS` |
| `machines-roar` | 机器轰鸣 | `MACHINES_ROAR` |
| `one-small-step` | 人类一小步 | `ONE_SMALL_STEP` |
| `turing-moment` | 图灵时刻 | `TURING_MOMENT` |

修改成就 API 名时，先改 `docs/versions/v0.2/product/content/achievements/*.md` 的 `steam_api_name`，再执行 `make data` 与 `make data-check`，确保文档、生成数据与 Steamworks 后台一致。

### 1.2 Apple 签名与公证

| 项 | 要求 |
| --- | --- |
| 证书 | `Developer ID Application`；导出 `.p12` 并安装到钥匙串 |
| Team ID | 与证书主体一致 |
| 公证凭据 | Apple ID + App 专用密码 + Team ID，用 `notarytool store-credentials` 存入本机钥匙串 |
| 公证配置档名 | 下文以 `civilization-notary` 为例 |

```bash
xcrun notarytool store-credentials "civilization-notary" \
  --apple-id "<Apple ID>" \
  --team-id "<Team ID>" \
  --password "<App 专用密码>"
```

凭据只存在本机钥匙串；不要把密码、`.p12` 或 `STEAM_WEB_API_KEY` 写入仓库。

### 1.3 服务端环境

- 服务端可出网访问 `api.steampowered.com:443`。
- Helm Secret 增加两个键：`STEAM_WEB_API_KEY`、`STEAM_APP_ID`；chart 通过 `existingSecretName` 以 `envFrom` 注入。
- 若集群默认拒绝出网，打开 chart 的网络策略：`networkPolicy.enabled=true`（默认 `steamEgress=true`，放行 DNS 与 443）。

## 2. 配置对照

| 值 | 落点 | 校验方式 |
| --- | --- | --- |
| App ID | Steamworks、客户端 `STEAM_APP_ID`、服务端 `STEAM_APP_ID` | 三处完全一致；不一致时验票失败 |
| Depot ID | `make steam-upload STEAM_DEPOT_ID=...` | SteamPipe 构建日志 |
| Publisher Key | Helm Secret `STEAM_WEB_API_KEY` | 登录成功即说明验票可用 |
| 服务端地址 | `make client-export PLATFORM=steam SERVER_URL=wss://...` | 必须是 `wss://`，否则导出直接失败 |
| 公证配置档 | `make client-notarize NOTARY_PROFILE=...` | 公证提交成功 |
| 签名主体 | `make client-notarize CODESIGN_IDENTITY=...` | `codesign --verify --deep --strict` 通过 |

## 3. 客户端构建、签名与公证

在仓库根目录执行：

```bash
make client-export PLATFORM=steam \
  SERVER_URL=wss://<正式域名>/ws \
  STEAM_APP_ID=<App ID>
```

通过标准：
- 命令退出 0，产出 `output/client/Civilization-steam-<构建号>.app`；
- 日志显示 `libsteam_api.dylib` 同时包含 `x86_64` 与 `arm64`；
- `client/generated/build_config.gd` 在命令结束后恢复为空配置（发布配置已嵌入产物）。

随后签名与公证：

```bash
make client-notarize \
  CODESIGN_IDENTITY="Developer ID Application: <名称> (<TEAMID>)" \
  NOTARY_PROFILE=civilization-notary
```

通过标准：
- 嵌套 dylib 与 `.app` 均签名成功；
- `codesign --verify --deep --strict` 通过；
- `notarytool submit --wait` 返回 `Accepted`；
- `stapler validate` 与 `spctl -a -vv -t exec` 通过。

注意：公证会修改 `.app` 内容。源码与渠道参数未变时重新执行 `make client-export PLATFORM=steam` 会命中戳记并复用已签名产物；一旦源码或参数变化就会重建并覆盖签名，需要重新签名与公证。

本地调试（可选）：把 `steam_appid.txt` 放到 `Civilization-steam-<构建号>.app/Contents/MacOS/` 下可直接运行未由 Steam 启动的包；该文件已被 `.gitignore` 与 depot 配置排除，不得进入发布包。

## 4. SteamPipe 上传

首次上传前，在 Steamworks 后台确认 depot 已创建并挂到 App 下。

```bash
make steam-upload \
  STEAM_APP_ID=<App ID> \
  STEAM_DEPOT_ID=<Depot ID> \
  STEAM_USER=<Steam 账号>
```

- `steamcmd` 会交互询问密码与 Steam Guard，密码不写入仓库或命令行。
- 上传要求 `output/client/Civilization-steam-<构建号>.app` 已签名；未签名时命令会在构建前直接失败并提示先执行 `make client-notarize`。
- `deploy/steam/app_build.vdf` 的 `setlive` 为空，表示只上传构建、不自动切分支。需要上线时，在 Steamworks 后台把目标构建设为 `default` 分支，或在渲染后的 `output/client/app_build.vdf` 中填入分支名后重跑。
- 首次上线建议先发到 beta 分支，用两个 Steam 账号完成第 6 节验收后再切 `default`。

## 5. 服务端发布与验票

1. 在 Helm Secret 写入 `STEAM_WEB_API_KEY` 与 `STEAM_APP_ID`。
2. 按现有流程发布服务端（`make deploy ENV=<dev|prod> SERVER=<ssh-target>`）。
3. 发布后用 Steam 构建登录一次；能进入大厅即表示验票链路可用。
4. 回滚只需移除这两个键：服务端降级为 `STEAM_DISABLED`，手机号登录不受影响。

## 6. 端到端验收

用两个真实 Steam 账号，从 Steam 客户端启动：

1. 首次启动自动登录并建号；再次启动进入同一账号与同一玩家编号。
2. 展示名等于 Steam 个人资料名，超过 20 字符被截断；空名时为「玩家N」。
3. 大厅出现「账号」入口；绑定手机号成功，账号页显示掩码号码。
4. 用同一手机号在非 Steam 构建登录，进入同一账号，好友与房间数据一致。
5. 两个 Steam 账号在同一房间验证「邀请 Steam 好友」：被邀请者接受后进入房间；满员、已开局、已解散、超时与已在其他房间分别提示。
6. Steam 离线或未登录时启动，登录页显示原因与重试；恢复后重试成功。
7. 成就运行时落地后补验：6 个成就解锁时 Steam 成就同步，重复解锁不重复同步。

## 7. 常见阻塞

| 现象 | 原因 | 处理 |
| --- | --- | --- |
| 登录返回 `STEAM_DISABLED` | 服务端缺 `STEAM_WEB_API_KEY` 或 `STEAM_APP_ID`，或值非法 | 检查 Secret 注入与数值格式 |
| 登录返回 `STEAM_AUTH_FAILED` | App ID 不一致、票据无效或票据过期 | 核对三处 App ID；重新从 Steam 启动 |
| 登录返回 `STEAM_UNAVAILABLE` | 服务端访问 `api.steampowered.com` 失败 | 检查出网与第 1.3 节网络策略 |
| 客户端提示「请先登录 Steam 客户端」 | 未由 Steam 启动，或 Steam 未登录 | 从 Steam 库启动；确认 Steam 客户端在线 |
| 公证被拒 | 嵌套库未先签、缺少 entitlements 或 hardened runtime 配置不符 | 按 `notarytool` 日志补签，不要手工跳步 |
| `steam-upload` 提示产物未签名 | 导出后未公证，或重新导出覆盖了签名 | 重新执行 `make client-notarize` |
| Steam 好友邀请打不开 | 大厅创建失败或 Steam 覆盖层不可用 | 确认入口未置灰；从 Steam 启动并启用覆盖层 |
| 成就不同步 | Steamworks 成就 API 名与数据不一致 | 对照第 1.1 节表格与 `make data-check` |

## 8. 当前实现边界

- 6 个局内成就的运行时已实现；真实 Steam 同步仍待 Steamworks 后台成就配置与票据验证。
- 产品规则要求「未通过 Steam 启动时提示并退出」；当前实现停在登录页，提供「重试」与「退出」，不自动结束进程。
- 生产短信通道未接入；手机号绑定目前只有开发固定验证码可用。
