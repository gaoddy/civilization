# 客户端技术基线增量

> 状态：方案未实施
> 依赖：[v0.1 客户端技术基线](../../../../current/technical/architecture/client-stack.md)、[联网与技术架构](../../../../current/technical/architecture/network-architecture.md)

本文记录 v0.2 客户端相对 v0.1 的增量：Android 端适配与验收；不重复 v0.1 已确定的引擎、语言、协议与 iOS 适配。

## 1. 目标平台

| 项 | 选择 | 说明 |
| --- | --- | --- |
| 桌面端 | 沿用 v0.1 | 不改动选型与运行边界 |
| Android 端 | 同一份 Godot 4 工程与协议 | 不维护第二套界面；复用 iOS 与桌面端已实现的页面、缩放与触控规则 |

## 2. 布局适配

- 桌面端、iOS 与 Android 共用同一份工程；固定横屏，不提供竖屏布局，旋转设备不重排页面。
- Android 沿用 iOS 已定的 960×540 手机设计基准、16:9 内容框、设备档位与安全区规则；目标设备、系统返回键与键盘处理见[待确认事项](../../open-questions.md)。
- 规则只在服务端运行、客户端不做本地预演的约束不变。

## 3. 验收

- Android 端可完成完整对局；
- Android 与桌面端、iOS 在会战与对局控制流上行为一致，差异只允许存在于布局与输入。

## 相关文档

- [v0.1 客户端技术基线](../../../../current/technical/architecture/client-stack.md)
- [待确认事项](../../open-questions.md)
