# 首局引导默认关闭

> 状态：已实施
> 需求归属：v0.1
> 产品依据：[product.md](product.md)

## 1. 改动边界

- 只改客户端首局引导的打开时机与由此失效的本地记录、会话步骤恢复，不改步骤表、覆盖层结构与视觉。
- 协议、服务端、卡牌数据与其它页面不变。

## 2. 方案

- `client/pages/game/game.gd`：装配引导后不再调用 `start()`，进入对局保持隐藏；顶部「提示」仍调用 `show_manual()`。
- `client/pages/game/first_game_guide.gd`：
  - 删除 `start()`、`CONFIG_PATH`、`SEEN_SECTION`、`_has_seen()`、`_mark_seen()` 与 `manual` 标记。
  - `_evaluate()` 统一按原手动路径执行：阶段不匹配时切到当前阶段对应步骤，`GameOver` 时收起。
  - `_finish()` 只清 `step` 并隐藏，不写本地文件。
  - 删除仅自动路径使用的 `_condition_met()` 与步骤 `condition` 字段。
- `client/autoload/game_store.gd`：删除 `guide_step` 字段与两处重置。
- `client/tests/ui_smoke_guide.gd`：按「默认隐藏 + 手动打开」重写断言，删除 `seen`、`guide_step` 恢复与条件跳步场景。
- `client/tests/ui_smoke.gd`：删除 `_dismiss_guide()` 辅助及其两处调用，引导默认隐藏后不再需要先卸层。

## 3. 兼容与影响

- `user://first_game_guide.cfg` 不再读取或写入；旧文件留在本机不影响运行。
- `FirstGameGuide`、`show_manual()`、`set_phase()`、`hide()` 与内部节点名保持不变，其它冒烟用例按节点名查找不受影响。

## 4. 验证

- `make client-check`：无脚本解析错误。
- `make client-smoke`：全部场景 `UI_SMOKE_OK`，无 `SCRIPT ERROR`。
- `python3 scripts/validate_docs.py`、`python3 scripts/check-runbooks.py`：退出码 0。
- 单主题（仅深色），不做浅色与多主题检查。
