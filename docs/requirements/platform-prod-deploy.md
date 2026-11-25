# prod 服务端上线

本文登记 R001（原 PLAT-01）「prod 服务端上线」需求；不描述接口、表结构与实现方案。

## 状态与实现版本

- 状态：实施中
- 实现版本：v0.2

## 背景与目标

prod 环境的部署入口与 dev 首发已执行，prod 尚未执行；本需求补齐 prod 上线所需的部署、配置与公网验证。

## 需求规则与验收

- prod 环境部署完成
- prod 域名与 TLS 证书可用
- 服务健康检查与公网联机可用
- 客户端经公网连接 prod 完成完整对局

验收：prod 部署通过，健康检查与公网 WebSocket 可用，客户端经公网完成一局。

## 排除项

- 不以 dev 环境替代 prod 验收

## 待定夺事项

| 编号 | 事项 | 影响 | 提出日期 |
| --- | --- | --- | --- |
| Q001 | prod 对外域名是否复用 `civilization.satori.host`，以及 TLS 证书签发方式 | 影响 prod 首次发布与客户端 `CLIENT_PROD_SERVER_URL` | 2026-11-25 |

## 定夺记录

无

## 关联文档

- [部署与接入](../current/technical/deployment/README.md) §8
- [版本路线与风险](../version-roadmap.md) M4
