# Steamworks 后台与商店页

本文登记「Steamworks 后台与商店页」需求；不描述接口、表结构与实现方案。

## 编号、状态与实现版本

- 编号：R-PLAT-REL-005
- 类型与模块：平台与发行 / 发行与商店
- 状态：待确认
- 实现版本：待定
- 历史编号：R005、PLAT-05

## 背景与目标

Steam 渠道契约已实现，但 Steamworks 后台与商店页尚未建立；本需求完成发行配置。

## 需求规则与验收

- App ID 与 depot 配置
- 6 个成就的 API 名与图标
- 商店页、截图与前 4 张素材
- 5 秒预告
- 按 Steam 上线操作清单执行

验收：Steamworks 后台配置完整，商店页与素材可提交审核。

## 排除项

- 不包含 Windows depot（见 R-PLAT-REL-009）

## 待定夺事项

| 编号 | 事项 | 影响 | 提出日期 |
| --- | --- | --- | --- |
| Q001 | App ID、成就 API 名与图标、depot 的具体配置 | 影响 Steam 发布与成就同步 | 2026-11-25 |

## 定夺记录

无

## 关联文档

- [Steam 上线操作清单](../versions/v0.2/technical/steam-onboarding.md)
- [销量预测](../sales-forecast.md) §7.1
