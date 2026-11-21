# 客户端技术基线

> 状态：已实现（v0.1 M1b）
> 依赖：[设计总纲](../../product/overview/design-overview.md)（§8）、[联网与技术架构](network-architecture.md)
> 实现现状：[Godot 客户端](../../../../current/technical/client.md)

本文记录 v0.1 客户端已确定的技术选型与运行边界；不描述服务端选型、页面布局与具体实现。未确定事项见[待确认事项](../../open-questions.md)。

## 1. 选型

| 项 | 选择 | 说明 |
| --- | --- | --- |
| 引擎 | Godot 4 | 补丁版本固定 4.7.2，本机引擎不符时 make 直接失败 |
| 脚本语言 | GDScript | 不使用 C# |
| 目标平台 | 桌面端 | 首版只做桌面端 |
| 网络 | WebSocket + JSON | 事件信封与消息清单见[联网与技术架构](network-architecture.md)第 5 节 |

## 2. 运行边界

- 客户端不加载、不执行规则引擎，不做本地预演；状态变化只来自服务端事件，见[联网与技术架构](network-architecture.md)硬约束第 2、3 条。
- 客户端不自行推导「是否买得起」「能否三连」等规则结论，以服务端接受或拒绝为准；可操作提示只读 `PlayerView.affordances`。点击入口另按视图阶段、`self.recruited` 与本人存活状态做与服务端一致的语义预检，挡住阶段切换瞬间仍可点的按钮，不推导任何规则结论。
- 战斗只播放服务端下发的 `CombatResult`，播放速度不影响结算。
- 协议常量由 `protocol/schema/v1/` 生成，仓库根产物为 `protocol/generated/protocol.gd`，客户端产物为 `client/generated/protocol.gd`，两处由 `make protocol` 同批输出、`make protocol-check` 检测漂移；客户端不做完整运行时 schema 校验，只检查 `type`、`v` 与关键字段，字段级契约由服务端与样例测试保证。规则见[通信协议](protocol.md) §8。

## 3. 界面布局

- 桌面端界面按横屏布局设计，使用 Control 节点与容器适配窗口尺寸。
- 断点、目标设备与安全区处理由 UI 设计确定。

## 4. 实现约定

- 工程目录：`client/`，Godot 4.7.2，GDScript；主场景 `client/scenes/login.tscn`。
- 测试：`make client-smoke` 无头实例化五个场景；`tests/test_client_boundary.py` 断言客户端不引用规则实现；`make m1b-e2e` 用 1 个无头客户端 + 7 个 `sim bot` 打完一局。
- 构建与导出：本机运行与 macOS 桌面端导出均已落地，命令与产物见[Godot 客户端](../../../../current/technical/client.md) §4 与 §8。
- 未确定事项见[待确认事项](../../open-questions.md)。

## 相关文档

- [Godot 客户端](../../../../current/technical/client.md)
- [服务端技术基线](server-stack.md)
- [联网与技术架构](network-architecture.md)
- [设计总纲](../../product/overview/design-overview.md)
- [待确认事项](../../open-questions.md)
