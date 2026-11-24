# 服务端部署：本机渲染清单，远端只执行 kubectl/ctr。由根 Makefile include。
# ENV 是部署环境（dev / prod），与客户端构建的 ENV/env 相互独立。

ENV ?= dev
SERVER ?= root@dell.local
DEPLOY_CHART ?= deploy/helm/civilization-server
DEPLOY_VALUES ?= deploy/helm/values.$(ENV).yaml
DEPLOY_NAMESPACE ?= $(if $(filter prod,$(ENV)),civilization,civilization-dev)
DEPLOY_HOST ?= $(if $(filter prod,$(ENV)),civilization.platform.internal,civilization.dev.platform.internal)
DEPLOY_IMAGE ?= civilization-server:$(ENV)

.PHONY: deploy-render deploy-lint deploy-check-env deploy-image deploy-load deploy-preflight \
	deploy-apply deploy-rollout deploy-status deploy-smoke deploy

# 输出就是清单本身：命令回显会混进管道，被 kubectl 当成清单解析失败。
deploy-render:
	@helm template civilization-server $(DEPLOY_CHART) --namespace $(DEPLOY_NAMESPACE) -f $(DEPLOY_VALUES)

deploy-lint:
	helm lint $(DEPLOY_CHART) -f deploy/helm/values.dev.yaml
	helm lint $(DEPLOY_CHART) -f deploy/helm/values.prod.yaml
	$(MAKE) deploy-render ENV=dev >/dev/null
	$(MAKE) deploy-render ENV=prod >/dev/null

deploy-image:
	docker build --platform linux/amd64 -t $(DEPLOY_IMAGE) -f server/Dockerfile .

deploy-load:
	docker save $(DEPLOY_IMAGE) | ssh $(SERVER) "ctr -n k8s.io images import -"

# ENV 拼错时不能静默回落到 dev 的 namespace 与域名。
deploy-check-env:
	@$(if $(filter dev prod,$(ENV)),,$(error ENV 只能是 dev 或 prod，收到：$(ENV)))

# 子 make 必须静默：它回显的命令行会混进管道，被 kubectl 当成清单解析失败。
deploy-preflight: deploy-check-env
	$(MAKE) -s deploy-render ENV=$(ENV) | ssh $(SERVER) "kubectl apply --dry-run=server -f -"

deploy-apply: deploy-check-env
	$(MAKE) -s deploy-render ENV=$(ENV) | ssh $(SERVER) "kubectl apply -f -"

deploy-rollout: deploy-check-env
	ssh $(SERVER) "kubectl rollout restart deployment/civilization-server -n $(DEPLOY_NAMESPACE) && kubectl rollout status deployment/civilization-server -n $(DEPLOY_NAMESPACE) --timeout=180s"

deploy-status: deploy-check-env
	ssh $(SERVER) "kubectl get pods,service,httproute -n $(DEPLOY_NAMESPACE) -l app.kubernetes.io/name=civilization-server"

deploy-smoke: deploy-check-env
	ssh $(SERVER) "curl -fsS https://$(DEPLOY_HOST)/healthz && curl -fsS https://$(DEPLOY_HOST)/readyz"

deploy: deploy-check-env deploy-image deploy-load deploy-preflight deploy-apply deploy-rollout deploy-smoke

# 快捷别名：deploy-<环境> = deploy ENV=<环境>；deploy-<步骤>-<环境> = deploy-<步骤> ENV=<环境>。
# deploy-lint 同时校验两套 values、deploy-check-env 是内部校验，都与环境无关，不生成别名。
DEPLOY_ALIAS_STEPS := render image load preflight apply rollout status smoke
DEPLOY_ALIAS_ENVS := dev prod

define deploy_step_alias
.PHONY: deploy-$(1)-$(2)
deploy-$(1)-$(2):
	@$$(MAKE) --no-print-directory deploy-$(1) ENV=$(2)
endef

define deploy_env_alias
.PHONY: deploy-$(1)
deploy-$(1):
	@$$(MAKE) --no-print-directory deploy ENV=$(1)
endef

$(foreach env,$(DEPLOY_ALIAS_ENVS),$(foreach step,$(DEPLOY_ALIAS_STEPS),$(eval $(call deploy_step_alias,$(step),$(env)))))
$(foreach env,$(DEPLOY_ALIAS_ENVS),$(eval $(call deploy_env_alias,$(env))))
