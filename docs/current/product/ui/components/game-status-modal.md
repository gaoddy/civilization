# 对局状态模态

> 状态：已实现
> 入口：`GameStatusModal`，`client/pages/game/game_status_modal.gd`
> 使用：对局页 `game.gd` 创建并调用 `show_status` / `hide_status`

## 1. 用途

对局页唯一的状态提示载体：等待视图、断线 / 重连、操作未发出与服务端错误。居中显示提示文案与可选协议原文，带「知道了」手动关闭。提示不锁对局：遮罩与图层都忽略鼠标，指针与拖拽继续落到下层对局控件。

## 2. 结构

```text
PopupLayer（CENTER，黑 0.55）
├─ PopupScrim：忽略鼠标，只压暗
└─ PopupBody
   └─ StatusPanel（card_panel raised）
      └─ 内边距 space-lg
         ├─ StatusMessage：主文案，font-body，自动换行并水平居中
         ├─ StatusDetail：协议原文等次要说明，font-caption，color-text-muted，空时隐藏
         └─ 操作行
            └─ StatusConfirm：PillButton primary「知道了」
```

## 3. 接口

| 成员 | 说明 |
| --- | --- |
| `show_status(text, tone := "dim", detail := "")` | 写入文案与色调并显示；`tone` 取 `dim` / `danger` / `text` |
| `hide_status()` | 清空文案并收起 |
| `acknowledged` | 点击「知道了」后发出；页面据此清空当前提示来源 |
| 面板宽度 | `status-dialog-width`（360）；iPhone `status-dialog-width-compact`（280） |

## 4. 生命周期

- 由 `game.gd` 控制：错误与「未发出」停留 `ERROR_NOTICE_SECONDS`（4 秒）；等待视图、断线与重连保持到状态恢复。
- 连接提示只在连接状态变化时弹出；手动关闭后同一连接状态不重复弹出。
- 错误到时或阶段 / 回合变化清空时，若仍断线则恢复连接提示。
- 模态为单实例复用，只切换文案与显隐，不随视图刷新重建。

## 5. 层级

- 位于选择浮层、图鉴覆盖层与详情层之上；首局引导是 `CanvasLayer`（`layer = 100`），层级高于本模态。
- 不拦截对局输入；只有提示面板自身接收点击，「知道了」按钮可操作。

## 6. 验证

- `make client-smoke`：`ui_smoke` 覆盖错误写入、4 秒收起、回合变化提前收起与未发出提示；`ui_smoke_side` 断言模态存在且默认隐藏。

## 7. 关联

- 弹出层外壳：[popup-layer.md](popup-layer.md)
- 胶囊按钮：[pill-button.md](pill-button.md)
- 对局页：[../pages/game.md](../pages/game.md)
