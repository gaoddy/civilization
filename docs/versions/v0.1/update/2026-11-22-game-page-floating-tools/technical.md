# 对局页悬浮工具组与状态模态

> 状态：已实施
> 需求归属：v0.1
> 产品依据：[product.md](product.md)

## 1. 改动边界

- 只改对局页布局入口、状态提示载体与共享胶囊按钮；不改协议、服务端、卡牌数据、其它页面与拖拽 / 待选 / 战斗演示逻辑。
- 保留图鉴、提示、退出对局三个信号与动作；保留原有错误停留 4 秒、阶段 / 回合变化提前清空与断线优先的规则。

## 2. 方案

- `client/ui/pill_button.gd`（新增）：`PillButton` 胶囊按钮，`neutral` / `primary` / `danger` 变体；高度取 `pill-button-height`，触控端不低于 `hit_small()`。
- `client/ui/theme.gd`：新增 `pill_button_style(variant, state)` 与 `pill-button-height` 26、`pill-button-height-compact` 24、`status-dialog-width` 360、`status-dialog-width-compact` 280；删除 `top-bar-height` 与 `top-bar-height-compact`。
- `client/pages/game/game_tool_buttons.gd`（新增）：`GameToolButtons`，横排三枚 `PillButton`，输出 `gallery_pressed` / `hint_pressed` / `leave_game_pressed`；`set_connected()` 只禁用提示与退出对局。
- `client/pages/game/game_status_modal.gd`（新增）：`GameStatusModal` 继承 `PopupLayer`，居中提示文案、可选协议原文与「知道了」按钮；遮罩 `mouse_filter = IGNORE`，不拦截对局输入。
- `client/pages/game/game.gd`：
  - 删除 `GameTopBar` 与顶部纵向根容器；主体直接铺入页边距。
  - `_build_tools()` 在页面叠加 `GameToolsOverlay`（`MarginContainer`，全屏忽略输入），工具组右对齐并让出页面右边距与上边距。
  - 状态提示统一走 `_show_notice()` / `_hide_notice()`；`_notice_kind` 记录 `waiting` / `connection` / `error` / `intent`。
  - `_refresh()` 在空视图且当前无提示时显示等待提示，收到视图后收起等待提示。
  - `_refresh_connection()` 只在连接状态变化时弹出；空视图阶段不显示连接提示。
  - `_tick_error_notice()` 覆盖错误与未发出，到时收起；断开时恢复连接提示。
  - `_on_notice_acknowledged()` 处理「知道了」，断线时关闭错误或未发出后恢复连接提示。
  - `_on_error()` 把协议原文作为模态次要说明，相同文案不重复显示。
- `client/pages/game/game_top_bar.gd`、`game_top_bar.gd.uid`：删除。
- `client/text/zh_CN.csv`：新增 `ui.common.got_it`（知道了）。
- `client/tests/ui_smoke.gd`、`ui_smoke_side.gd`、`ui_smoke_layout.gd`：按新节点名与提示模态行为改写断言。

## 3. 兼容与影响

- `GameTopBar` 与 `StatusLabel` 不再存在；对局页不再有任何标签形式的状态载体。
- 状态提示遮罩不拦截输入，保留原状态文本的非阻塞行为；提示面板本身接收点击，只有「知道了」按钮可操作。
- 连接提示在空视图阶段不显示，收到视图后才按连接状态弹出，与原有「空视图时显示等待状态」一致。
- 其他页面（登录、房间、大厅）的 `StatusLabel` 不受影响。
- 仅深色主题；无浅色或多主题检查。

## 4. 验证

- `make client-check`：协议、数据、客户端边界 / 风格 / 文案检查与 `--import` 退出码 0。
- `make client-smoke`：全部场景 `UI_SMOKE_OK`，无 `SCRIPT ERROR`。
- `python3 scripts/validate_docs.py`、`python3 scripts/check-runbooks.py`：退出码 0。
- 单主题（仅深色），不做浅色与多主题检查。
