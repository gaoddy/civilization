# Civilization 接入 platform 共享 Kubernetes 环境设计

> 日期：2026-11-19
> 范围：dev + prod，含对外 WebSocket 域名
> 关联：`platform/PROJECT_ONBOARDING.md`、`docs/versions/v0.1/technical/architecture/server-stack.md`、`docs/versions/v0.1/open-questions.md`

## 1. 目标

把 Civilization 唯一可部署进程 `room-service` 接入 platform 共享 Kubernetes 环境：

- dev 与 prod 各有独立 namespace（`civilization-dev` / `civilization`）、共享 PostgreSQL 项目库与项目角色。
- 服务在集群内监听 8080，提供 `/ws`、`/healthz`、`/readyz`。
- 通过 platform Gateway 对外提供 WebSocket：`wss://civilization.dev.platform.internal/ws`、`wss://civilization.platform.internal/ws`。
- 客户端新增默认关闭的 TLS 校验跳过开关，供 dev 自签证书联调。
- 仓库内自带镜像构建、Helm 清单、渲染/预检/发布/回查入口，不依赖手改集群资源。

## 2. 非目标

- 不做房间、对局、意图日志的持久化与崩溃恢复（v0.1 明确未实现）。
- 不做多副本与房间分片：服务状态仍在进程内，v0.1 固定单实例。
- 不引入 Redis、Kafka、Elasticsearch、SMS、Moderation。
- 不把 Godot 客户端打成工作负载；客户端仍由使用者在大厅输入服务端地址。
- 不改 platform 仓库已有的平台自有服务与共享基础设施。
- 不在本任务执行任何集群写入、推送或生产发布。

## 3. 现状与约束

### 3.1 Civilization 现状

| 项 | 现状 |
| --- | --- |
| 可部署进程 | `server/crates/room-service`，axum WebSocket `/ws` |
| 监听地址 | 默认 `127.0.0.1:8080`，`--listen ADDR` 覆盖 |
| 数据库 | `DATABASE_URL` 存在时 sqlx 连接 PostgreSQL，启动时执行 `server/migrations/` 迁移；无该变量时纯内存 |
| 健康检查 | 无 `/healthz`、`/readyz`（服务端技术基线写明只暴露 WebSocket 与健康检查） |
| 容器化 | 无 Dockerfile、无 Kubernetes 清单、无发布入口 |
| 客户端 | Godot；登录页已有服务端地址输入框，默认 `ws://127.0.0.1:8080/ws` |

### 3.2 platform 契约

- 业务清单留在项目仓库；平台仓库只维护 `deploy/projects/<project>.yaml` 登记表与 `deploy/config/postgres-databases.conf` 库名登记。
- namespace 的 ResourceQuota、LimitRange、默认拒绝 NetworkPolicy、DNS、Gateway 入站与共享服务出站由平台按登记表生成；项目仓库不重复声明。
- 项目库与角色由 `make ensure-project-database` 创建，凭据写入 `<project>-postgres-<db>` Secret（键：`username`、`password`、`DATABASE_URL`；prod 另有 `DATABASE_READ_URL`）。
- 运行时 DSN 走 `pgbouncer:6432`；prod 才有 `pgbouncer-read:6432`；手册建议迁移直连 primary，但已接入的 wolf 迁移 Job 同样走 `pgbouncer`（`api/deploy/helm/wolf-api/templates/migration-job.yaml` + `API_POSTGRES_RUNTIME_HOST ?= pgbouncer...`），本设计沿用该先例。
- Gateway 名称 `platform-gateway`，listener `https`；hostname 首段必须等于登记表的 `hostnamePrefix`。
- 镜像按 `linux/amd64` 构建并导入目标节点 containerd `k8s.io`，`imagePullPolicy: Never`。
- 项目自己负责 Service、HTTPRoute、鉴权边界以及 WebSocket/长连接策略。

### 3.3 已识别的关键冲突

1. **sqlx 预处理语句 + PgBouncer 事务池**：sqlx 默认 `statement_cache_capacity = 100`，事务池下跨事务复用命名预处理语句会报 `prepared statement does not exist`。必须关闭语句缓存。
2. **sqlx 迁移的会话级 advisory lock**：`Migrator` 默认用 `pg_advisory_lock` 串行化迁移；事务池下加锁与解锁可能落在不同后端连接，锁会泄漏且不提供串行化。必须关闭迁移锁，并以单实例 + `Recreate` 策略保证同一时刻只有一个迁移者。因此迁移继续放在进程启动流程里、经 `pgbouncer` 执行，不额外引入直连 primary 的 Job：prod 的 `postgres:5432` 是 Patroni headless Service，直连不保证落到 primary，而 `pgbouncer` 写入口本身只路由 primary。
3. **平台生成的 `DATABASE_URL` 带 pgx 专用参数** `default_query_exec_mode=simple_protocol`：sqlx 会忽略未知参数（已核对 `sqlx-postgres-0.8.6/src/options/parse.rs`），因此可直接消费该 Secret，不需要改写 DSN。
4. **网关空闲超时**：Envoy Gateway 未暴露 WebSocket 空闲超时配置，长连接可能被默认 stream idle timeout 断开；客户端已有 3 秒自动重连与重新认证，作为兜底。部署后用实际连接时长验证，若不可接受再提平台侧策略变更。

## 4. 方案比较

| 方案 | 做法 | 代价 | 结论 |
| --- | --- | --- | --- |
| A：kustomize base + overlay | base 放公共资源，dev/prod overlay 只写差异 | 需自己写 patch；单组件场景足够 | 未选 |
| **B：Helm chart** | 仓库内 chart + 每环境 values 文件 | 引入 Helm 模板层 | **采用** |
| C：原生清单 | 每环境一套 YAML，kubectl apply | dev/prod 90% 重复，必然漂移 | 未选 |

选 B 的理由：与 wolf、Social 两个已接入项目一致；值文件天然承载环境差异；`helm lint` / `helm template` 可离线自检；镜像、探针、HTTPRoute 的模板可读性优于手工 YAML。Helm 只在本地渲染，远端只需要 `kubectl`。

## 5. 设计

### 5.1 服务端代码改动（room-service）

**健康检查**

- `net::router` 增加 `GET /healthz` 与 `GET /readyz`。
- `/healthz` 恒返回 200，仅表示进程存活。
- `/readyz`：无数据库时返回 200；有数据库时执行 `SELECT 1`（2 秒超时），成功 200，失败 503。
- `AppState` 增加 `db: Option<Arc<Db>>`，`AppState::with_database` 保存克隆；`AppState::new` 置 `None`。

**监听地址**

- `main.rs` 增加 `ROOM_SERVICE_LISTEN` 环境变量，作为 `--listen` 未提供时的默认值；命令行参数优先级更高，默认值仍为 `127.0.0.1:8080`。容器通过 ConfigMap 注入 `0.0.0.0:8080`，不把地址写死在镜像里。

**PgBouncer 适配**

- `Db::pool` 统一设置 `PgConnectOptions::statement_cache_capacity(0)`，本地直连与集群走同一路径，避免环境差异。
- `Db::migrate` 使用 `sqlx::migrate!("../../migrations").set_locking(false)`；注释写明：单实例部署 + `Recreate` 策略下串行迁移由发布策略保证，扩展到多副本前必须迁移到独立 Job。
- 新增 `Db::ping()`（`SELECT 1`），供 `/readyz` 使用。

**测试**

- 新增 `server/crates/room-service/tests/health.rs`：用 `tower::ServiceExt::oneshot` 断言 `/healthz` 与 `/readyz` 在无数据库时均返回 200，`/ws` GET 返回协议升级所需状态（101 或 400 由 axum 决定，断言非 404 即可）。
- `cargo test -p room-service` 现有测试保持通过。

### 5.2 镜像

- 新增 `server/Dockerfile`，多阶段构建，构建上下文为 `server/`（`server/data` 被 `include_str!` 内嵌，必须一起复制）。
- builder 阶段使用与 `server/rust-toolchain.toml` 一致的 Rust 1.94.1；`cargo build --locked --release -p room-service`。
- 运行阶段 `debian:bookworm-slim`，非 root 用户，仅拷贝二进制；`ENTRYPOINT ["room-service"]`。
- 镜像名 `civilization-server`，tag 与 `ENV` 一致（`dev` / `prod`）。
- 构建命令固定 `docker build --platform linux/amd64`；导入使用 `docker save | ssh <SERVER> ctr -n k8s.io images import -`。

### 5.3 Helm chart

```
deploy/helm/
  civilization-server/
    Chart.yaml
    values.yaml
    templates/
      _helpers.tpl
      configmap.yaml
      deployment.yaml
      service.yaml
      httproute.yaml
  values.dev.yaml
  values.prod.yaml
```

**values.yaml 默认值**

| 键 | 默认 | 说明 |
| --- | --- | --- |
| `replicaCount` | `1` | v0.1 固定单实例 |
| `image.repository` | `civilization-server` | 本地导入 |
| `image.tag` | `dev` | values 文件覆盖 |
| `image.pullPolicy` | `Never` | 本地 containerd 导入 |
| `existingSecretName` | `civilization-postgres-civilization` | 平台创建的库 Secret，chart 不管理 |
| `config.listenAddr` | `0.0.0.0:8080` | ConfigMap → `ROOM_SERVICE_LISTEN` |
| `service.port` | `8080` | Service 与容器端口 |
| `resources.requests` | `cpu: 100m` / `memory: 128Mi` | 显式声明，避开 LimitRange 默认值 |
| `resources.limits` | `cpu: 1` / `memory: 512Mi` | |
| `nodeSelector` | `{}` | platform 每个环境为单节点；多节点时由 values 固定已导入镜像的节点 |
| `httpRoute.enabled` | `true` | |
| `httpRoute.gatewayName` | `platform-gateway` | |
| `httpRoute.sectionName` | `https` | |
| `httpRoute.pathPrefix` | `/` | `/ws` 与健康检查同路由 |

**values.dev.yaml**：`namespace: civilization-dev`、`platformNamespace: platform-dev`、`image.tag: dev`、`hostnames: [civilization.dev.platform.internal]`。

**values.prod.yaml**：`namespace: civilization`、`platformNamespace: platform-service`、`image.tag: prod`、`hostnames: [civilization.platform.internal]`、资源可按需上调（初始与 dev 相同）。

**Deployment 要点**

- `strategy: Recreate`：避免滚动更新期间两个进程同时迁移；也符合房间状态在进程内的现状。
- 容器端口 8080，命名 `http`。
- `envFrom.secretRef` 注入平台库 Secret；`env` 从 ConfigMap 注入 `ROOM_SERVICE_LISTEN`。
- 探针：
  - `startupProbe` `/healthz`，`periodSeconds: 2`，`failureThreshold: 30`（容纳迁移时间）。
  - `readinessProbe` `/readyz`，`periodSeconds: 5`，`failureThreshold: 3`。
  - `livenessProbe` `/healthz`，`periodSeconds: 10`，`failureThreshold: 3`。
- `securityContext`：`runAsNonRoot`、只读根文件系统、丢弃全部 capabilities、`seccompProfile: RuntimeDefault`。
- `terminationGracePeriodSeconds: 30`。
- 标签 `app.kubernetes.io/name: civilization-server`，供平台 `make status` / `make logs` 与 NetworkPolicy 使用。

**Service**：ClusterIP，端口 8080，`targetPort: http`。

**HTTPRoute**：`parentRefs` 指向 `platform-gateway`（命名空间由 values 提供）；`hostnames` 按环境；`PathPrefix /` 转发到 Service 8080。WebSocket 升级由 Envoy 透传，不需要额外注解。

**不放进 chart 的资源**：namespace、ResourceQuota、LimitRange、NetworkPolicy、PDB、ServiceMonitor。前四项由平台按登记表生成；单实例 PDB 无意义；服务没有指标端点。

### 5.4 配置与 Secret

- 非敏感配置只有监听地址，放 ConfigMap。
- 敏感配置只有数据库凭据，全部来自平台生成的 `civilization-postgres-civilization` Secret；chart 只引用不创建，避免与 `ensure-project-database` 冲突。
- 仓库与 chart 内不出现任何密码、token 或运行时 Secret 文件。

### 5.5 Makefile 发布入口

新增变量与目标（全部本地渲染、远端只执行 `kubectl`/`ctr`）：

| 目标 | 行为 |
| --- | --- |
| `deploy-lint` | `helm lint` + 两个环境各渲染一次 |
| `deploy-render ENV=dev` | `helm template` 输出到 stdout |
| `deploy-image ENV=dev` | `docker build --platform linux/amd64` |
| `deploy-load ENV=dev SERVER=...` | `docker save` 后经 SSH 导入 containerd |
| `deploy-preflight ENV=dev SERVER=...` | 渲染结果经 SSH 执行 `kubectl apply --dry-run=server -f -` |
| `deploy-apply ENV=dev SERVER=...` | 渲染结果经 SSH `kubectl apply -f -` |
| `deploy-rollout ENV=dev SERVER=...` | `kubectl rollout restart deployment/civilization-server` 后等待 `rollout status`；固定 tag 下这是让新导入镜像生效的必要步骤 |
| `deploy-status ENV=dev SERVER=...` | 输出 Pod、Service、HTTPRoute |
| `deploy-smoke ENV=dev SERVER=...` | 在目标机请求 `https://<hostname>/healthz` 与 `/readyz` |
| `deploy ENV=dev SERVER=...` | 依次 image → load → preflight → apply → rollout → smoke |

`SERVER` 默认 `root@dell.local`，与 platform 一致；`ENV` 只接受 `dev` / `prod`，其他值直接报错。

### 5.6 platform 仓库改动

- 新增 `deploy/projects/civilization.yaml`：
  - `project: civilization`，`owner: gaoddy`，`status: active`
  - dev：`namespace: civilization-dev`、`quotaProfile: small`、`hostnamePrefix: civilization`
  - prod：`namespace: civilization`、`quotaProfile: small`、`hostnamePrefix: civilization`
  - `databases: [{name: civilization, role: civilization}]`
  - `sharedServices.postgres: true`，其余 `false`
  - `gateway.northbound: true`，`paths: ["/"]`
  - `podSecurity.enforce: baseline`
- `deploy/config/postgres-databases.conf` 追加：`project civilization civilization civilization dev,prod`。
- 离线校验：`ruby scripts/validate_project_registry.rb` 与 `make validate-project ENV=dev PROJECT=civilization ONLINE=0`。
- 不执行 `ensure-project-namespace`、`ensure-project-database`、`sync-satori-prod-nginx` 等集群/生产动作。

### 5.7 客户端 TLS 配置

dev 的 Gateway 证书为自签或非公共 CA 签发，Godot 默认校验会拒绝连接；开发者机器还需要用 hosts 把 `civilization.dev.platform.internal` 指向 Gateway 节点（dev 为 `192.168.31.5`）。

- `client/project.godot` 增加 `[network] allow_insecure_tls=false`。
- `client/autoload/ws_client.gd` 在 `_open()` 里读取该设置；为 `true` 时用 `TLSOptions.client_unsafe()` 连接，否则保持默认校验。
- 默认关闭，生产构建不得打开；部署文档写明 dev 联调需要同时配置 hosts 与该开关。
- 大厅已有服务端地址输入框，无需新增 UI。

### 5.8 文档回写

- 新增 `docs/current/technical/deployment/README.md`：环境、hostname、镜像、发布命令、验证与排障。
- 更新 `docs/versions/v0.1/technical/architecture/server-stack.md`：部署一行改为「Helm chart + platform 共享 Kubernetes；单实例」；未确定事项中删除部署编排，链接到新文档。
- 更新 `docs/versions/v0.1/open-questions.md`：移除「服务端部署编排与多实例切分维度」一行，多实例切分仍作为后续版本事项保留在技术文档。
- 客户端不新增代码；部署文档写明大厅地址填 `wss://civilization.dev.platform.internal/ws`。

## 6. 验证矩阵

| 改动 | 必须执行 | 期望 |
| --- | --- | --- |
| Rust 代码 | `cargo test -p room-service`、`cargo clippy -p room-service --all-targets -- -D warnings`、`cargo fmt --check` | 通过 |
| 数据库适配 | `make db-test`（真实 PostgreSQL，直连） | 通过；迁移与社交持久化不受语句缓存关闭影响 |
| 数据库适配（PgBouncer 路径） | 本机无 PgBouncer，无法覆盖；部署到 dev 后以 `/readyz` 与启动日志验证 | 列为未验证项 |
| 镜像 | `make deploy-image ENV=dev` | `linux/amd64` 镜像产出 |
| Helm | `make deploy-lint` | 两个环境模板渲染成功、lint 无错误 |
| 离线清单 | `helm template` 输出不含明文密码、Secret 不被 chart 创建 | 人工核对 |
| platform 登记 | `validate_project_registry.rb`、`make validate-project ENV=dev PROJECT=civilization ONLINE=0` | 退出码 0 |
| 文档 | 相对链接检查、命令与路径核对 | 无失效链接 |
| 客户端 | `make client-check`、`make client-import` | 通过；新增设置默认 false |

不可在本任务验证的项：真实集群 apply、rollout、Gateway 解析、WS 经网关的长时间连接、prod nginx 转发。用户已确认本批只交付清单与离线校验，全部集群动作列入「阻塞」。客户端 TLS 只通过 `client-check` 做静态验证，不连接真实域名。

## 7. 风险

| 风险 | 影响 | 处理 |
| --- | --- | --- |
| Envoy 空闲超时断开长连接 | 大厅挂机可能被断 | 客户端自动重连兜底；部署后实测，必要时提平台侧策略变更 |
| 关闭迁移锁 | 多实例同时迁移会冲突 | `replicaCount: 1` + `Recreate`；扩展到多副本前改独立迁移 Job |
| 固定镜像 tag | 再次发布时 Pod 模板不变，不会自动更新 | `deploy-rollout` 先 `rollout restart`；不做不可变 tag 以免 Makefile 与环境值复杂化 |
| 关闭语句缓存 | 单查询多一次解析开销 | 首版规模可接受；换取 PgBouncer 兼容 |
| prod 首次发布需要平台侧 nginx 同步 | 域名不通 | 列为阻塞项，由用户执行或授权后执行 |
| 目标节点镜像导入依赖 SERVER 可达 | 无法在本机验证发布链路 | 本任务只做离线校验，集群动作列为未验证 |

## 8. 已确认参数与交付边界

已确认：

1. 客户端按「内网 + 信任自签证书」接入：dev 用 hosts 指向 `192.168.31.5`，客户端新增默认关闭的跳过证书校验设置。
2. 本批只交付两个仓库的清单、代码与离线校验；不执行 `ensure-project-namespace`、`ensure-project-database`、镜像导入、apply、rollout、smoke 与 prod nginx 同步。

交付时随附待执行清单：platform 侧 `make ensure-project-namespace` / `ensure-project-database` / `validate-project ONLINE=1`，以及 Civilization 侧 `make deploy ENV=<env> SERVER=<ssh-target>` 与 `make sync-satori-prod-nginx`（prod 首次对外）。`SERVER` 取值由执行者按实际环境传入，不写入仓库默认值以外的地方。
