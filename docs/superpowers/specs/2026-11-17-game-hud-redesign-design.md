# 对局页 HUD 改版 设计

> 状态：待确认
> 需求归属：v0.1
> 实施方式：分两步，先数据后端，再客户端表现（用户选定方向 B）
> 附带产品改动：落单座位由「自战镜像」改为「对战一名随机出局者的幽灵阵容」，并正常扣血（§5.2、§7.7）

## 1. 意图与原因

当前对局页把同一份信息重复了多遍：顶栏的生命/时代/金币与右栏属性行重复；右栏战报混着事件流；积分榜每个座位都写满「HP · 时代 · 随从数」。改版要把页面收敛成三条清晰的视线：

- **左栏看别人**：一列头像，各自的血量与名次，点开看这名玩家的战报。
- **右栏看自己这一场**：顶部是当前场景的身份（招募时的售货员、战斗时的对手英雄），中部是回合与倒计时，底部是自己的英雄。
- **中区只做操作**：商店、棋盘、手牌，战斗时换成双方随从。

## 2. 目标

- 顶栏整条移除，生命/时代/金币只在自己英雄介绍里出现一次。
- 事件流（购买、刷新、升时代、三连、伤害、淘汰）不再有文字呈现：`GameStore.event_log` 与右栏战报面板一并删除，反馈只由界面状态变化本身承载。
- 积分榜改为头像 + 名次角标 + 血量条，不显示名字与数值文本，去掉「积分榜」小标题。
- 点座位行在其下方展开该玩家参与过的战报，同屏只展开一个；战报数据由服务端下发，重连不丢。
- 右栏顶部按阶段切换：招募阶段显示售货员形象，战斗阶段显示对方英雄。
- 右栏中部显示第几回合与剩余时间；底部显示自己英雄介绍。
- 战斗视图删掉上下的双方英雄条，只留双方随从与结果横幅。
- 商店卡左上角显示购买费用、右上角显示时代；冻结状态用蒙层 + 冰晶纹理表达。
- 名次由服务端计算，客户端不做任何排序推导。
- 存活人数为奇数时，落单座位对战**一名随机出局者的幽灵阵容**，必有对战：输了正常扣血，赢下计入胜者（§5.2）。

## 3. 非目标

- 不改战斗结算算法、配对规则与伤害公式。
- 不改商店抽取逻辑；卡牌 `era` 只是展示字段，不参与抽取。
- 不改英雄技能与结束招募两个按钮的语义，只调整位置。
- 不做移动端适配（属 v0.2）。
- 不引入图片、字体或主题资源；一切仍由 Godot 控件与 `StyleBoxFlat` 构建。
- 不给积分榜做排序切换、筛选或折叠全部等额外交互。
- 不新增事件流替代物（如 toast、飘字、消息滚动条）；事件文字反馈就此移除。
- 不改配对算法本身：洗牌、相邻成对、排除上一场对手、最多重洗 100 次，全部保留；只改落单座位对战的对象与结算口径。

## 4. 反目标

- 客户端自行按生命排序冒充名次。
- 为凑血条比例在客户端硬编码 40。
- 用颜色单独表达冻结、存活、己方等状态而不给文本或图形冗余。
- 把战报做成整场流水账（用户明确要按座位展开）。

## 5. 约束

### 5.1 事实约束（已核对代码与文档）

| 约束 | 依据 |
| --- | --- |
| `placement` 局内恒为 `null` | `server/crates/rule-engine/src/view.rs:483`，并由 `placement_is_hidden_before_game_over` 断言 |
| 局内 `ranking` 是「已出局者在前」的临时块，下标 +1 不等于名次 | 同上注释 |
| 真实排序能力在 `settle.rs::compare_alive`：生命 → 上战胜者 → 累计伤害 → 座位 | `server/crates/rule-engine/src/settle.rs:655` |
| 客户端拿不到 `damage_dealt` 与 `last_won` | `PlayerSummaryView` 不含这两个字段 |
| 配置里 `initial_hp = 40`，对局中生命只减不增 | `server/data/v0.1/config.json`、`settle.rs` 只有 `saturating_sub` |
| 服务端只把 `CombatResult` 发给自己那一场，其他座位看不到 | `room-service/src/game.rs::combat_updates` |
| 36 张效果牌在卡表里没有 `era`，规范写明「仅随从」 | `scripts/validate_docs.py:93`、`card-system.md` |
| 仓库没有任何图片资源 | `client/` 下无位图或字体文件 |
| 落单座位目前自战「自己上一场阵容」的幽灵，且不掉血 | `settle.rs::resolve_mirror`、`core-rules.md:187`、测试 `mirror_loss_does_not_damage_or_streak` |
| 已出局玩家的 `board` 不会被清空，也不会再被修改 | `engine.rs:135/165/197/203` 全部意图对 `eliminated` 早退；`clear_combat_duration` 只清 `turn` 层与临时关键词，不动 `permanent` |
| `ranking` 是全部出局者的全集，按出局新旧排列 | `settle.rs::resolve_settle` 把本回合新出局者 `rev()` 后前插，`ranking` 长度随出局人数增长、出局者不会被移除 |
| 真实战的胜负与伤害逻辑集中在 `resolve_real` 尾部，可复用 | `settle.rs:300-352`：`tie_break` → `damage_formula` → `pending_settle` → `last_won` |
| 光环只由 `state.auras` 派生，而入表只收未出局玩家 | `effects/aura.rs:22-50`；持久棋盘本身不带光环层（`set_aura` 只在 `fold_auras` 里调用），所以出局者的幽灵要显式重算一次它的光环来源，否则以零光环参战 |

### 5.2 落单座位：从「自战镜像」改为「对战已出局者」

本轮要对齐炉石传说酒馆战棋（调研见 §5.3）。现行规则是落单座位对**自己上一场阵容**的镜像幽灵，且失败不扣血（`core-rules.md:187`）。改成：

| 项 | 决定 |
| --- | --- |
| 幽灵来源 | **从全部出局者里随机选一名**，取其最终阵容。幽灵只从出局者里取，**不再自战自己**；随机走 `Stream::Matchmaking`，每回合只抽一次，保证重放一致 |
| 没有出局者 | 该座位照样对战：幽灵为空阵容（无随从），配对、模拟、战报与结算都照常。真实对局到不了这个状态（见下），只有测试能构造 |
| 落单必战 | 奇数人数时落单座位**永远产生配对与战报**，不出现「本回合不战」 |
| 阵容口径 | 原样使用该玩家 `player.board`：`turn` 层在出局那次结算已被 `clear_combat_duration` 清掉，`permanent`/`growth` 是真实数值 |
| 输给幽灵 | **正常扣血**，伤害走 `damage_formula`：败者时代取落单座位自己的时代，星级取幽灵存活随从——与 `resolve_real`「败者时代 + 胜方存活随从星级」的口径一致，不照搬炉石「对手酒馆等级 + 存活随从星级」的公式 |
| 赢下幽灵 | 计入 `last_won` 与 `last_winner`，参与「上战胜者」名次比较；不累加 `damage_dealt`（本场没有打出真实伤害） |
| 幽灵侧副作用 | 不结算英雄被动（`defender_hero: None`）、不写回牌池与持久棋盘、不产生手牌、不写 `previous_opponent` |
| 幽灵光环 | 开战时对幽灵棋盘单独重算一次光环，只在本次模拟的克隆上生效，不写回 `state.auras` |
| 双方同回合清场 | **平局**：双方都不扣血、都不计入胜者（`last_won` 两边都不写），战报 `winner` 为 `null`、`damage` 为 0。真实战与幽灵战同一口径，不再按生命/累计伤害/座位分出胜者 |
| 跨回合重复 | 不做「3 回合内不重复」限制：每回合独立随机抽取，可能连续抽到同一名出局者 |
| 配对洗牌 | 洗牌不消耗额外流：幽灵抽取在洗牌结束后用同一条 `Stream::Matchmaking` 流再抽一次 |

「没有出局者」只存在于测试构造的状态：`resolve_settle` 每回合都把生命 ≤ 0 的人标为出局并写入 `ranking`；断线玩家不会被移出对局（`game.rs::expire_disconnects` 只改在线状态），所以真实对局里存活人数为奇数时，`ranking` 至少有一个人。

#### 一并删除的旧机制

| 删除 | 位置 |
| --- | --- |
| `mirror_ghost`、`mirror_ghost_with_auras` | `settle.rs:455-480` |
| `strip_ghost` 及其「只保留永久属性」的用例 | `settle.rs:517+`、`tests/settle.rs` 的 `mirror_ghost_keeps_only_permanent_stats` |
| 上一场快照回退的用例 | `tests/settle.rs::mirror_ghost_falls_back_to_current_board`、`mirror_ghost_keeps_growth_breed_and_contagion`、`tests/auras.rs::mirror_ghost_from_previous_board_includes_auras` |
| `Pairing.mirror` | 换成 `Pairing.ghost: Option<Seat>`（§7.7.1） |
| `GameState.previous_board` | 只写不读的死状态（`state.rs:332`、`engine.rs:66`、`hash.rs:58-59`、`settle.rs:312-313`），连同 `hash` 里的 3 行与 `tests/combat.rs:890-891`、`tests/settle.rs:285-286` 断言一起删除 |

`state.previous_board` 的唯一读者就是旧的自战镜像，删除镜像后它只写不读（`settle.rs:312-313,395`、`hash.rs:58-59`、`state.rs:332`、`engine.rs:66`），一并删除。

### 5.3 炉石传说酒馆战棋的实现（调研）

来源：`hearthstone.wiki.gg/wiki/Battlegrounds`（Kel'Thuzad 条目）。

- 存活人数为奇数时，每回合有一名玩家对上「克尔苏加德」：一个没有英雄技能、没有实际生命的空壳英雄。
- 空壳的阵容与酒馆等级复制自**一名已被淘汰的玩家**。27.2 补丁（2023-05-09）前随机挑一名出局者，之后固定为**最近出局者**。
- 空壳没有血量可扣，但**活着的一方输了照样掉血**，按常规公式（对手酒馆等级 + 存活随从星级）。
- 与同一名对手（含空壳）在 3 个战斗回合内不会重复相遇，除非只剩 2 人。
- 在招募阶段退出对局的人，其阵容会立刻成为空壳的素材。

本项目的差异与取舍：

- 幽灵来源改为**全部出局者中随机一名**，不沿用炉石 27.2 之后的「最近出局者」。
- 不做 3 回合不重复限制。
- 幽灵没有英雄被动，`defender_hero: None` 已经等价于「空壳没有英雄」。
- 掉血沿用本项目公式（败者时代 + 胜方存活随从星级），不照搬炉石公式。

## 6. 做法比较

| 方向 | 做法 | 代价 | 结论 |
| --- | --- | --- | --- |
| A 一次做完 | 协议、引擎、卡牌数据、客户端 UI 在同一个分支一次交付 | 跨四层的大 diff，出问题难定位，评审只能整体看 | 否 |
| **B 两步顺序做** | 第一步交付 `hpCap`、`rank`、战报历史、效果牌时代；第二步在就绪的数据上重排对局页 | 需要两次验收 | **选定** |
| C 先 UI 后数据 | UI 先用兜底值（血条按 40、名次按座位序、战报只显示自己那场）跑起来 | 兜底逻辑写两遍再删；中途看到的名次是错的 | 否 |

B 的每一步都有独立可验收的完成标准：第一步靠协议样例、引擎测试与 `make protocol-check` / `make test`；第二步靠 `make client-smoke`。

## 7. 服务端与协议（第一步）

### 7.1 `PlayerView.hpCap`

```json
"hpCap": { "type": "integer", "minimum": 1, "description": "生命上限，v0.1 为 40" }
```

- 加入 `required`。取值来自 `catalog.config.initial_hp`，与 `boardSlots` 同一个位置、同一种传法。
- 客户端在字段缺失时回退 40，兼容旧服务端。
- 选择放在视图顶层而不是 `GameStarted`：重连只回 `Snapshot`，视图每次都带。

### 7.2 `PlayerSummary.rank`

```json
"rank": { "type": "integer", "minimum": 1, "maximum": 8, "description": "当前名次；对局结束前为实时名次，结束后等于 placement" }
```

- 加入 `required`。
- 引擎新增 `rank_of(state, seat) -> Option<u8>`，放在 `settle.rs`，与 `compare_alive` 同模块：

```rust
pub fn rank_of(state: &GameState, seat: Seat) -> Option<u8> {
    if state.phase == Phase::GameOver {
        return placement_of(state, seat);           // 结算后与 placement 完全一致
    }
    let player = player(state, seat)?;
    if player.eliminated {
        // 已出局者占据尾部名次：存活数 + 榜内下标 + 1
        let alive = state.players.iter().filter(|p| !p.eliminated).count();
        let index = state.ranking.iter().position(|s| *s == seat)?;
        u8::try_from(alive + index + 1).ok()
    } else {
        // 存活者：1 + 比它强的存活者数，比较器与第 20 回合收束完全一致
        let better = state.players.iter()
            .filter(|p| !p.eliminated && p.seat != seat)
            .filter(|p| compare_alive(state, p.seat, seat) == Ordering::Less)
            .count();
        u8::try_from(better + 1).ok()
    }
}
```

- `compare_alive` 现为私有函数，`rank_of` 同模块可直接调用，不需要改可见性。
- `PlayerSummaryView` 新增 `rank: Option<u8>`，`view.rs::player_summary` 填入。
- 名次在局内公开是**产品规则变化**（现在刻意隐藏），写入需求 `product.md`。

#### 名次在战斗演示期间是战前值

`resolve_combat` 只把伤害写进 `state.pending_settle` 与 `damage_dealt`，真正扣血在 `resolve_settle`（`settle.rs:584`）；`enter_combat` 期间阶段停在 `Combat`，结算推迟到 `finish_combat`（`game.rs:571-600`）。

因此战斗演示播放时，左栏的名次与存活状态保持战前值，演示结束、进入下一回合招募后一并刷新。这是有意为之：演示期间如果血量与名次提前跳到最终值，会与正在播放的动作序列矛盾。`rank_of` 不读 `pending_settle`，也不需要在演示期间特殊处理。

#### 战斗阶段的对手身份

改动 §7.7 之后，`opponentPlayerId` 只有三种取值：

| 场景 | `opponentPlayerId` |
| --- | --- |
| 真实战 | 对手座位 |
| 幽灵战（§7.7） | **幽灵归属的那个已出局座位** |
| 已出局者观战、或落单但无出局者（不战） | `null` |

已淘汰座位不参与配对（`settle.rs:102-104`），但服务端仍把战斗阶段视图推给全部座位（`game.rs` 的 `all_seats` / `view_updates`），所以观战者的 `opponentPlayerId` 是 `null`。

客户端不能只凭 `opponentPlayerId == null` 判定身份，右栏顶部的判定顺序：

1. 自身已淘汰 → 「已淘汰 · 观战中」，不显示对方英雄；
2. `GameStore.last_combat.mirror` 为真且 `opponentPlayerId` 有值 → 显示该座位的英雄卡片 + 「幽灵」角标；
3. `opponentPlayerId` 有值 → 显示对方英雄；
4. 其余（已淘汰观战、或本回合没有配对）→ 「等待配对」。

幽灵战的对手英雄从 `view.players` 里按 id 找，那名玩家 `alive` 为假，卡片按已淘汰样式压暗，并在名字后标「幽灵」。不显示其血量条（幽灵没有可扣的血）。

`CombatResult.mirror` 仍为 `true`，含义从「对战自己的镜像」变为「对战幽灵阵容」；`opponentBoard` 现在是那名已出局者的最终阵容。

### 7.3 战报历史 `PlayerView.combatLog`

```json
"combatLog": {
  "type": "array",
  "description": "本局最近 5 个回合的全部战斗，按回合升序；全对局公开",
  "items": {
    "type": "object",
    "additionalProperties": false,
    "required": ["turn", "participantIds", "winnerId", "loserId", "damage", "mirror"],
    "properties": {
      "turn": { "type": "integer", "minimum": 1, "maximum": 20 },
      "participantIds": {
        "type": "array",
        "minItems": 1,
        "maxItems": 2,
        "items": { "type": "string" },
        "description": "参战玩家 id；幽灵战只有对战方一个元素（幽灵不是玩家）"
      },
      "winnerId": { "type": ["string", "null"], "description": "平局无胜者时为 null" },
      "loserId": { "type": ["string", "null"], "description": "幽灵战失利时为该玩家；幽灵不受伤，所以幽灵输时也为 null" },
      "damage": { "type": "integer", "minimum": 0 },
      "mirror": { "type": "boolean", "description": "true 表示对战已出局者的幽灵阵容" }
    }
  }
}
```

- 加入 `required`；旧服务端缺失时客户端按空数组处理。
- `participantIds` 是必须的：幽灵战里幽灵没有玩家 id，只有对战方能说明这条战报属于哪个座位；真实战两个座位都在里面。座位展开按 `participantIds` 是否包含该座位来筛选。
- 幽灵战的 `winnerId`/`loserId`：打输时 `loserId` 是对战方、`winnerId` 为 `null`（幽灵赢但没有 id）；打赢时 `winnerId` 是对战方、`loserId` 为 `null`（幽灵不掉血）。
- 存放位置：`GameTask` 新增 `combat_log: Vec<CombatLogEntry>`，每次 `enter_combat` 从 `outcome.reports` 去重后追加。**不写入 `GameState`**，因此不影响 `state_hash` 与确定性重放。
- 去重：一份真实战在 `reports` 里有两个座位各一条，按 `(turn, winner, loser)` 归一，每场只记一条；幽灵战本来就只有一条。
- 截断：只保留最近 5 个回合的条目，超出时按 `turn` 丢弃最旧的。`turn_limit` 为 20，每回合存活 8 人时产生 4 场战斗，5 回合最多 20 条。
- 下发：`player_view` 在 `to_protocol` 之后填 `view.combat_log = self.combat_log.clone()`。重连走 `Snapshot`，历史自然带出。

### 7.4 效果牌补 `era`

- `scripts/validate_docs.py`：`era` 从「仅随从」改为随从与效果牌都必填；`EFFECT_FIELDS` 加入 `era`；`expected_card_attrs` 的效果牌分支输出「时代」行；新增效果牌时代分布断言。
- 36 张效果牌按「## 出处」的年代归入七时代，映射表见 §10，逐条可在实现时改。
- 效果牌 `era` 只用于展示：商店抽取只看 `level`、`tribe` 与共享池（`recruit.rs::is_eligible`），不读 `era`。
- 同步修订规范文档，否则 `card-system.md` 的「仅随从」与校验脚本直接矛盾：
  - `docs/versions/v0.1/product/content/card-system.md`：`era` 字段说明改为两类牌都必填，校验覆盖一行补「效果牌时代分布」。
  - `docs/versions/v0.1/product/content/cards/README.md`：效果牌的「数值」列补上时代。
- 生成物 `server/data/v0.1/cards.json` 与 `client/generated/cards.json` 由 `make data` 重新生成。

### 7.5 卡表 `era` 是中文名，不是序号

`client/generated/cards.json` 的 `era` 直接是「工业」「未来」这样的中文名，不是 1–7 的整数（`client/ui/ui.gd:155` 现有的用法也直接读字符串）。客户端显示卡牌时代时**直接用 `entry.era`**，不经过 `CardDB.era_name`；`era_name(int)` 只用于玩家时代（`self.era` / `player.era` 是整数）。

### 7.6 协议与样例

- `protocol/generated/protocol.rs`、`client/generated/protocol.gd`、字段表由 `make protocol` / `make protocol-rs` 重新生成。
- 含 `PlayerView` 的样例（`Snapshot`、`StateUpdate`、`StateUpdatePendingChoices`、`StaleStateUpdate`、`GameStarted`）补 `hpCap`、`combatLog`，含 `PlayerSummary` 的补 `rank`。
- `server/crates/sim/tests/sim_bot.rs:15` 用手写 JSON 反序列化 `PlayerView`，而生成的类型是 `deny_unknown_fields`，缺必填字段会直接失败。该文件的 `view()` 辅助函数必须补 `hpCap`、`combatLog`（`players` 为空数组，不涉及 `rank`）。客户端 `client/tests/fixture.gd` 的 `view()` / `game_view()` 同理，必须补 `hpCap`、`combatLog`，并给 `players` 补 `rank`。

### 7.7 幽灵战：引擎改动

对应 §5.2 的决定。改动集中在 `settle.rs`、`state.rs`、`view.rs`、`effects/aura.rs`、`hash.rs`，**不改配对算法本身**。

#### 7.7.1 `Pairing` 携带幽灵座位

```rust
pub struct Pairing {
    pub a: Seat,
    pub b: Seat,
    /// 幽灵战的幽灵归属座位（已出局者）；真实战为 `None`。
    pub ghost: Option<Seat>,
}
```

- `mirror` 字段被 `ghost.is_some()` 取代，不再依赖「`b == a` 即镜像」的隐式约定。
- 落单座位仍由 `pair_adjacent` 产生（`Pairing { a: seat, b: seat, ghost: None }`）；`pair_round` 拿到结果后补 `ghost`：

```rust
/// 幽灵归属座位：从全部出局者里随机抽一名，走 `Stream::Matchmaking`。
/// `ranking` 是「出局顺序」（最近出局在前），作为候选全集；
/// 没有出局者时返回 `None`，落单座位对战空阵容——真实对局不可达（见 §5.2）。
fn ghost_seat(state: &mut GameState) -> Option<Seat> {
    if state.ranking.is_empty() {
        return None;
    }
    let bound = state.ranking.len() as u64;
    let index = state.rng.below(Stream::Matchmaking, bound) as usize;
    state.ranking.get(index).copied()
}
```

- 补 `ghost` 的动作发生在 `pair_round` 的两条返回路径上（`settle.rs:117`、`120`），统一在函数末尾处理。抽幽灵**只在存在落单配对时才消耗随机流**，不影响有人数偶数时的洗牌序列：

```rust
let mut pairings = last;
let ghost = if pairings.iter().any(|pairing| pairing.a == pairing.b) {
    ghost_seat(state)
} else {
    None
};
for pairing in &mut pairings {
    if pairing.a == pairing.b {
        pairing.ghost = ghost;
    }
}
pairings
```

- `ghost = None` 的落单配对仍然保留：幽灵是空阵容，落单座位照常对战、照常出战报。

- `repeats_previous_opponent` 把 `pairing.mirror` 换成 `pairing.ghost.is_some()`。
- `Pairing` 参与 `state.pairings` 与 `EngineError` 的 Debug 输出；`hash.rs:76` 的 `hasher.bool(pairing.mirror)` 换成「有无幽灵 + 幽灵座位」。这会让历史 `state_hash` 与旧对局不兼容，但本版本没有需要重放的存量对局。

#### 7.7.2 幽灵阵容的来源

```rust
/// 幽灵阵容：幽灵归属座位（已出局）的最终棋盘，按它自己的光环来源折叠一次。
/// 只在本次模拟的克隆上生效，不写回 `state.auras`。
/// `None`（没有出局者）与未知座位都返回空阵容。
pub fn ghost_board(engine: &Engine, state: &GameState, ghost: Option<Seat>) -> Vec<MinionInstance> {
    let Some(corpse) = ghost.and_then(|seat| player(state, seat)) else {
        return Vec::new();
    };
    let auras = player_auras(engine, state, corpse);
    fold_auras(&engine.catalog, &auras, &corpse.board)
}
```

- **不做 `strip_ghost`**：`turn` 层已在出局那次结算被 `clear_combat_duration` 清掉，`permanent`/`growth` 是死时真实数值。
- 光环必须显式重算：`recompute_auras` 只遍历未出局玩家并整体替换 `state.auras`（`effects/aura.rs:22-50`），出局者棋盘上的实例不在里面，持久棋盘本身又不带光环层（`set_aura` 只在 `fold_auras` 里调用）。直接 `corpse.board.clone()` 会让幽灵比死时弱一截（它自己的光环随从不再生效），与「最终阵容」不符。
- 做法：把 `recompute_auras` 循环体（`aura_sources` → `aura_active` → `aura_targets` → `accumulate`）抽成 `player_auras(engine, state, &PlayerState) -> BTreeMap<InstanceId, AuraEffect>`，`recompute_auras` 改为对每个未出局玩家调用后合并。这套计算只看 `player.board` 与英雄卡、不消耗随机流，对已出局玩家同样可用。
- 幽灵来自其它座位时实例编号全局唯一（`next_instance_id` 单调递增），`renumber_ghost` 不会触发改号；保留调用以防将来复制同座位阵容。

#### 7.7.3 `resolve_ghost` 改为结算真实伤害

`resolve_mirror(engine, state, seat, ...)` 换成 `resolve_ghost(engine, state, pairing, ...)`，从 `pairing` 取 `a`（落单座位）与 `ghost: Option<Seat>`。幽灵阵容用 §7.7.2 的 `ghost_board(engine, state, pairing.ghost)`，攻方棋盘仍是落单座位折叠当前光环后的棋盘。`ghost` 为 `None` 时幽灵是空阵容，其余流程不变。

```rust
// 双方同回合清场一律平局：既不判幽灵赢，也不判存活座位赢。
let cleared = log.attacker_survivors.is_empty() && log.defender_survivors.is_empty();
let ghost_won = !cleared && log.attacker_survivors.is_empty();
// 星级和取「幽灵棋盘 × defender_survivors」，口径同 resolve_real 的「胜方战前棋盘 × 胜方存活」。
let stars = ghost
    .iter()
    .filter(|minion| log.defender_survivors.contains(&minion.instance_id))
    .map(|minion| card_level(catalog, &minion.card_id))
    .fold(0u32, u32::saturating_add);
let damage = if ghost_won {
    damage_formula(&catalog.config, player_era(state, seat), stars)
} else {
    0
};
if ghost_won {
    state.pending_settle.insert(seat, damage);
} else if !cleared {
    state.last_won.insert(seat);
    state.last_winner = Some(seat);
}
```

- 平局判据只有一条：双方同回合清场（两侧 `survivors` 都为空）。它优先于「幽灵侧清场即胜」——双方清空时既不扣血也不计胜者。`combat::simulate` 在双方清场时给 `winner = None`，幽灵战不使用它的 `winner` 字段，因此不会与它冲突。
- `tie_break` 不用于幽灵战，也不用于任何互清情形。
- 报告：`opponent_seat: ghost`、`mirror: true`、`damage` 为实际伤害、`winner` 为 `Some(seat)`（赢）、幽灵座位（输）或 `None`（平局）。
- `GameEvent`：赢为 `CombatResolved { winner: seat, loser: ghost.unwrap_or(seat), damage: 0 }`，输为 `CombatResolved { winner: ghost.unwrap_or(seat), loser: seat, damage }`，平局为 `CombatDrawn { a: seat, b: seat }`。这两个事件在 `filter.rs:134` 被丢弃，不进入协议事件流。
- **不写** `previous_opponent`（幽灵不参与下一回合配对）；**不累加** `damage_dealt`（`compare_alive` 的第三键是「累计造成的伤害」，幽灵战没有造成真实伤害，落单座位不应因此获得名次加权）。
- 幽灵侧 `defender_hero: None`、`pool: None`、`defender_hand: 0`，保持现状。
- 攻击方仍是落单座位，`first` 固定为它（幽灵没有「上战胜者」资格，不参与先手判定）。

#### 7.7.4 随机抽取的确定性与分布

- 候选全集是 `state.ranking` 的全部元素，即所有出局者（不再只看 `ranking[0]`）。
- 抽取用 `state.rng.below(Stream::Matchmaking, ranking.len() as u64)`。`Stream::Matchmaking` 原本只被 `pair_round` 的洗牌使用，重放时同一颗种子会得到同一序列，因此确定性成立。
- 只有存在落单配对时才抽，偶数人数局面的随机流与现在完全一致。
- 同一个出局者可能连续多个回合被抽中，不做去重。

#### 7.7.5 视图与协议

- `view.rs` 的 `opponent_seat`：

```rust
let opponent_seat = state.pairings.iter().find_map(|pairing| {
    if pairing.a != seat && pairing.b != seat {
        return None;
    }
    if let Some(ghost) = pairing.ghost {
        return Some(ghost);
    }
    Some(if pairing.a == seat { pairing.b } else { pairing.a })
});
```

- `CombatResult.mirror` 仍为 `true`；`winner_id` 现在是「幽灵战打赢时的自己」，`damage` 是真实伤害。
- `docs/versions/v0.1/technical/architecture/protocol-messages.md:105` 对 `mirror` 与 `damage` 的说明要改。

#### 7.7.6 规则文档

| 文件 | 改动 |
| --- | --- |
| `docs/versions/v0.1/product/rules/core-rules.md:187` | 落单座位对战**一名随机出局者的最终阵容**，必有对战；失败正常扣血、获胜计入胜者；幽灵按空壳结算（无英雄被动、无手牌、不写回） |
| `docs/versions/v0.1/product/overview/design-overview.md:113` | 同上口径 |
| `docs/versions/v0.1/product/overview/game-intro.md:215` | 「镜像幽灵」条目改为「幽灵阵容：随机一名出局者的最终阵容」 |
| `docs/current/technical/rule-engine-and-simulator.md:194` | `renumber_ghost` 说明保留，补「幽灵来源改为随机出局者，随机走 `Stream::Matchmaking`」 |
| `docs/current/technical/room-service.md:124` | 镜像战口径改为幽灵战，说明 `opponentPlayerId` 是幽灵归属座位 |

#### 7.7.7 测试

- `settle.rs::mirror_loss_does_not_damage_or_streak` **反转**为 `ghost_loss_damages_like_real_combat`：断言 `hp` 按公式扣减、`pending_settle` 有该座位、`last_won` 不含该座位。
- 删除与自战镜像一起作废的用例：`mirror_ghost_keeps_only_permanent_stats`、`mirror_ghost_falls_back_to_current_board`、`mirror_ghost_keeps_growth_breed_and_contagion`、`tests/auras.rs::mirror_ghost_from_previous_board_includes_auras`；新增「幽灵取自某个出局者棋盘」「幽灵保留出局者光环」「同一颗种子下抽取结果可重放」三条用例。
- 新增：存活为奇数且 `ranking` 为空时，落单座位照样产生 `Pairing`（`ghost: None`），`outcome.reports` 里有它、幽灵棋盘为空、判落单座位获胜。
- 新增：落单座位打赢幽灵 → `last_won` 含该座位、`last_winner` 是它；随后 `compare_alive` 的名次比较把它算作「上战胜者」。
- 新增：两名出局者存在时，不同种子抽到不同出局者；同一种子两次运行抽到同一名。
- 新增：幽灵阵容保留 `permanent` 层（不做 `strip_ghost`）。
- 新增：幽灵保留出局者自己的光环（棋盘上有一张光环随从时，幽灵随从的攻击/生命含该光环），且 `state.auras` 未被污染。
- `combat.rs`、`combat_damage.rs`、`turn_effects.rs`、`view.rs`、`auras.rs` 里构造 `Pairing { mirror: true }` 或调用 `mirror_ghost` / `mirror_ghost_with_auras` 的地方改签名；`turn_effects` 的「镜像战不写回」用例改为 `ghost: Some(其它座位)` 并断言幽灵棋盘未被修改；`auras` 的上一场快照用例删除，换成幽灵光环用例。
- `room-service` 的 `mirror_combat_result_marked_mirror` 用例改为断言 `winnerId` 与 `damage`；`tests/filter.rs:268-302` 里「镜像战 `damage == 0`」「未获胜 `winnerId == null`」两条断言按新口径改写。

## 8. 客户端（第二步）

### 8.1 页面骨架

- `game.gd`（1825 行）先拆成按功能划分的视图文件，各视图互不引用，只依赖 `GameUiState` 这份纯数据快照：`game.gd`（骨架、刷新调度、手牌、选择浮层、意图、门控、计时）、`game_card_face.gd`（卡面与文本格式化）、`game_detail_layer.gd`（详情层，含 `_input` 定位）、`game_players_rail.gd`（左栏）、`game_side_panel.gd`（右栏）、`game_recruit_view.gd`（商店与棋盘）、`game_combat_view.gd`（战斗视图）。拆分是纯搬运，节点名与行为不变，由现有冒烟用例兜底。
- 同时把冒烟测试的公共能力（`_expect`、场景装载、鼠标事件合成、战斗夹具注入）抽成 `client/tests/smoke_case.gd`，`client-smoke` 改为逐个跑 `client/tests/ui_smoke*.tscn`，每个改版任务各自新增一个独立场景。

```text
背景
└─ 外边距
   └─ 横向主体
      ├─ 左栏 座位列（196，滚动）
      │  └─ 座位行 × 8：头像 + 名次角标 + 血条
      │     └─ 展开时：该座位参与过的战报条目
      ├─ 中区（自适应）
      │  ├─ 招募视图：商店（6 槽 + 刷新/升时代/冻结）· 我方棋盘
      │  ├─ 战斗视图：对手随从 · 结果横幅 · 我方随从
      │  └─ 手牌行
      └─ 右栏（272）
         ├─ 顶部：场景身份（招募=售货员 / 战斗=对方英雄）
         ├─ 中部：第 N 回合 · 剩余 M 秒
         └─ 底部：英雄技能 · 结束招募 · 自己英雄介绍
   └─ 状态行（含断线提示）
选择浮层（全屏，默认隐藏）
详情层（全屏，忽略鼠标输入）：卡牌详情与座位详情
```

- 顶栏节点整条删除，不保留隐藏节点。
- `_conn_label` 的断线提示移入状态行。
- 事件流一并删除：`GameStore.event_log`、`game.gd` 的 `_refresh_log` / `_event_text` / `_combat_text` 全部移除，`GameStore.handle` 不再累积 `events`。冒烟用例 `ui_smoke.gd:633`（断言 `LogText` 不显示上一回合战斗）改写为「`LogText` 节点不存在」。
- 战斗中的「等待战斗动作…」空态提示保留在对手随从行，不依赖事件流。

### 8.2 座位行

| 元素 | 内容 |
| --- | --- |
| 头像 | `UI.avatar`，边长取新的 `avatar-rail` 度量（44） |
| 名次角标 | 头像左上角，`font-caption`、金色底；已淘汰仍显示数字，整行降透明度 |
| 血量条 | 头像下方同宽，按 `hp / hpCap` 填充；不显示数字 |
| 已淘汰 | 整行 `UI.apply_dim` |
| 本人 | 行描边用 `color-accent` |

- 不显示名字、时代、随从数、准备状态；这些进悬停详情。
- 点击行切换展开状态；同一时刻只展开一个，点另一行先收起旧的。
- **展开状态按 `playerId` 记住，视图重建后恢复**：`view_changed` 每收到视图都整块重建左栏（现有 `_refresh_players` 就是全量重建），不记住的话战报播完就自动收起。记住的 id 在新视图里找得到就重建展开块，找不到就清空。
- 展开块列该座位参与过的 `combatLog` 条目：`第 N 回合 · 对手名 · 胜/负 · 伤害 X`；幽灵战的对手名写「幽灵（某某）」，伤害为 0 时省略「伤害」段；没有条目时显示「无战报」。
- 悬停头像显示座位详情面板：名字、英雄名、生命 `当前/上限`、时代、随从数、存活/淘汰、已招募。

### 8.3 右栏

| 位置 | 招募阶段 | 战斗阶段 |
| --- | --- | --- |
| 顶部 | 售货员：中性沙色块 + 「售」字 + 「商店营业中」 | 按 §7.2「战斗阶段的对手身份」的顺序判定：已淘汰→「已淘汰 · 观战中」；`last_combat.mirror` 且有 `opponentPlayerId`→幽灵归属者的英雄卡片 + 「幽灵」角标（不显示血量条）；有 `opponentPlayerId`→对方英雄卡片（头像、英雄名、生命/时代/随从数）；否则→「等待配对」 |
| 中部 | `第 N 回合`，下方 `剩余 M 秒`；倒计时归零显示「等待结算」 | 同左 |
| 底部 | 英雄技能按钮 · 结束招募按钮 · `UI.hero_tile` · 属性行「生命 X/上限 · 时代 · 金币」 | 英雄技能与结束招募隐藏（保持现有行为） |

- 售货员是页面私有的静态块，不进共享层：它是唯一一个非玩家身份的头像，复用头像块的形状但用固定中性色。
- 属性行补上上限，与血条同源。

### 8.4 战斗视图

- 删除 `CombatOpponentStrip` 与 `CombatSelfStrip`，中区只剩「对手随从 / 结果横幅 / 我方随从」。
- 结果横幅、动作播放、存活随从渲染、按 `phaseDeadline` 续播全部保持不变。

### 8.5 商店卡角标与冻结

- 左上角：费用，取 `CardDB.display_cost(card_id)`，随从固定 3、效果牌取文档 `cost`，与服务端 `recruit.rs` 的扣费口径一致。
- 右上角：时代，取卡表 `era`（中文名，见 §7.5）直接显示；两处都用小号角标块，不与部落封面抢视觉。
- 原本左上角的「冻」角标移除；冻结改为：
  - 整卡盖一层 `alpha("color-info", 0.22)` 的半透明蒙层；
  - 蒙层上叠斜向冰晶纹理（`draw_line` 或旋转的细 `ColorRect` 组），角度与间距固定，不随卡片尺寸变化；
  - 右下角保留一个小「冻」字，保证不靠颜色单独传达状态。
- 卡面底部信息区仍是「名称 / 攻击 · 类型 · 生命」，不变。

### 8.6 页面状态

新增或调整的状态：

| 状态 | 表现 |
| --- | --- |
| 战报为空 | 展开座位显示「无战报」 |
| 座位已淘汰 | 名次角标继续显示数字，整行降透明度，仍可点击展开历史 |
| 幽灵战 | 右栏顶部显示幽灵归属者的英雄卡片 +「幽灵」角标，不显示血量条；战报条目对手名写「幽灵（某某）」 |
| 倒计时归零 | 右栏中部显示「等待结算」，其余禁用逻辑不变 |

## 9. 影响范围

### 9.1 文件

| 层 | 文件 |
| --- | --- |
| 协议 | `protocol/schema/v1/view.schema.json`、`protocol/generated/protocol.rs`、`client/generated/protocol.gd`、`protocol/samples/v1/s2c/*.json` 中 5 个含视图的样例 |
| 引擎 | `server/crates/rule-engine/src/view.rs`、`server/crates/rule-engine/src/settle.rs`、`server/crates/rule-engine/src/effects/aura.rs`、`server/crates/rule-engine/src/state.rs`、`server/crates/rule-engine/src/hash.rs`、`server/crates/rule-engine/tests/{view,settle,combat,combat_damage,turn_effects,auras}.rs` |
| 房间服务 | `server/crates/room-service/src/view.rs`、`game.rs`、`filter.rs`、`tests/{game,filter}.rs` |
| 机器人 | `server/crates/sim/tests/sim_bot.rs` 的手写 `PlayerView` JSON |
| 卡牌数据 | 36 份 `docs/versions/v0.1/product/content/cards/<效果牌 id>.md`、`scripts/validate_docs.py`、`server/data/v0.1/cards.json`、`client/generated/cards.json` |
| 客户端 | `client/scenes/game.gd`、`game_ui_state.gd`、`game_card_face.gd`、`game_detail_layer.gd`、`game_players_rail.gd`、`game_side_panel.gd`、`game_recruit_view.gd`、`game_combat_view.gd`、`client/autoload/game_store.gd`、`client/ui/ui.gd`、`client/ui/theme.gd`、`client/tests/smoke_case.gd`、`client/tests/ui_smoke*.gd`、`client/tests/fixture.gd`、`Makefile` |
| 文档 | `docs/current/product/ui/pages/game.md`、`tokens.md`、`interactions.md`、`components/avatar.md`、`docs/current/technical/client.md`、`room-service.md`、`rule-engine-and-simulator.md`、`docs/versions/v0.1/product/rules/core-rules.md`、`overview/design-overview.md`、`overview/game-intro.md`、`technical/architecture/protocol-messages.md`、`docs/versions/v0.1/product/content/card-system.md`、`docs/versions/v0.1/product/content/cards/README.md`、`docs/versions/v0.1/update/` 需求目录 |

### 9.2 兼容

- `hpCap`、`rank`、`combatLog` 都是新增必填字段；旧客户端忽略新字段。新客户端对旧服务端的回退：`hpCap` 缺失按 40、`rank` 缺失时**不显示名次角标**（不猜）、`combatLog` 缺失时展开显示「无战报」。
- `rank` 在 `GameOver` 阶段与 `placement` 取值一致，结算页继续读 `GameOver.ranking`，不受影响。
- 效果牌补 `era` 只动文档与生成数据，不影响已上线对局；`recruit.rs::is_eligible` 不读 `era`。

### 9.3 风险

- `rank` 让对局中所有人都能看到彼此的实时名次，属于信息面扩大；已确认为产品规则变化。
- 战报全对局公开会暴露所有人的每场配对与胜负；已确认。
- 商店卡新增两个角标后，108×120 的卡面更满，需要在真实分辨率下核对文字不重叠、不截断。

## 10. 效果牌时代映射

按「## 出处」的年代与题材归入七时代。实现时逐条核对，可改。

| 时代 | 卡牌 |
| --- | --- |
| 古典 | 奥林匹克祭典、城邦法令、二分法、方阵 |
| 封建 | 黑死病·播散、印刷术、专利制度 |
| 工业 | 布朗运动、法拉第笼、光电效应、交变电流、球状闪电、特斯拉线圈、有丝分裂、征兵令、蒸汽革命、蒸汽动力 |
| 现代 | 阿Q精神、低声细语、光合作用、艾滋病毒·抑制剂、流感病毒·变异、进化论、狂人日记、呐喊、彷徨、青霉素·提纯、人造闪电 |
| 未来 | DNA双螺旋结构、广义相对论、基因突变、人工智能、狭义相对论、引力波、原子弹、质能等价 |

- 合计 4 + 3 + 10 + 11 + 8 = 36。
- 判定口径：与随从牌的既有归属对齐（达尔文 1859 → 现代、爱因斯坦 1905 → 现代、海森堡 1927 → 未来、费米 1942 → 未来），理论的抽象度决定落在「现代」还是「未来」。

## 11. 验证

### 11.1 第一步

| 命令 | 覆盖 |
| --- | --- |
| `make protocol-rs`、`make protocol` | 生成物与 schema 同步 |
| `make protocol-rs-check`、`make protocol-check` | 无漂移、样例通过 |
| `python3 scripts/validate_docs.py` | 效果牌 `era` 必填、效果牌时代分布、属性表「时代」行 |
| `make data`、`make data-check` | 36 张效果牌 `era` 进入两处生成数据且无漂移 |
| `make test` | 引擎 `rank_of`、幽灵战胜负与伤害、房间服务 `combatLog`、`sim_bot` 反序列化 |
| `make lint` | `cargo fmt --check`、`clippy -D warnings` |
| `make m1b-e2e` | 真实客户端 + 7 机器人打完一局，确认 `rank` 与 `combatLog` 不破坏现有流程 |

`make test`（`Makefile:6-8`）只跑 `unittest discover` 与 `cargo test --workspace`，不调用 `scripts/validate_docs.py`；`make data-check` 只跑两个生成器与 `audit_effects.py`。文档校验必须单独执行，且要在 `make data` 之前跑一次，确认 36 张效果牌的时代字段合法。

引擎新增用例：
- 存活 7 人、1 人已出局时，存活者名次为 1–7，出局者为 8。
- 名次与第 20 回合收束时的 `compare_alive` 顺序一致（构造同生命、不同 `damage_dealt` 的两人）。
- `GameOver` 阶段 `rank == placement`。
- 幽灵战（§7.7.7）：随机出局者棋盘、输掉按公式扣血、赢下写入 `last_won`、同种子可重放、无出局者时对战空阵容。

房间服务新增用例：
- 一场真实战写入 1 条 `combatLog`，两个座位各收到同一条，`participantIds` 为这两人。
- 幽灵战写入一条：`mirror` 为真，`participantIds` 只有一个元素；获胜时 `winnerId` 为该座位、`loserId` 为空、`damage` 为 0，失利时 `winnerId` 为空、`loserId` 为该座位、`damage` 大于 0。**归属靠 `participantIds`，不靠 `winnerId`/`loserId`。**
- 超过 5 个回合后，最旧回合的条目被丢弃。
- 重连的 `Snapshot` 携带完整历史。

### 11.2 第二步

| 命令 | 覆盖 |
| --- | --- |
| `make client-check` | 边界断言、协议与数据无漂移 |
| `make client-smoke` | 对局页新结构与各状态 |
| `make m1b-e2e` | 真实对局跑通 |

冒烟新增断言：
- 顶栏节点不存在；右栏顶部在招募阶段是售货员块、战斗阶段是对手英雄块；已淘汰观战者显示「已淘汰 · 观战中」。
- 积分榜 8 行，每行含头像、名次角标与血条；不含量化为文本的血量。
- 点击座位行展开战报，同屏只有一块；再点收起；无条目的座位显示「无战报」。
- 战斗视图不含双方英雄条（`CombatOpponentStrip` / `CombatSelfStrip` 节点不存在）。
- `ui_smoke.gd:305-319` 现断言 `EraPill` 文本与「积分榜本人/对手行显示时代名」，顶栏与座位行都不再显示时代，这些断言随结构一起改写：`EraPill` 断言删除，时代名改在右栏属性行断言。
- `LogText` 节点不存在，`GameStore.event_log` 不再增长。
- 商店卡左上角是费用、右上角是时代；冻结卡存在蒙层节点与「冻」字。

## 12. 留给实现的部分

- 冰晶纹理的具体几何（线段角度、间距、透明度），以视觉可辨且不遮挡数值为准。
- 座位行的高度与间距，以 8 行在 196 px 栏宽、720 px 高内不滚动为准；超出时滚动。
- 右栏顶部身份块的高度，使中部回合与底部英雄介绍在 720 px 高度内不需要滚动。

## 13. 实施者已定的取舍（请一并确认）

分节里的细节由本文档直接定下，这里把可能有争议的几条集中列出：

1. **幽灵保留自己的光环**：开战时按出局者的棋盘重算一次它的光环来源，幽灵以死时的真实战力参战。代价是把 `recompute_auras` 的循环体抽成 `player_auras` 供两处复用。
2. **双方同回合清场一律判平局**：真实战与幽灵战同一口径，双方都不扣血、都不计入「上战胜者」，战报 `winner` 为 `null`。这会改掉现行 `core-rules.md:123`（互清按生命 → 累计伤害 → 座位判胜），属于附带的产品规则改动。行动数到上限（200 次）而双方仍有存活者时不是「清场」，仍按 §5.3 的生命 → 累计伤害 → 座位判出胜者。
3. **平局不改先手**：`last_winner` 在平局时保持不变，下一次战斗的先手沿用上一场真实胜者。
4. **幽灵战胜利不累加 `damage_dealt`**：`compare_alive` 的第三键是「累计造成的伤害」，幽灵战没有打出真实伤害，落单座位不应因此获得名次加权。
5. **幽灵从全部出局者里随机抽取**，每回合抽一次，可能连续抽到同一人；不做「最近出局者」，也不做 3 回合去重。
6. **`Pairing.mirror` 换成 `Pairing.ghost: Option<Seat>`**：一个字段表达「对手是不是幽灵、幽灵是谁」，不再依赖「`b == a` 即镜像」的隐式约定。会让历史 `state_hash` 不兼容，本版本没有需要重放的存量对局。
7. **`combatLog` 不带幽灵归属者的 id**：`participantIds` 只放真实参战玩家，幽灵战就是落单座位一个元素；幽灵归属者不进名单，已出局座位的历史里不会出现「自己死后被人拿去当幽灵」的战报。
8. **战斗视图左侧仍按「我方 / 对方」两行**：幽灵战的对手行渲染的是出局者的最终阵容，与服务端下发的 `opponentBoard` 一致，不额外加「幽灵」标记（只在右栏顶部身份块与战报条目里标注）。
9. **彻底移除自战镜像**：幽灵只来自出局者，落单座位不会再对自己。`mirror_ghost`、`mirror_ghost_with_auras`、`strip_ghost`、`GameState.previous_board` 一并删除。
10. **落单座位永远对战**：没有出局者时幽灵是空阵容，配对、战报与结算照常（真实对局不可达，只有测试能构造）。
