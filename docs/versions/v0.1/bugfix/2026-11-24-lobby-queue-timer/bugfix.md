# 大厅排队卡等待时长一直停在 00:00

状态：已修复（本地冒烟通过，待开发环境客户端复验）

## 现象与复现

- 现象：开发环境点「开始匹配」后，大厅右栏排队卡的大号计时一直显示 `00:00`，下面的「已凑齐 N/8」正常变化。
- 复现：`make run-desktop-dev` 进入大厅点「开始匹配」，等待任意时长后计时不变。无头冒烟同样复现。
- 触发条件：经 `UILayout.safe_area` 收纳的页面里，排队卡嵌在中间容器内（非页面直接子节点）。

## 影响范围

- 只影响大厅排队卡的等待时长显示；排队、匹配、进对局均正常，人数胶囊正常。
- 同类风险：任何嵌在中间容器里、又在 `_ready` 中订阅信号的组件，经安全区收纳后会丢订阅。当前大厅只有排队卡属于这种结构。

## 根因

- `UILayout._adopt_children` 收纳页面直接子节点时执行 `remove_child` + `request_ready()` + `add_child`。`request_ready()` 只对被移动的那个直接子节点生效，其子树里的节点只收到 `_exit_tree`，再次进树时 `_ready` 不会重跑。
- `LobbyQueueCard` 在 `_ready` 里订阅 `GameStore.queue_changed`，位于被移动的中间容器子树内：收纳后订阅丢失，`_refresh` 不再被调用，`Timer` 停在未启动状态，文本永远停在入队时的 `00:00`。
- 计时基准本身也依赖服务端：原实现取最近一条 `QueueState.waitingMs` 并把「收到消息时刻」当基准，本地只补两次消息之间的漂移。开发服只在入队与人数变化时下发 `QueueState`（实测入队两条，`waitingMs` 为 0），后续完全静默；即使订阅正常，基准设计也把显示压在 0 附近。

## 修复方案

- 订阅时机：`LobbyQueueCard` 的订阅从 `_ready` 移到 `_enter_tree`，`_exit_tree` 断开，收纳重挂后自动恢复；进入树时立即 `_refresh()` 恢复显示。
- 计时基准：`GameStore` 记录本地入队时刻 `queue_started_at_msec`，`queue_wait_seconds()` 直接返回本地经过时间，服务端 `waitingMs` 不再参与显示；`QueueState` 只更新人数。
- 不改服务端与协议：`QueueState.waitingMs` 字段保留，客户端不再依赖。
- 兼容：无。开发阶段，不保留旧的基准算法。

## 验证方式

- 失败用例（修复前失败、修复后通过）：`make client-smoke SMOKE=lobby`，`_check_queue_timer` 断言入队后计时应开始走、服务端下发 `waitingMs=0` 不清零、收到快照后继续递增。
- 回归点：`make client-smoke SMOKE=lobby` 全量、`make client-check`。
- 复验：开发环境客户端点「开始匹配」后计时应从 `00:00` 起逐秒递增。
