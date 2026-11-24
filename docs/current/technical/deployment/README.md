# 部署与接入

> 状态：清单与发布入口已实现；dev 集群首次发布已执行（2026-11-23），prod 尚未执行（见文末「发布记录与待执行」）。
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
| 公网入口 | `civilization.satori.host` | 待定 |

namespace、ResourceQuota、LimitRange、默认拒绝 NetworkPolicy、DNS、Gateway 入站与共享服务出站由 platform 按 `deploy/projects/civilization.yaml` 登记表生成；本仓库不重复声明。

dev 的公网入口由 satori 宿主 nginx 终止 TLS，反代到 dev Gateway NodePort；站点配置源文件为 [deploy/nginx/civilization.satori.host.conf](../../../../deploy/nginx/civilization.satori.host.conf)，证书由 certbot 维护。

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

dev 节点没有 `*.platform.internal` 的 DNS，也不信任 dev 自签证书，`make deploy-smoke` 在 dev 会停在 curl 解析或证书校验；用 `curl --resolve civilization.dev.platform.internal:443:192.168.31.5 -k` 验证，见 §5。

`ENV` 只接受 `dev` 或 `prod`，写成其他值会直接报错，不会回落到 dev 的 namespace 与域名。`deploy-render` 的 stdout 只有清单，可直接管道给 `kubectl apply -f -`。

公网入口：satori 宿主 nginx 站点 `/etc/nginx/sites-available/civilization.satori.host` 由 [deploy/nginx/civilization.satori.host.conf](../../../../deploy/nginx/civilization.satori.host.conf) 安装，反代到 `https://10.66.66.4:32715`（dev Gateway NodePort），Host 与 SNI 改写为 `civilization.dev.platform.internal`。首次安装先只启用 80 段，用 `certbot certonly --webroot -w /var/www/letsencrypt -d civilization.satori.host` 签发证书，再启用 443 段。

prod 首次新增 hostname 时，platform 侧还需要同步 satori nginx 转发：`make sync-satori-prod-nginx SERVER=<ssh-target>`。

## 5. 客户端

服务端地址由客户端构建配置决定，登录页不再提供服务器输入。客户端构建环境分 `local`、`dev`、`prod` 三档，默认 `local`；参数写 `ENV=` 或 `env=` 均可。

- 开发构建（`make client-export ENV=dev`、`make client-export PLATFORM=ios ENV=dev`；环境默认 `local`，连 dev 入口必须显式传 `ENV=dev` 或 `env=dev`）内置 dev 公网入口：

```text
wss://civilization.satori.host/ws
```

- 正式构建用 `ENV=prod`，地址取 `CLIENT_PROD_SERVER_URL`；该值当前为空，配置前导出会直接失败，避免把开发地址带进正式包。

公网域名由 satori 宿主 nginx 终止 TLS 后反代到 dev Gateway，证书为公共 CA 签发，客户端不需要 hosts 或 `network/allow_insecure_tls`。

局域网直连或排障时用构建参数覆盖开发地址，例如 `make client-export ENV=dev CLIENT_DEV_SERVER_URL=wss://civilization.dev.platform.internal/ws`（dev 也接受 `ws://` 局域网地址）：把该域名指向 Gateway 节点（dev 为 `192.168.31.5`），并打开客户端项目设置 `network/allow_insecure_tls`（`client/project.godot`，默认 `false`）。生产构建不得打开。

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
| 客户端 TLS 报错 | 公网域名证书或内网 hosts 与 `network/allow_insecure_tls` | 公网域名检查 satori nginx；内网按第 5 节配置；生产不要打开 |
| 公网域名 502 | satori nginx 到 `10.66.66.4:32715` 是否可达、Gateway Pod 是否就绪 | 从 satori 执行 `curl -k https://10.66.66.4:32715/healthz`；检查 `kubectl -n civilization-dev get pods` |
| 迁移失败 | 数据库是否可达、角色是否有 DDL 权限 | 查看 Pod 日志中的 `执行数据库迁移失败`；必要时先手工确认库与角色 |
| `migration N was previously applied but has been modified` | 开发阶段是否改过基线迁移文件 | 开发阶段基线变更按约定需要重建库：清空 dev 库 `public` schema 后 rollout，让迁移重新执行；生产阶段不得修改基线 |

## 8. 发布记录与待执行

- 2026-11-23 dev 首次发布：platform 登记、namespace 与数据库创建、`make deploy ENV=dev` 完成；Pod 1/1 Running，`/healthz`、`/readyz` 返回 200，`/ws` 握手返回 101。提交 `41955f7a`。
- 2026-11-23 dev 公网入口：`civilization.satori.host` 证书签发、satori nginx 站点上线；`/healthz`、`/readyz` 返回 200，`/ws` 握手返回 101。
- 2026-11-23 dev v0.2 发布：提交 `0da42b7c`（成就运行时等 v0.2 服务端增量）。发布时基线迁移已随开发阶段变更，原有 dev 库因 checksum 不匹配拒绝启动；确认各表 0 行后重建 `public` schema 并重新执行迁移。Pod 1/1 Running，`/healthz`、`/readyz` 返回 200，`/ws` 握手返回 101。Secret 未注入 `STEAM_WEB_API_KEY` / `STEAM_APP_ID`，Steam 相关能力保持 `STEAM_DISABLED`。
- prod 待执行：platform 三项目标、`make deploy ENV=prod` 与 satori nginx 同步；prod 对外域名、TLS 与首次发布尚未验证。
- 空闲长连接表现尚未观察。
