# dev-bots 信号处理收尾

> 状态：已确认，待实施
> 日期：2026-11-16

## 意图

`scripts/dev_bots.sh` 用前台方式执行 `sim dev`。bash 在前台命令运行期间不执行 trap，所以只收到 `SIGTERM`（被 `timeout` 包装、CI 取消、进程管理器发信号）时，脚本里的 `trap cleanup TERM` 要等 `sim` 自己结束才触发，`sim` 与 `room-service` 都继续跑，端口被占住。下次 `make dev-bots` 会立刻失败，而这是这个脚本唯一会立即失败的路径。

同时删除一个恒真的断言：`dev_bots_leaves_exactly_one_seat_for_the_developer` 断言 `DEV_BOTS == PLAYER_COUNT - 1`，而 `DEV_BOTS` 就是这么定义的，永远成立，无法失败。

## 目标与非目标

- 目标：任何终止信号（INT / TERM / HUP）到达时，`room-service` 与 `sim` 都在脚本退出前结束，端口释放。
- 目标：`sim` 的退出码原样透传给 `make dev-bots`。
- 目标：这三条路径有自动化测试，进 `make test`。
- 非目标：不改 `sim dev` 的 Rust 逻辑，不改 `make dev-bots` 的对外用法。
- 非目标：不追求 SIGKILL 兜底（SIGKILL 不可捕获，属操作系统边界）。

## 设计

### 脚本

`sim` 改为后台启动并记 `SIM_PID`，然后 `wait "$SIM_PID"` 取退出码。`cleanup` 同时关两个进程，并补 `HUP`：

```bash
trap cleanup EXIT INT TERM HUP
cleanup() {
  kill "$SIM_PID" "$SERVER_PID" 2>/dev/null || true
  wait "$SIM_PID" "$SERVER_PID" 2>/dev/null || true
}
```

`wait` 要放在 `set +e` 或 `|| true` 里，否则 `set -e` 会在 `sim` 非零退出时提前终止，透传的退出码就丢了。

`SERVER_BIN` 与 `SIM_BIN` 支持环境变量覆盖，默认仍是 `server/target/debug/` 下两个二进制：

- `DEV_SERVER_BIN`
- `DEV_SIM_BIN`

覆盖只用于测试注入桩程序，不改变 `make dev-bots` 的用法。

### 测试

新增 `tests/test_dev_bots_script.py`（标准库 unittest，与仓库其余 Python 测试一致）。每个用例在 `tmp_path` 等价的临时目录里生成两个桩：

- 服务端桩：一个 Python 脚本，绑定并 listen 指定端口后 sleep，直到被杀。
- `sim` 桩：一个 shell 脚本，sleep 指定秒数后以指定退出码退出。

三个用例：

| 用例 | 断言 |
| --- | --- |
| 端口被占用 | 先起一个占位进程占住端口，再跑脚本；退出码 1，stdout 含「端口 X 已被占用」与 `DEV_PORT=<X+1>` |
| SIGTERM 清理 | 端口空闲，起脚本，等「房间码」类输出不再出现后发 SIGTERM；脚本退出后，服务端桩端口已释放，桩进程消失 |
| 退出码透传 | `sim` 桩以 3 退出，脚本退出码为 3 |

脚本在测试里用 `subprocess.Popen(..., start_new_session=True)` 起，这样发信号能打到整组；发信号用 `os.killpg`。

## 删除的断言

`dev_bots_leaves_exactly_one_seat_for_the_developer` 整条删除。`DEV_BOTS` 的取值由 `PLAYER_COUNT` 定义式给出，重复断言它不产生任何保护；真正验证「留一个座位」的是同文件里满员开局那条端到端用例。

## 验收

- `make test` 退出码 0，新增三个用例全过。
- 手工：`make dev-bots` 后 `kill -TERM` 脚本进程组，`lsof -i :8080` 为空。
- 手工：`make dev-bots` 后 Ctrl-C 仍干净退出。
- 手工：端口占用时仍立即失败并提示换端口。
