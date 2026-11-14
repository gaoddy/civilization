# 客户端技术基线

> 状态：方案未实施
> 依赖：[设计总纲](../../product/overview/design-overview.md)（§8）、[联网与技术架构](network-architecture.md)

本文记录 v0.1 客户端已确定的技术选型与运行边界；不描述服务端选型、页面布局与具体实现。未确定事项见[待确认事项](../../open-questions.md)。

## 1. 选型

| 项 | 选择 | 说明 |
| --- | --- | --- |
| 引擎 | Godot 4 | 小版本在工程初始化时锁定 |
| 脚本语言 | GDScript | 不使用 C# |
| 目标平台 | 桌面端 | 首版只做桌面端 |
| 网络 | WebSocket + JSON | 事件信封与消息清单见[联网与技术架构](network-architecture.md)第 5 节 |

## 2. 运行边界

- 客户端不加载、不执行规则引擎，不做本地预演；状态变化只来自服务端事件，见[联网与技术架构](network-architecture.md)硬约束第 2、3 条。
- 客户端不自行推导「是否买得起」「能否三连」等规则结论，以服务端接受或拒绝为准；可操作提示只读 `PlayerView.affordances`。
- 战斗只播放服务端下发的 `CombatResult`，播放速度不影响结算。
- 协议常量由 `protocol/schema/v1/` 生成到 `protocol/generated/protocol.gd`，工程建立后由 `make protocol` 输出到客户端工程；客户端不做完整运行时 schema 校验，只检查 `type`、`v` 与关键字段，字段级契约由服务端与样例测试保证。规则见[通信协议](protocol.md) §8。

## 3. 界面布局

- 桌面端界面按横屏布局设计，使用 Control 节点与容器适配窗口尺寸。
- 断点、目标设备与安全区处理由 UI 设计确定。

## 4. 未确定事项

- 工程目录、构建与导出流程、客户端测试框架：搭建工程时确定，见[待确认事项](../../open-questions.md)。
- 服务端技术：见[服务端技术基线](server-stack.md)。

## 相关文档

- [联网与技术架构](network-architecture.md)
- [设计总纲](../../product/overview/design-overview.md)
- [待确认事项](../../open-questions.md)
