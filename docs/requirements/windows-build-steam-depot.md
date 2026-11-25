# Windows 构建与 Steam depot

本文登记 R009（原 PLAT-09）「Windows 构建与 Steam depot」需求；不描述接口、表结构与实现方案。

## 状态与实现版本

- 状态：待确认
- 实现版本：待定

## 背景与目标

客户端当前只有 macOS、iOS、Steam(macOS)、macOS Local Debug、Android 与 Android AAB 导出预设，没有 Windows。

## 需求规则与验收

- Windows 导出预设可用
- 产物进入 Steam depot
- Steam 成就与云存档路径在 Windows 生效

验收：Windows 产物可导出、安装并通过 Steam 启动，成就与云存档可用。

## 排除项

- 不在未立项前承诺 Windows 版本

## 待定夺事项

| 编号 | 事项 | 影响 | 提出日期 |
| --- | --- | --- | --- |
| Q001 | Windows 构建是否立项，以及与 Steam 商店页的排期关系 | 影响 Steam 首发平台覆盖面 | 2026-11-25 |

## 定夺记录

无

## 关联文档

- [销量预测](../sales-forecast.md) §7.1（列为 P0）
