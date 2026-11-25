# Steam 成就真实同步验收

本文登记 R006（原 PLAT-06）「Steam 成就真实同步验收」需求；不描述接口、表结构与实现方案。

## 状态与实现版本

- 状态：实施中
- 实现版本：v0.2

## 背景与目标

成就触发、奖励、账号级幂等与消息下发已验证；本需求用真实 Steamworks 票据完成端到端验收。

## 需求规则与验收

- 6 个成就在真实票据下端到端解锁
- 重复触发保持幂等
- 补同步路径可用

验收：真实 Steamworks 环境下 6 个成就解锁，重复与补同步不重复发奖。

## 排除项

- 不做成就以外 Steam 功能

## 待定夺事项

无

## 定夺记录

无

## 关联文档

- [发行渠道与账号](../current/product/accounts-and-distribution.md) §8
- [Steam 接入](../current/technical/steam-integration.md)
