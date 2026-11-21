# v0.1 验收记录

> 状态：已记录（M2 断线重连一致性；iOS 适配自动化与真机项均通过）
> 关联：M2 联机内测、iOS 移动端适配（破例纳入 v0.1）

本文记录 v0.1 的验收口径与自动化验收结果，不描述产品规则或实现方案。规则见[核心规则](product/rules/core-rules.md)，实现见[当前实现状态](../../current/README.md)。

## 1. 验收口径

- M2 不设真人完成率验收。断线重连后状态一致由自动化用例保证（见 §2）。
- M3 不做单独验收记录：桌面端完整对局、崩溃率、意图拒绝率与单局时长不记入本文，也不作为 v0.1 的退出条件。
- iOS 适配的验收形态为真机可安装的开发签名包，分自动化与真机两部分记录（见 §3）。

## 2. 断线重连一致性

自动化用例覆盖「重连快照与当前视图逐字段一致」与「战斗阶段重连补发本场 `CombatResult`」，不依赖真人操作：

```bash
cd server && cargo test -p room-service --test reconnect
```

| 日期 | 用例数 | 结果 | 备注 |
| --- | --- | --- | --- |
| 2026-11-20 | 11 | 全部通过 | `cargo test -p room-service --test reconnect`；同时由 `make m1-check` 的 `cargo test --workspace` 覆盖 |

## 3. iOS 移动端适配

适配规则见[移动端适配](../../current/product/ui/mobile-adaptation.md)，实现见[客户端技术现状](../../current/technical/client.md) §9。验收分自动化与真机两部分。

### 3.1 自动化（可复跑）

```bash
make client-check
make client-smoke
make m1b-e2e
```

| 日期 | 项 | 结果 | 备注 |
| --- | --- | --- | --- |
| 2026-11-20 | `make client-check` | 退出码 0 | 无脚本解析错误 |
| 2026-11-20 | `make client-smoke` | 13 个场景全部 `UI_SMOKE_OK` | 含 `ui_smoke_touch`、`ui_smoke_layout`、`ui_smoke_platform`；无 `SCRIPT ERROR` |
| 2026-11-20 | 设备基准与命中换算 | 通过 | 桌面 / iPhone（960×540）/ iPad（1280×720）三档断言：44pt→60/41、32pt→44/30；安全区与键盘像素换算 |
| 2026-11-20 | 小屏布局 | 通过 | 960×540 下六个页面关键节点不越界；图鉴目录、详情与 4 列网格 |
| 2026-11-20 | 触控交互 | 通过 | 长按 450ms 显示详情、移动取消、多指与阻塞态不触发；手牌悬停残留复位 |
| 2026-11-20 | `make m1b-e2e` | 退出码 0 | 客户端到服务端联调未受影响 |
| 2026-11-20 | 中文字体覆盖 | 通过 | `ui_smoke_text` 断言内置子集覆盖 `zh_CN.csv` 全部字符，默认主题字体为 `NotoSansSC-Regular-subset.ttf`；桌面渲染无缺字 |
| 2026-11-20 | 移动端默认地址 | 通过 | `ui_smoke_platform` 断言桌面 `ws://127.0.0.1:8080/ws`、移动端 `ws://cooter.local:8080/ws` |
| 2026-11-20 | iOS 工程编译 | `BUILD SUCCEEDED`（`CODE_SIGNING_ALLOWED=NO`） | 导出 Xcode 工程可编译；正式签名包见 §3.2 |
| 2026-11-21 | 内容框 16:9 居中 | 通过 | `content_frame` 在 1280×800（16:10）与 1280×960（4:3）下按 contain 居中：4:3 宽度贴满、上下各留 12.5%，16:10 上下各留 5%；六个页面内容不越界、对局手牌贴内容框底；背景铺满视口 |
| 2026-11-21 | `make client-check` / `make client-smoke` | 退出码 0 / 13 个场景全部 `UI_SMOKE_OK` | 本次窗口策略改动后复跑，无 `SCRIPT ERROR` |
| 2026-11-21 | 桌面 16:10 实机 | 通过 | `make client-run` 启动为 16:9；拖成 16:10 后内容上下各露出约 5% 背景、完整居中，登录页背景铺满窗口 |
| 2026-11-21 | 窗口自由缩放 | 通过 | `ui_smoke_platform` 断言 `content_frame` 在更宽/更窄/正好 16:9 下的框位置与尺寸、`launch_window_size` 1512×982→1360×765；拖动后不再回弹 |
| 2026-11-21 | 非 16:9 覆盖层对齐 | 通过 | `ui_smoke_layout` 断言超宽视口下瞄准层源矩形落在内容框局部坐标、详情层与战报按 `ContentRoot` 缩放定位；真机复查通过 |
| 2026-11-21 | 安全区背景铺满 | 通过 | `ui_smoke_layout` 断言六个页面 `PageBackground` 留在安全区外、安全区内直接子节点数量正确；iPhone 真机复看通过（左右背景带按 16:9 内容框保留） |
| 2026-11-21 | 移动端主机名连接 | 通过 | `Net.prefer_ipv4_url` 把主机名解析为 IPv4 后再连接；`ui_smoke_platform` 覆盖主机名替换、解析失败保留、IP 字面量与 IPv6 字面量；真实解析 `ws://cooter.local:8080/ws` 可连；iPad 重装后真机复看通过 |
| 2026-11-21 | 弹出层公共外壳 | 自动化通过 | `ui_smoke_side` 断言 `PopupLayer` 的内容替换、居中、锚点夹紧与底边贴边；邀请、好友、选择、战斗开场与图鉴分类面板的既有场景全部通过 |
| 2026-11-21 | 非 16:9 窗口背景铺满 | 自动化与截图验证通过 | `stretch/aspect` 由 `keep_width` 改为 `expand`：1428×440、960×800、1280×720 三档逻辑视口均铺满窗口，内容框居中，框外露出 `PageBackground` 纹理而非引擎黑边；`ui_smoke_platform` 断言同步为 `expand` |

### 3.2 真机项

使用 `Apple Development` 证书签名的开发包，在 iPhone 17 Pro 与 iPad Air 11-inch 上执行：

| 项 | 判定 | 结果 |
| --- | --- | --- |
| `make client-export-ios` 产出签名 `.ipa` | 命令退出码 0，`output/ios/Civilization-ios.ipa` 可安装 | ✅ 2026-11-21：`BUILD SUCCEEDED`，`output/ios/Civilization-ios.ipa` 生成 |
| iPhone 真机安装并启动 | 横屏进入登录页，无闪退 | ✅ 2026-11-21 |
| 中文显示 | 登录页、大厅、对局与图鉴中文全部可读，无缺字或方块 | ✅ 2026-11-21 |
| iPad 真机安装并启动 | 1280×720 基准，图鉴 5 列 | ✅ 2026-11-21 |
| 全触控完成一局 | 8 人房间招募、战斗、结算走完，不依赖鼠标 | ✅ 2026-11-21 |
| 安全区实测 | 刘海/灵动岛与 Home 指示条不遮挡按钮与文本 | ✅ 2026-11-21 |
| 左横与右横 | 两个方向均可操作 | ✅ 2026-11-21 |

## 相关文档

- [v0.1 版本入口](README.md)
- [未确认事项](open-questions.md)
- [房间服务与联网](../../current/technical/room-service/README.md)
