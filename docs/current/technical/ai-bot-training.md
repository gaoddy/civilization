# AI 机器人与训练

> 状态：训练管线已实现（C0/C1/C2 训练与评测入口、神经策略接入）；C1/C2 的训练目标尚未按验收口径复核，C3 待跑
> 代码：`server/crates/ai-core`、`server/crates/ai-model`、`server/crates/ai-train`、`server/crates/bot-ai`、`make/ai.mk`

本文记录已实现的 AI 训练与接入现状：crate 职责、训练阶段与门槛、命令、产物落点、线上接入与重训方式；不描述产品难度表与后续里程碑规划。

## 1. 工程结构

| crate | 职责 |
| --- | --- |
| `ai-core` | 观测与动作编码：`Observation`（`OBS_DIM = 250`）、`Action`（15 种，含 `PickHero`）、`ActionFeatures`、`CardIndex`、`History`、`Encoder`；`legal_actions` 在 `rule-engine`，这里是唯一的编码口径 |
| `ai-model` | 权重格式与纯 Rust 推理：`Weights`（`weights.json` 清单 + `weights.bin` 扁平 `f32`）、`ModelConfig::from_weights`（从张量形状反推结构）、`forward` / `board_value` / `forward_hero`；不依赖任何 ML 运行时 |
| `ai-train` | 数据与训练：`data_c0`（棋盘对生成）、`env`（8 人完整对局）、`env_recruit`（C1 单座位局）、`env_full` + `league`（C2 联盟）、`ppo`（`TrainModel` 与 PPO 更新）、`train_c0` / `train_c1` / `train_c2`、`eval_c1` / `eval_full`；`candle` 依赖只在 `train` feature 后 |
| `bot-ai` | 对局策略：现有启发式 `plan`、`Policy` trait、`HeuristicPolicy`、`NeuralPolicy`（用 `ai-model` 推理，不引入 `candle`） |

网络结构：主干 64×2（`trunk.*`）、价值头（`value.*`）、按候选打分的头（`cand.*`）、选英雄的独立头（`hero_head.*`，输入为英雄嵌入 + 主干），三张嵌入表（`card.emb` / `hero.emb` / `skill.emb`，宽度 16）。结构由权重形状反推，调用方不另配。

选英雄的独立头是设计 D2 的实现：一批候选整批都是 `PickHero` 时走 `hero_head.*`，其余走 `cand.*`；训练与推理用同一个判据（`ai_core::features::is_hero_choice_batch`），混合批次里按决策分组打分再按原顺序拼回。旧检查点没有 `hero_head.*` 张量，选英雄时会推理失败并回退启发式，需要用当前代码重训。

## 2. 训练阶段

| 阶段 | 目标 | 门槛（初始值） |
| --- | --- | --- |
| C0 | 棋盘强度估值 `s(board, hero)` | 留出棋盘对 Brier ≤ 0.25、ECE ≤ 0.03 |
| C1 | 单座位构筑（招募决策） | 对冻结棋盘分布胜率 ≥ 55%，三次评测点单调上升 |
| C2 | 8 人完整对局（自博弈 + 联盟） | 2000 个留出种子上平均名次比启发式锚点好 ≥ 0.3，95% 置信区间不跨 0 |
| C3 | 接入房间服务后的真实对局复核 | 与 C2 同口径，在房间服务驱动路径上复测 |

现有实测（`output/` 为缓存，数值随重训变化）：C0（20000 步、release）Brier 0.0614、ECE 0.0241；C1（200 局训练、32 局评测）胜率 0.125、均差 −0.719；C2（40 局训练）训练内平均名次 4.43。C1/C2 的验收口径尚未复核。

候选特征归一化：位置类（打牌插入点、站位目标）按最大合法下标 6 归一化（`BOARD_POSITION_DIVISOR`），发现/进化选项按「实际候选数 − 1」归一化；最大合法取值恒为 1.0。

`make ai-eval` 的评测口径对应 C2 门槛：每局 1 个学习者座位 + 7 个锚点座位（默认启发式机器人），贪心策略、座位轮换；报告平均名次、锚点平均名次、按局配对的名次优势 `anchor_mean − mean_placement` 及其 95% 置信区间。评测种子从 `EVAL_SEED_BASE = 1_000_000` 起，与训练种子池（`C2Config::seeds`，默认 `0..10_000`）不相交，避免在训练过的种子上评测。单局超过 `max_steps_per_game` 视为截断：跳过该局、计入报告的 `truncated`，全部截断时报错而不是返回空报告。C2 训练同理：`C2Config::max_steps_per_episode` 截断的对局只计入 `truncated_episodes`，不进入 `mean_placement` 与 `mean_return`。

## 3. 命令

```bash
make ai-data             # 生成 C0 棋盘对（重训前先跑一次）
make train-c0            # 训练估值网络并校准
make train-c1            # 用 C0 权重初始化，训练单座位构筑
make train-c2            # 8 人自博弈混训（release；EPISODES 可调）
make ai-eval             # 固定种子评测检查点并挑最佳（SEEDS、RUN 可调）
make ai-test             # ai-core / ai-model / ai-train 测试（开启 train feature）
make ai-check            # format + clippy（开启 train feature，警告即错误）
```

参数：

| 变量 | 默认 | 作用 |
| --- | --- | --- |
| `AI_MODELS` | `../output/models`（相对 `server/`） | 训练产物根目录 |
| `SEED` | `0` | 各阶段随机种子（C0/C1/C2 共用） |
| `GAMES` | `2000` | `ai-data` 生成的棋盘对数 |
| `EPISODES` | `1000` | C2 总对局数 |
| `SEEDS` | `2000` | `ai-eval` 的固定种子数 |
| `RUN` | 空 | `ai-eval --run <dir>`，把评测指向某个检查点目录 |

阶段顺序固定：C0 产出的 `pairs.jsonl` 与 `weights.json` 是 C1 的输入，C1 的 `weights.json` 是 C2 的演员初始化；C2 缺 C1 权重时允许从随机初始化开始（冒烟用）。

训练代码全部在 `train` feature 后，`cargo test --workspace` 与 `make lint` 默认不编译它们；`make check` 额外并行跑 `ai-test` 与 `ai-check`，`make check-core` 不跑（基础门禁保持轻量）。

## 4. 产物

```
output/models/c0/pairs.jsonl        # 棋盘对数据（含 catalog_hash）
output/models/c0/weights.json(.bin) # C0 估值网络
output/models/c1/weights.json(.bin) # C1 策略
output/models/c2/weights.json(.bin) # C2 策略
output/models/<phase>/manifest.json # schema、weights_version、catalog_hash、结构参数、seed、配置与训练指标
output/models/c1/eval.json          # C1 冻结分布评测（胜率、均差）
output/models/c2/snapshots/<更新次数>/weights.json # train-c2 的周期检查点（--checkpoint-every，默认每 20 次更新）
output/models/c2/eval.json          # ai-eval 的检查点对比与最佳路径
```

`output/` 是纯缓存，可整体删除；真源是卡表与训练代码，重跑命令即可重建。`weights.json` 含 `hero_head.*`（选英雄独立头）与 `cand.*`（通用候选头）两套动作打分参数。`manifest.json` 的 `catalog_hash` 是卡表指纹（`ai_core::catalog_hash`）：`NeuralPolicy::load` 读到它时与当前卡表比对，不一致直接拒绝加载（回退启发式），避免旧权重的嵌入查错。

## 5. 线上接入与回退

房间服务的机器人策略由环境变量选择：

| 变量 | 默认 | 取值 |
| --- | --- | --- |
| `BOT_POLICY` | `heuristic` | `heuristic` / `neural` |
| `BOT_MODEL_DIR` | `output/models/c2` | 含 `weights.json` 的目录 |

回退规则：

1. `BOT_POLICY=neural` 加载失败（目录缺失、权重缺失、嵌入表条目数或张量形状与卡表不符、`catalog_hash` 与当前卡表不一致）→ 记录日志，回退启发式；
2. 神经策略拒绝决策（非招募阶段、推理失败、候选为空）→ 该次调用回退启发式；
3. 启发式也返回 `None`（座位已出局）→ 该机器人本回合停手。

`NeuralPolicy::plan` 是纯函数：同一状态与权重给出同一意图；全部随机性来自状态与对局种子，不读时钟。每次决策按完整合法动作重新打分，被拒绝的意图在下一次调用中自然被跳过。

权重按（目录，卡表指纹）进程级共享加载（`NeuralPolicy::load_shared`）：同一份权重只读一次，所有 `GameTask` 复用同一实例；缓存不失效，替换权重文件后需要重启进程。

**观测覆盖缺口（已知，未实现）**：`Observation` 只实现了 spec §5.1 的一部分。已给：金币、时代、血量、英雄 id、商店（冻结）、手牌、棋盘（攻防/金色/冻结）、本局玩法、回合与阶段、其余 7 人的存活/血量/时代/英雄/上阵数/名次、上一场对手棋盘。未给：卡牌关键词、随从技能的剩余次数、英雄主动技能及剩余次数、商店是否买得起、待选的候选内容（待选只有类型码）、本回合对手座位。补这些字段会改变 `OBS_DIM` 与观测布局（`observation_layout_is_stable` 的基准），必须同时递增 `WEIGHTS_VERSION` 并重训，因此不在本次修复范围内。

**已知观测缺口（C3 范围）**：观测里有「本人亲自对战过的对手最近一次棋盘」（`History`）。训练环境在战斗结算时记录它，但房间服务目前不重建，`NeuralPolicy` 始终用全 `-1` 的 `History` 推理，因此这 56 维在线上是训练分布外的常数。影响是神经策略在这部分信息上偏弱，不影响正确性；按 spec 的 C3（接入房间服务后的真实对局复核）补服务端重建。在 C3 完成前，神经策略只作为非默认选项，默认仍是启发式。

延迟实测（M1 Max，release，64×2、嵌入 16，开局满合法动作）：`plan` p50 0.10 ms、p99 0.13 ms（1000 次）；debug 构建 p50 2.79 ms、p99 50.89 ms，因此延迟门禁只在 release 下断言。

## 6. 重训

1. 卡表或规则变化后按 `make ai-data` → C0 → C1 → C2 顺序重跑；`catalog_hash` 变化即说明旧权重与旧棋盘对都不再对应。`train-c2` 每 `--checkpoint-every`（默认 20）次更新把权重写到 `output/models/c2/snapshots/`，崩溃后可从最近检查点继续评测或作为续训初始化；检查点只含权重，不含优化器状态，因此不是逐位可复现的断点续训。
2. 权重格式变化时递增 `ai-model::WEIGHTS_VERSION`，加载端按版本拒绝。
3. 训练可复现：初始化种子化（`seed_weights`），同一 seed 与步数得到同一份权重。
4. 检查点选择只看固定种子集上的评测指标（平均名次），不看训练损失或训练内胜率。
5. 选英雄独立头启用后，C1/C2 的旧权重需要重训（缺 `hero_head.*`）；C1/C2 的全局分布对手来自 `distribution_boards`，按「玩法 + 随从集合（卡、金色、当前攻血）」去重，只在英雄或实例编号上不同的棋盘算同一份对手。
