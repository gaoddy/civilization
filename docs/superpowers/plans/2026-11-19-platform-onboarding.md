# Civilization 接入 platform 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 `room-service` 以 Helm 清单接入 platform 的 dev/prod Kubernetes 环境，并提供健康检查、PgBouncer 兼容、镜像构建、发布入口与平台登记表。

**Architecture:** 服务端新增 `/healthz`、`/readyz` 与 PgBouncer 适配；仓库内 `server/Dockerfile` 产出 `linux/amd64` 镜像，`deploy/helm/` 提供单服务 chart 与 dev/prod 值文件；Makefile 只在本机渲染、远端只用 `kubectl`/`ctr`；platform 仓库登记项目、库与 hostname。

**Tech Stack:** Rust 1.94.1、axum、sqlx、PostgreSQL、PgBouncer、Docker、Helm 4、Kubernetes Gateway API、Godot 4。

**Spec:** `docs/superpowers/specs/2026-11-19-platform-onboarding-design.md`

## Global Constraints

- 服务只部署 `room-service` 一个进程；`replicaCount: 1` 且 Deployment `strategy: Recreate`。
- 镜像必须 `linux/amd64`，`imagePullPolicy: Never`；tag 固定 `dev` / `prod`，重新发布由 `kubectl rollout restart` 生效。
- 运行时 DSN 来自平台 Secret `civilization-postgres-civilization`；chart 不创建 Secret。
- sqlx 必须 `statement_cache_capacity(0)`；迁移必须 `set_locking(false)`。
- chart 不声明 Namespace、ResourceQuota、LimitRange、NetworkPolicy、PDB、ServiceMonitor。
- 本批不执行任何集群写入、镜像导入、apply、rollout、smoke 或 nginx 同步。
- 发布入口统一走 `make`；不手写与目标等价的命令。
- 客户端跳过 TLS 校验开关默认 `false`，生产构建不得打开。
- 两个仓库的工作位置：Civilization `/Users/gaoddy/Projects/Civilization/.worktrees/feature-platform-onboarding`；platform `/Users/gaoddy/Projects/platform/.worktrees/codex-civilization-onboarding`。子 agent 只在对应目录内执行。

## Review Focus

- 数据库启动后不可用：`/readyz` 必须返回 503（Task 1 的 `readiness` 映射测试覆盖）。
- 迁移重复执行：同一数据库上连续启动两次不得重复建表或失败（Task 2 的重复迁移测试覆盖）。
- 健康探针路由与 `/ws` 共存：`/healthz`、`/readyz` 不得被 `/ws` 捕获或返回 404（Task 1 的路由测试覆盖）。
- 客户端在 TLS 开关缺失时仍能连本地 `ws://`，且不因设置缺失报错（Task 6 的边界测试覆盖）。
- 镜像缺少 `server/data` 时 `include_str!` 编译失败：构建上下文必须是 `server/`（Task 3 的构建验证覆盖）。

---

### Task 1: 健康检查路由

**Files:**
- Create: `server/crates/room-service/tests/health.rs`
- Modify: `server/crates/room-service/src/lib.rs`（`AppState` 与 `with_database`）
- Modify: `server/crates/room-service/src/net.rs`（`router`、handlers、单元测试）
- Modify: `server/crates/room-service/Cargo.toml`（dev-dependency `tower`）

**Interfaces:**
- Consumes: `room_service::bind_state(listen, clock) -> Result<(SocketAddr, Router, AppState), String>`
- Produces: `AppState { pub db: Option<Arc<db::Db>> }`；`net::router(AppState) -> Router` 同时挂载 `/ws`、`/healthz`、`/readyz`；`fn readiness(is_db_ok: bool) -> StatusCode`（`net.rs` 内私有）。

- [ ] **Step 1: 写失败测试**

创建 `server/crates/room-service/tests/health.rs`：

```rust
//! 健康检查路由：健康探针与 WebSocket 共存，未配置数据库时均可用。

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use room_service::SystemClock;
use tower::ServiceExt;

async fn app() -> axum::Router {
    let (_addr, router, _state) = room_service::bind_state("127.0.0.1:0", Arc::new(SystemClock))
        .await
        .expect("路由应能装配");
    router
}

#[tokio::test]
async fn healthz_is_ok() {
    let response = app()
        .await
        .oneshot(Request::builder().uri("/healthz").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn readyz_without_database_is_ok() {
    let response = app()
        .await
        .oneshot(Request::builder().uri("/readyz").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn ws_route_is_registered() {
    let response = app()
        .await
        .oneshot(Request::builder().uri("/ws").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_ne!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn unknown_path_is_not_found() {
    let response = app()
        .await
        .oneshot(Request::builder().uri("/nope").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
```

并在 `server/crates/room-service/Cargo.toml` 的 `[dev-dependencies]` 增加：

```toml
tower = { version = "0.5", features = ["util"] }
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd server && cargo test -p room-service --test health -v`
Expected: 编译通过但 `/healthz`、`/readyz` 返回 404，断言失败。

- [ ] **Step 3: 实现状态字段与路由**

`lib.rs`：

- `AppState` 增加字段 `pub db: Option<Arc<db::Db>>`。
- `AppState::new` 置 `db: None`。
- `with_database` 在 `social::SocialStore::load(db.clone(), ...)` 之后保留一个 `db` 克隆，写入 `db: Some(db)`。

`net.rs`：

- `router` 增加 `.route("/healthz", get(healthz))` 与 `.route("/readyz", get(readyz))`。
- 新增：

```rust
/// 存活检查：进程能响应即可。
async fn healthz() -> StatusCode {
    StatusCode::OK
}

/// 就绪检查：无数据库时恒就绪；有数据库时要求 `SELECT 1` 在 2 秒内成功。
async fn readyz(State(state): State<AppState>) -> StatusCode {
    let Some(db) = state.db.as_ref() else {
        return readiness(true);
    };
    let ok = tokio::time::timeout(Duration::from_secs(2), db.ping())
        .await
        .map(|result| result.is_ok())
        .unwrap_or(false);
    readiness(ok)
}

/// 数据库可用性映射为就绪状态码；独立出来便于单测覆盖 503 分支。
fn readiness(is_db_ok: bool) -> StatusCode {
    if is_db_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}
```

- `net.rs` 需要引入 `axum::http::StatusCode`；`Duration` 已有 `std::time::Duration` 引用。
- `db.ping()` 由 Task 2 实现；本任务先加一个临时桩会破坏编译，因此**本任务同时**在 `db.rs` 增加最小实现：

```rust
/// 执行 `SELECT 1`，供就绪探针使用。
///
/// # Errors
///
/// 查询失败时返回错误。
pub async fn ping(&self) -> DbResult<()> {
    sqlx::query("SELECT 1").execute(&self.pool).await.map(|_| ())
}
```

- 在 `net.rs` 末尾增加单元测试：

```rust
#[cfg(test)]
mod tests {
    use super::readiness;
    use axum::http::StatusCode;

    #[test]
    fn readiness_maps_database_state() {
        assert_eq!(readiness(true), StatusCode::OK);
        assert_eq!(readiness(false), StatusCode::SERVICE_UNAVAILABLE);
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd server && cargo test -p room-service --test health -v && cargo test -p room-service --lib -v`
Expected: PASS，4 个集成用例与 1 个映射用例通过。

- [ ] **Step 5: 提交**

```bash
git add server/crates/room-service/src/lib.rs server/crates/room-service/src/net.rs \
  server/crates/room-service/src/db.rs server/crates/room-service/tests/health.rs \
  server/crates/room-service/Cargo.toml
git commit -m "feat(room-service): 增加 healthz/readyz 与就绪检查"
```

---

### Task 2: PgBouncer 适配与监听环境变量

**Files:**
- Modify: `server/crates/room-service/src/db.rs`
- Modify: `server/crates/room-service/src/main.rs`
- Modify: `server/crates/room-service/tests/persistence.rs`

**Interfaces:**
- Consumes: `Db::connect(database_url: &str) -> Result<Self, String>`、`Db::ping()`（Task 1）
- Produces: `Db::pool` 固定 `statement_cache_capacity(0)`；`Db::migrate` 关闭迁移锁；`ROOM_SERVICE_LISTEN` 环境变量作为 `--listen` 缺省值。

- [ ] **Step 1: 写失败测试**

在 `server/crates/room-service/tests/persistence.rs` 末尾增加：

```rust
#[tokio::test]
async fn ping_succeeds_and_migrations_are_repeatable() {
    let Some(url) = common::test_database_url() else {
        skip("ping_succeeds_and_migrations_are_repeatable");
        return;
    };
    let first = room_service::Db::connect(&url).await.expect("首次连接与迁移应成功");
    first.ping().await.expect("SELECT 1 应成功");
    drop(first);
    room_service::Db::connect(&url)
        .await
        .expect("重复连接与迁移应幂等");
}
```

若 `common` 没有暴露 `test_database_url()`，就在 `tests/common/mod.rs` 增加：

```rust
pub fn test_database_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .ok()
        .filter(|value| !value.trim().is_empty())
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `make db-up && make db-test`
Expected: `ping_succeeds_and_migrations_are_repeatable` 失败（若 `common::test_database_url` 不存在则编译失败）。

- [ ] **Step 3: 实现适配**

`db.rs` 的 `pool`：在 `PgConnectOptions::from_str` 之后、schema options 之前加入：

```rust
// PgBouncer 事务池不保证跨事务复用命名预处理语句；关闭语句缓存。
options = options.statement_cache_capacity(0);
```

`db.rs` 的 `migrate`：

```rust
let mut migrator = sqlx::migrate!("../../migrations");
// 事务池下会话级 advisory lock 会泄漏；单实例 + Recreate 保证同一时刻只有一个迁移者。
migrator.set_locking(false);
migrator
    .run(&self.pool)
    .await
    .map_err(|err| format!("执行数据库迁移失败: {err}"))
```

`main.rs`：

```rust
let mut listen = std::env::var("ROOM_SERVICE_LISTEN")
    .ok()
    .filter(|value| !value.trim().is_empty())
    .unwrap_or_else(|| DEFAULT_LISTEN.to_string());
```

并在 `USAGE` 增加一行：`ROOM_SERVICE_LISTEN  未提供 --listen 时的监听地址，默认 127.0.0.1:8080`。

- [ ] **Step 4: 运行测试确认通过**

Run: `make db-test && cd server && cargo test -p room-service --test health && cargo clippy -p room-service --all-targets -- -D warnings && cargo fmt --check`
Expected: 全部退出 0。

- [ ] **Step 5: 提交**

```bash
git add server/crates/room-service/src/db.rs server/crates/room-service/src/main.rs \
  server/crates/room-service/tests/persistence.rs server/crates/room-service/tests/common/mod.rs
git commit -m "feat(room-service): 适配 PgBouncer 事务池并支持监听地址环境变量"
```

---

### Task 3: 服务端镜像

**Files:**
- Create: `server/Dockerfile`
- Create: `.dockerignore`（根白名单：`server/`、`protocol/`）

**Interfaces:**
- Consumes: `server/Cargo.lock`、`server/crates/room-service`、`server/data/v0.1/*.json`、`protocol/generated/protocol.rs`
- Produces: 镜像 `civilization-server:dev` / `:prod`，默认入口 `room-service`，监听 8080（由 `ROOM_SERVICE_LISTEN` 决定）。

- [ ] **Step 1: 写 Dockerfile**

```dockerfile
# room-service 运行镜像。构建上下文是仓库根：protocol 生成物与 server/data 都要复制进去。
FROM rust:1.94.1-bookworm AS builder
WORKDIR /src
COPY server/ /src/server/
COPY protocol/ /src/protocol/
WORKDIR /src/server
RUN cargo build --locked --release -p room-service

FROM debian:bookworm-slim
RUN useradd --system --uid 10001 --home /nonexistent --shell /usr/sbin/nologin room-service
COPY --from=builder /src/server/target/release/room-service /usr/local/bin/room-service
USER 10001
EXPOSE 8080
ENTRYPOINT ["room-service"]
```

- [ ] **Step 2: 构建验证**

Run: `docker build --platform linux/amd64 -t civilization-server:dev -f server/Dockerfile .`
Expected: 退出 0；若 `include_str!` 文件缺失会在此失败。

- [ ] **Step 3: 启动验证**

Run:

```bash
docker run --rm -d --name civilization-image-check -e ROOM_SERVICE_LISTEN=0.0.0.0:8080 -p 18080:8080 civilization-server:dev
sleep 2
curl -fsS http://127.0.0.1:18080/healthz
docker logs civilization-image-check 2>&1 | head -5
docker rm -f civilization-image-check
```

Expected: `curl` 退出 0；日志出现 `监听 ws://0.0.0.0:8080/ws` 与内存存储提示。如果 18080 已被占用，改用其他端口并对应修改 curl。

- [ ] **Step 4: 提交**

```bash
git add server/Dockerfile .dockerignore
git commit -m "feat(server): 增加 room-service 容器镜像"
```

---

### Task 4: Helm chart 与值文件

**Files:**
- Create: `deploy/helm/civilization-server/Chart.yaml`
- Create: `deploy/helm/civilization-server/values.yaml`
- Create: `deploy/helm/civilization-server/templates/_helpers.tpl`
- Create: `deploy/helm/civilization-server/templates/configmap.yaml`
- Create: `deploy/helm/civilization-server/templates/deployment.yaml`
- Create: `deploy/helm/civilization-server/templates/service.yaml`
- Create: `deploy/helm/civilization-server/templates/httproute.yaml`
- Create: `deploy/helm/values.dev.yaml`
- Create: `deploy/helm/values.prod.yaml`

**Interfaces:**
- Consumes: 镜像名 `civilization-server`；Secret `civilization-postgres-civilization`
- Produces: 渲染名 `civilization-server`；标签 `app.kubernetes.io/name: civilization-server`；Service 8080；HTTPRoute hostname 由值文件注入。

- [ ] **Step 1: 写 chart 元数据与默认值**

`Chart.yaml`：

```yaml
apiVersion: v2
name: civilization-server
description: Kubernetes deployment chart for the Civilization room service.
type: application
version: 0.1.0
appVersion: "0.1.0"
```

`values.yaml`：

```yaml
replicaCount: 1

image:
  repository: civilization-server
  tag: dev
  pullPolicy: Never

namespace: civilization-dev
platformNamespace: platform-dev

existingSecretName: civilization-postgres-civilization

config:
  listenAddr: 0.0.0.0:8080

service:
  port: 8080

resources:
  requests:
    cpu: 100m
    memory: 128Mi
  limits:
    cpu: "1"
    memory: 512Mi

nodeSelector: {}

httpRoute:
  enabled: true
  gatewayName: platform-gateway
  gatewayNamespace: platform-dev
  sectionName: https
  hostnames: []
  pathPrefix: /
```

- [ ] **Step 2: 写模板**

`_helpers.tpl`：定义 `civilization-server.labels`（`helm.sh/chart`、`app.kubernetes.io/name: civilization-server`、`app.kubernetes.io/instance: {{ .Release.Name }}`、`app.kubernetes.io/managed-by`、`app.kubernetes.io/version`）与 `civilization-server.selectorLabels`（`app.kubernetes.io/name`、`app.kubernetes.io/instance`）。

`configmap.yaml`：

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: civilization-server-config
  namespace: {{ .Values.namespace }}
  labels:
    {{- include "civilization-server.labels" . | nindent 4 }}
data:
  listen-addr: {{ .Values.config.listenAddr | quote }}
```

`deployment.yaml` 关键字段：

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: civilization-server
  namespace: {{ .Values.namespace }}
  labels:
    {{- include "civilization-server.labels" . | nindent 4 }}
spec:
  replicas: {{ .Values.replicaCount }}
  strategy:
    type: Recreate
  selector:
    matchLabels:
      {{- include "civilization-server.selectorLabels" . | nindent 6 }}
  template:
    metadata:
      labels:
        {{- include "civilization-server.selectorLabels" . | nindent 8 }}
    spec:
      {{- with .Values.nodeSelector }}
      nodeSelector:
        {{- toYaml . | nindent 8 }}
      {{- end }}
      securityContext:
        runAsNonRoot: true
        runAsUser: 10001
        runAsGroup: 10001
        fsGroup: 10001
        seccompProfile:
          type: RuntimeDefault
      containers:
        - name: server
          image: "{{ .Values.image.repository }}:{{ .Values.image.tag }}"
          imagePullPolicy: {{ .Values.image.pullPolicy }}
          securityContext:
            allowPrivilegeEscalation: false
            readOnlyRootFilesystem: true
            capabilities:
              drop: ["ALL"]
          ports:
            - name: http
              containerPort: 8080
          env:
            - name: ROOM_SERVICE_LISTEN
              valueFrom:
                configMapKeyRef:
                  name: civilization-server-config
                  key: listen-addr
          envFrom:
            - secretRef:
                name: {{ required "existingSecretName is required" .Values.existingSecretName }}
          startupProbe:
            httpGet: { path: /healthz, port: http }
            periodSeconds: 2
            failureThreshold: 30
          readinessProbe:
            httpGet: { path: /readyz, port: http }
            periodSeconds: 5
            failureThreshold: 3
          livenessProbe:
            httpGet: { path: /healthz, port: http }
            periodSeconds: 10
            failureThreshold: 3
          resources:
            {{- toYaml .Values.resources | nindent 12 }}
      terminationGracePeriodSeconds: 30
```

`service.yaml`：ClusterIP，`port: {{ .Values.service.port }}`，`targetPort: http`，selector 用 `civilization-server.selectorLabels`。

`httproute.yaml`：

```yaml
{{- if .Values.httpRoute.enabled }}
apiVersion: gateway.networking.k8s.io/v1
kind: HTTPRoute
metadata:
  name: civilization-server
  namespace: {{ .Values.namespace }}
  labels:
    {{- include "civilization-server.labels" . | nindent 4 }}
spec:
  parentRefs:
    - name: {{ .Values.httpRoute.gatewayName }}
      namespace: {{ .Values.httpRoute.gatewayNamespace }}
      sectionName: {{ .Values.httpRoute.sectionName }}
  hostnames:
    {{- range .Values.httpRoute.hostnames }}
    - {{ . | quote }}
    {{- end }}
  rules:
    - matches:
        - path:
            type: PathPrefix
            value: {{ .Values.httpRoute.pathPrefix }}
      backendRefs:
        - name: civilization-server
          port: {{ .Values.service.port }}
{{- end }}
```

- [ ] **Step 3: 写环境值文件**

`deploy/helm/values.dev.yaml`：

```yaml
namespace: civilization-dev
platformNamespace: platform-dev
image:
  tag: dev
httpRoute:
  gatewayNamespace: platform-dev
  hostnames:
    - civilization.dev.platform.internal
```

`deploy/helm/values.prod.yaml`：

```yaml
namespace: civilization
platformNamespace: platform-service
image:
  tag: prod
httpRoute:
  gatewayNamespace: platform-service
  hostnames:
    - civilization.platform.internal
```

- [ ] **Step 4: 渲染与断言**

Run:

```bash
helm lint deploy/helm/civilization-server -f deploy/helm/values.dev.yaml
helm template civilization-server deploy/helm/civilization-server -f deploy/helm/values.dev.yaml > /tmp/civ-dev.yaml
helm template civilization-server deploy/helm/civilization-server -f deploy/helm/values.prod.yaml > /tmp/civ-prod.yaml
grep -q "namespace: civilization-dev" /tmp/civ-dev.yaml
grep -q "civilization.dev.platform.internal" /tmp/civ-dev.yaml
grep -q "civilization.platform.internal" /tmp/civ-prod.yaml
grep -q "kind: Secret" /tmp/civ-dev.yaml && exit 1 || true
grep -q "type: Recreate" /tmp/civ-dev.yaml
```

Expected: lint 无错误；断言全部通过；渲染结果中没有 `kind: Secret`。（`/tmp` 仅用于一次性验证，不提交。）

- [ ] **Step 5: 提交**

```bash
git add deploy/helm
git commit -m "feat(deploy): 增加 civilization-server Helm chart 与 dev/prod 值"
```

---

### Task 5: 发布入口

**Files:**
- Modify: `Makefile`（变量区、`.PHONY`、目标区）

**Interfaces:**
- Consumes: chart `deploy/helm/civilization-server`；值文件 `deploy/helm/values.<env>.yaml`；镜像 `civilization-server:<env>`
- Produces: `deploy-lint`、`deploy-render`、`deploy-image`、`deploy-load`、`deploy-preflight`、`deploy-apply`、`deploy-rollout`、`deploy-status`、`deploy-smoke`、`deploy`

- [ ] **Step 1: 增加变量**

在 `Makefile` 顶部（`DEV_PORT` 附近）增加：

```make
ENV ?= dev
SERVER ?= root@dell.local
DEPLOY_CHART ?= deploy/helm/civilization-server
DEPLOY_VALUES ?= deploy/helm/values.$(ENV).yaml
DEPLOY_NAMESPACE ?= $(if $(filter prod,$(ENV)),civilization,civilization-dev)
DEPLOY_HOST ?= $(if $(filter prod,$(ENV)),civilization.platform.internal,civilization.dev.platform.internal)
DEPLOY_IMAGE ?= civilization-server:$(ENV)
```

- [ ] **Step 2: 增加目标**

```make
deploy-render:
	helm template civilization-server $(DEPLOY_CHART) --namespace $(DEPLOY_NAMESPACE) -f $(DEPLOY_VALUES)

deploy-lint:
	helm lint $(DEPLOY_CHART) -f deploy/helm/values.dev.yaml
	helm lint $(DEPLOY_CHART) -f deploy/helm/values.prod.yaml
	$(MAKE) deploy-render ENV=dev >/dev/null
	$(MAKE) deploy-render ENV=prod >/dev/null

deploy-image:
	docker build --platform linux/amd64 -t $(DEPLOY_IMAGE) -f server/Dockerfile .

deploy-load:
	docker save $(DEPLOY_IMAGE) | ssh $(SERVER) "ctr -n k8s.io images import -"

deploy-preflight:
	$(MAKE) deploy-render ENV=$(ENV) | ssh $(SERVER) "kubectl apply --dry-run=server -f -"

deploy-apply:
	$(MAKE) deploy-render ENV=$(ENV) | ssh $(SERVER) "kubectl apply -f -"

deploy-rollout:
	ssh $(SERVER) "kubectl rollout restart deployment/civilization-server -n $(DEPLOY_NAMESPACE) && kubectl rollout status deployment/civilization-server -n $(DEPLOY_NAMESPACE) --timeout=180s"

deploy-status:
	ssh $(SERVER) "kubectl get pods,service,httproute -n $(DEPLOY_NAMESPACE) -l app.kubernetes.io/name=civilization-server"

deploy-smoke:
	ssh $(SERVER) "curl -fsS https://$(DEPLOY_HOST)/healthz && curl -fsS https://$(DEPLOY_HOST)/readyz"

deploy: deploy-image deploy-load deploy-preflight deploy-apply deploy-rollout deploy-smoke
```

把以上名称加入 `.PHONY`。

- [ ] **Step 3: 验证离线目标**

Run: `make deploy-lint`
Expected: 两个环境的 lint 与渲染均退出 0。

Run: `make deploy-render ENV=dev | grep -q "civilization.dev.platform.internal"`
Expected: 退出 0。

不执行 `deploy-image`/`deploy-load`/`deploy-*` 中任何连接集群或构建镜像的目标（本批边界）。

- [ ] **Step 4: 提交**

```bash
git add Makefile
git commit -m "feat(deploy): 增加 room-service 渲染与发布入口"
```

---

### Task 6: 客户端 TLS 开关

**Files:**
- Modify: `client/project.godot`
- Modify: `client/autoload/ws_client.gd`
- Modify: `tests/test_client_boundary.py`

**Interfaces:**
- Consumes: Godot `ProjectSettings`
- Produces: 项目设置 `network/allow_insecure_tls`（默认 `false`）；`WebSocketPeer.connect_to_url(url, tls_options)` 按设置选择 `TLSOptions.client_unsafe()` 或默认校验。

- [ ] **Step 1: 写失败测试**

在 `tests/test_client_boundary.py` 增加：

```python
    def test_insecure_tls_default_off(self):
        project = (CLIENT / "project.godot").read_text(encoding="utf-8")
        self.assertRegex(project, r"(?m)^allow_insecure_tls=false$")

    def test_ws_client_reads_tls_setting(self):
        source = (CLIENT / "autoload" / "ws_client.gd").read_text(encoding="utf-8")
        self.assertIn('ProjectSettings.get_setting("network/allow_insecure_tls"', source)
        self.assertIn("TLSOptions.client_unsafe()", source)
```

- [ ] **Step 2: 运行测试确认失败**

Run: `python3 -m unittest tests.test_client_boundary -v`
Expected: 两个新用例失败。

- [ ] **Step 3: 实现**

`client/project.godot` 在 `[internationalization]` 之前增加：

```ini
[network]

allow_insecure_tls=false
```

`client/autoload/ws_client.gd` 的 `_open()`：

```gdscript
    var insecure_tls: bool = ProjectSettings.get_setting("network/allow_insecure_tls", false)
    var tls: TLSOptions = TLSOptions.client_unsafe() if insecure_tls else null
    var err: int = socket.connect_to_url(url, tls)
```

- [ ] **Step 4: 验证**

Run: `python3 -m unittest tests.test_client_boundary -v && make client-import && make client-check`
Expected: 全部退出 0。

- [ ] **Step 5: 提交**

```bash
git add client/project.godot client/autoload/ws_client.gd tests/test_client_boundary.py
git commit -m "feat(client): 增加可选的自签证书连接开关"
```

---

### Task 7: platform 登记表与库名

**Files:**
- Create（platform 仓库）: `deploy/projects/civilization.yaml`
- Modify（platform 仓库）: `deploy/config/postgres-databases.conf`
- 工作目录：`/Users/gaoddy/Projects/platform/.worktrees/codex-civilization-onboarding`

**Interfaces:**
- Consumes: platform `scripts/validate_project_registry.rb` 契约
- Produces: 项目 `civilization`、namespace `civilization-dev` / `civilization`、库与角色 `civilization`、hostname 前缀 `civilization`、`sharedServices.postgres: true`、`gateway.northbound: true`

- [ ] **Step 1: 写登记表**

`deploy/projects/civilization.yaml`：

```yaml
schemaVersion: 1
project: civilization
owner: gaoddy
status: active
environments:
  dev:
    namespace: civilization-dev
    quotaProfile: small
    hostnamePrefix: civilization
  prod:
    namespace: civilization
    quotaProfile: small
    hostnamePrefix: civilization
databases:
  - name: civilization
    role: civilization
sharedServices:
  postgres: true
  redis: false
  kafka: false
  elasticsearch: false
  moderation: false
  sms: false
podSecurity:
  enforce: baseline
gateway:
  northbound: true
  paths: ["/"]
notes: |
  room-service（Rust，axum WebSocket）经 pgbouncer:6432 使用项目库 civilization；
  Gateway 暴露 /ws 与 /healthz、/readyz，单实例部署。
```

- [ ] **Step 2: 登记库名**

`deploy/config/postgres-databases.conf` 末尾追加：

```text
project civilization civilization civilization dev,prod
```

- [ ] **Step 3: 离线校验**

Run（platform worktree 内）:

```bash
ruby scripts/validate_project_registry.rb
make validate-project ENV=dev PROJECT=civilization ONLINE=0
make validate-project ENV=prod PROJECT=civilization ONLINE=0
```

Expected: 全部退出 0。不执行 `ensure-project-namespace`、`ensure-project-database`、`make validate` 之外的集群命令。

- [ ] **Step 4: 提交**

```bash
git add deploy/projects/civilization.yaml deploy/config/postgres-databases.conf
git commit -m "feat(projects): 登记 civilization 项目与数据库"
```

---

### Task 8: 文档回写

**Files:**
- Create: `docs/current/technical/deployment/README.md`
- Modify: `docs/current/technical/README.md`
- Modify: `docs/versions/v0.1/technical/architecture/server-stack.md`
- Modify: `docs/versions/v0.1/open-questions.md`

**Interfaces:**
- Consumes: 设计文档、Task 4/5/7 的路径与命令
- Produces: 部署现状文档；技术基线不再把部署列为未确定；待确认事项移除部署编排行

- [ ] **Step 1: 写部署现状文档**

`docs/current/technical/deployment/README.md` 覆盖：

- 环境表：`dev` → `civilization-dev` + `civilization.dev.platform.internal`；`prod` → `civilization` + `civilization.platform.internal`。
- 镜像：`server/Dockerfile`、`docker build --platform linux/amd64`、`ctr -n k8s.io images import -`、`imagePullPolicy: Never`、发布后 `rollout restart`。
- 配置与 Secret：仅 `ROOM_SERVICE_LISTEN` 与平台生成的 `civilization-postgres-civilization`；不提交密码。
- 发布命令：`make deploy-lint`、`make deploy ENV=<env> SERVER=<ssh-target>`；先执行平台侧 `ensure-project-namespace` / `ensure-project-database`。
- 客户端：大厅填 `wss://civilization.dev.platform.internal/ws`；dev 需 hosts 指向 `192.168.31.5` 并打开 `network/allow_insecure_tls`。
- 约束：单实例、`Recreate`、迁移随进程启动、PgBouncer 语句缓存与迁移锁关闭；扩展到多副本前必须改独立迁移 Job。
- 排障：`/readyz` 503 查数据库与 NetworkPolicy；镜像不生效查是否执行 rollout restart；域名不通查 HTTPRoute 与 hosts。

- [ ] **Step 2: 更新索引与技术基线**

- `docs/current/technical/README.md` 表格增加一行：`[部署与接入](deployment/README.md)`，内容写「platform Kubernetes 接入、Helm 清单、镜像与发布入口、健康检查、客户端连接方式」，代码位置写 `deploy/helm`、`server/Dockerfile`、`Makefile`。
- `docs/versions/v0.1/technical/architecture/server-stack.md`：
  - §1 部署行改为「单二进制 + Helm chart，部署到 platform 共享 Kubernetes；单实例」。
  - §5 删除「部署编排（容器编排方式、单实例到多实例的切分维度）」一条，改为链接 `../../../../current/technical/deployment/README.md`；保留多实例切分作为后续版本事项的一句说明。
- `docs/versions/v0.1/open-questions.md` 移除「服务端部署编排与多实例切分维度」整行。

- [ ] **Step 3: 验证链接与命令**

Run:

```bash
python3 - <<'PY'
import pathlib, re
docs = [
    pathlib.Path('docs/current/technical/deployment/README.md'),
    pathlib.Path('docs/current/technical/README.md'),
    pathlib.Path('docs/versions/v0.1/technical/architecture/server-stack.md'),
    pathlib.Path('docs/versions/v0.1/open-questions.md'),
]
missing = []
for doc in docs:
    for target in re.findall(r'\]\(([^)#]+)', doc.read_text(encoding='utf-8')):
        if target.startswith(('http://', 'https://', 'mailto:')):
            continue
        if not (doc.parent / target).resolve().exists():
            missing.append(f'{doc}: {target}')
print('\n'.join(missing) if missing else 'links ok')
PY
```

Expected: `links ok`（只校验本次改动的四个文件）。

- [ ] **Step 4: 提交**

```bash
git add docs/current/technical/deployment/README.md docs/current/technical/README.md \
  docs/versions/v0.1/technical/architecture/server-stack.md docs/versions/v0.1/open-questions.md
git commit -m "docs: 回写 platform 部署现状与客户端接入方式"
```

---

## 全量验证（实施完成后）

```bash
make deploy-lint
make test
make lint
make client-check
make db-up && make db-test
```

platform worktree：

```bash
ruby scripts/validate_project_registry.rb
make validate-project ENV=dev PROJECT=civilization ONLINE=0
make validate-project ENV=prod PROJECT=civilization ONLINE=0
```
