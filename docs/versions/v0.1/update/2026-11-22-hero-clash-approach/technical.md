# 英雄对冲靠岸与克隆卡渲染（客户端）

> 状态：已实施
> 需求归属：v0.1
> 产品依据：[product.md](product.md)

## 1. 改动边界

- 只改客户端演出：`CombatHeroClash` 的时间轴与靠岸几何、英雄克隆卡的生成方式。
- 不改协议、服务端、战斗动作播放、胜负判定与 `CombatPlayed` 回报条件。

## 2. 克隆卡根因

`AvatarFrame` 在 `_init()` 里创建 `AvatarFrameClip`、`AvatarFrameBorder`、`AvatarFrameContent` 等子节点。`Node.duplicate()` 会先在副本上执行一遍 `_init()`，再把原件子节点复制进来；两份同名子节点里，原件复制层保留原名，`_init()` 新建的默认子层被改成自动名并排在后面，于是默认封面与默认描边盖在复制出的头像与描边上，看起来就是「移动中的英雄没有头像、没有边框」。

## 3. 修复方案

- `GameHeroCard.replica()`：按 `hero_id`、`hp`、`hp_cap` 重建一张同状态英雄卡，供演出克隆使用。
- `CombatFx._copy_card()`：`GameHeroCard` 走 `replica()`，其余卡面继续 `duplicate()`；`CombatFx._clone_card()` 的命名、尺寸、鼠标忽略与透明度设置不变。

## 4. 时间轴与几何

| 常量 | 值 | 含义 |
| --- | --- | --- |
| `APPROACH_MS` | 650 | 从原位缓慢移到河道两岸，`smoothstep` 平滑曲线 |
| `BREATH_MS` | 120 | 靠岸后蓄势停顿 |
| `STRIKE_MS` | 140 | 加速冲进河道中央（`k²`） |
| `PUSH_MS` | 140 | 胜方顶进、败方后撤 |
| `HOLD_MS` | 80 | 顶退后停顿 |
| `RETURN_MS` | 300 | 回到原位 |
| `HIT_MS` | 910 | 演出开始到相撞，供冒烟按帧推进 |
| `TOTAL_MS` | 1430 | 演出总时长 |

- 靠岸位置：我方卡顶边贴河道下沿下方 `BANK_GAP_RATIO`（0.08 卡高）；对手卡底边贴河道上沿上方同一间隙。
- 相遇位置保持河道中心上下贴合，命中点仍在河道中心，伤害飘字一侧判定不变。

## 5. 验证

- `make client-smoke SMOKE=combat`：`UI_SMOKE_OK`，退出码 0；新增克隆卡头像、描边与子节点层数断言。
- 真实窗口抓帧目检（临时场景，验证后删除）：克隆卡头像与描边完整；靠岸帧停在河道两岸；命中帧在河道中央；回位帧恢复原位。
- `make client-check`：退出码 0。
