# Android 的 godotsteam 扩展处理

本文登记 R003（原 PLAT-03）「Android 的 godotsteam 扩展处理」需求；不描述接口、表结构与实现方案。

## 状态与实现版本

- 状态：待确认
- 实现版本：v0.2

## 背景与目标

Steam 扩展声明了 Android / Linux / Windows 库路径，但仓库只有 macOS 二进制；Android 包内含 0 字节库并可能在启动时加载失败。

## 需求规则与验收

- Android 包不再加载不存在的 Steam 扩展库
- 启动日志无相关加载错误
- 不依赖 Steam 的业务功能不受影响

验收：Android 启动无 Steam 扩展加载错误；移除声明或平台排除的方案已选定并实施。

## 排除项

- 不移除 macOS 与 iOS 的 Steam 集成

## 待定夺事项

| 编号 | 事项 | 影响 | 提出日期 |
| --- | --- | --- | --- |
| Q001 | 移除非 macOS 扩展声明，还是按平台排除该扩展 | 影响 Android 包内容与后续 Steam 平台扩展 | 2026-11-25 |

## 定夺记录

无

## 关联文档

- [客户端技术基线](../versions/v0.2/technical/architecture/client-stack.md) §5
