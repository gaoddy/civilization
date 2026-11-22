# 部署与接入

> 状态：清单与发布入口已实现；集群首次发布尚未执行（见文末「待执行」）。
> 关联：[服务端技术基线](../architecture/server-stack.md)、[房间服务与联网](../room-service/README.md)。

本文记录 `room-service` 接入 platform 共享 Kubernetes 环境的现状：环境、镜像、配置、发布命令、客户端连接方式与排障。

## 1. 环境

| 项 | dev | prod |
| --- | --- | --- |
| namespace | `civilization-dev` | `civilization` |
| hostname | `civilization.dev.platform.internal` | `civilization.platform.internal` |
| 数据库库名 / 角色 | `civilization` / `civilization` | 同 dev |
| 数据库入口 | `pgbouncer:6432` | 写 `pgbouncer:6432`；读入口 `pgbouncer-read:6432` 可选 |
| 配额档位 | `small` | `small` |

namespace、ResourceQuota、LimitRange、默认拒绝 NetworkPolicy、DNS、Gateway 入站与共享服务出站由 platform 按 `deploy/projects/civilization.yaml` 登记表生成；本仓库不重复声明。

## 2. 镜像

- `server/Dockerfile` 多阶段构建；构建上下文为**仓库根**（`protocol` crate 引用根 `protocol/generated`）。
- 固定按 `linux/amd64` 构建，并导入目标节点 containerd `k8s.io`：

```bash
make deploy-image ENV=dev
make deploy-load ENV=dev SERVER=<ssh-target>
```

- chart 使用 `imagePullPolicy: Never`，tag 固定 `dev` / `prod`。重新导入镜像后必须执行 `kubectl rollout restart`（`make deploy-rollout` 已包含），否则 Pod 模板不变、旧进程继续运行。

## 3. 配置与 Secret

- 非敏感配置只有监听地址：ConfigMap `civilization-server-config` 的 `listen-addr`，以 `ROOM_SERVICE_LISTEN` 注入。
- 数据库凭据全部来自平台生成的 Secret `civilization-postgres-civilization`（键 `username`、`password`、`DATABASE_URL`；prod 另有 `DATABASE_READ_URL`）。chart 只引用、不创建。
- Steam 渠道另从同一 Secret 注入 `STEAM_WEB_API_KEY` 与 `STEAM_APP_ID`；缺任一项时服务端降级为 `STEAM_DISABLED`。
- 集群需要放行 `api.steampowered.com:443`；chart 的 `networkPolicy.enabled` 默认关闭，打开后按端口放行 443 与 DNS。详见 [Steam 接入](../steam-integration.md) §4。
- 仓库与 chart 内不出现密码、token 或运行时 Secret 文件。

## 4. 发布

首次发布前先由 platform 侧完成项目登记与 namespace、数据库：

```bash
# platform 仓库
make ensure-project-namespace ENV=<dev|prod> PROJECT=civilization SERVER=<ssh-target>
make ensure-project-database ENV=<dev|prod> PROJECT=civilization SERVER=<ssh-target>
make validate-project ENV=<dev|prod> PROJECT=civilization ONLINE=1
```

Civilization 仓库：

```bash
make deploy-lint                 # Helm lint + 两环境渲染
make deploy ENV=dev SERVER=<ssh-target>
```

`make deploy` 依次执行：构建镜像 → 导入节点 → server-side dry-run → apply → `rollout restart` 与等待 → 通过 Gateway 请求 `/healthz`、`/readyz`。

单独步骤：`deploy-render`、`deploy-preflight`、`deploy-apply`、`deploy-rollout`、`deploy-status`、`deploy-smoke`。

`ENV` 只接受 `dev` 或 `prod`，写成其他值会直接报错，不会回落到 dev 的 namespace 与域名。`deploy-render` 的 stdout 只有清单，可直接管道给 `kubectl apply -f -`。

prod 首次新增 hostname 时，platform 侧还需要同步 satori nginx 转发：`make sync-satori-prod-nginx SERVER=<ssh-target>`。

## 5. 客户端

大厅服务端地址填：

```text
wss://civilization.dev.platform.internal/ws
```

dev 的 Gateway 证书不是公共 CA 签发，且 `*.platform.internal` 不在公共 DNS 中：

- 开发者机器用 hosts 把 `civilization.dev.platform.internal` 指向 Gateway 节点（dev 为 `192.168.31.5`）。
- 打开客户端项目设置 `network/allow_insecure_tls`（`client/project.godot`，默认 `false`）。生产构建不得打开。

## 6. 运行约束

- 单实例：房间状态与连接登记都在进程内；`replicaCount: 1`，Deployment 使用 `Recreate`。
- 迁移随进程启动执行，经 `pgbouncer`。sqlx 已关闭语句缓存（事务池兼容）与迁移锁（单实例 + `Recreate` 保证不并发迁移）。扩展到多副本前必须改为独立迁移 Job。
- Gateway 对 WebSocket 的空闲超时未单独配置；客户端有 3 秒自动重连与重新认证兜底。

## 7. 排障

| 现象 | 先查什么 | 处理 |
| --- | --- | --- |
| `/readyz` 503 | 数据库 Secret 是否存在、PgBouncer 可达、项目角色权限 | 核对 `civilization-postgres-civilization` 与 NetworkPolicy；重新执行 `ensure-project-database` |
| Pod `ImagePullBackOff` | 镜像是否导入目标节点 | 在目标机 `ctr -n k8s.io images ls` 确认；重新 `make deploy-load` |
| 重新导入镜像后行为未变 | 是否执行 rollout restart | `make deploy-rollout` |
| 域名 404 / 502 | HTTPRoute 是否 Accepted、Service 端口、Pod 就绪 | `kubectl describe httproute civilization-server -n <namespace>`；核对 namespace 标签 |
| 客户端 TLS 报错 | hosts 与 `network/allow_insecure_tls` | 按第 5 节配置；生产不要打开 |
| 迁移失败 | 数据库是否可达、角色是否有 DDL 权限 | 查看 Pod 日志中的 `执行数据库迁移失败`；必要时先手工确认库与角色 |

## 8. 待执行

本仓库已交付清单与离线校验，以下动作需要目标集群与授权：

- platform：`ensure-project-namespace`、`ensure-project-database`、`validate-project ONLINE=1`。
- Civilization：`make deploy ENV=dev SERVER=<ssh-target>`；prod 另需 nginx 同步。
- 部署后验证：`/readyz` 通过、WebSocket 经 Gateway 可连接、空闲长连接表现。
