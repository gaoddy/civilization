# AI 机器人与训练

> 状态：训练管线已实现（C0/C1/C2 训练与评测入口、神经策略接入）；C2 在 200 个留出种子上已比启发式锚点好 0.57（门槛 0.3），正式的 2000 种子复核、C1 门槛与 C3 待跑
> 代码：`server/crates/ai-core`、`server/crates/ai-model`、`server/crates/ai-train`、`server/crates/bot-ai`、`make/ai.mk`

本文记录已实现的 AI 训练与接入现状：crate 职责、训练阶段与门槛、命令、产物落点、线上接入与重训方式；不描述产品难度表与后续里程碑规划。

## 1. 工程结构

| crate | 职责 |
| --- | --- |
| `ai-core` | 观测与动作编码：`Observation`（`OBS_DIM = 431`，`SCHEMA_VERSION = 2`）、`Action`（15 种，含 `PickHero`）、`ActionFeatures`、`CardIndex`、`History`、`Encoder`；`legal_actions` 在 `rule-engine`，这里是唯一的编码口径 |
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

现有实测（`output/` 为缓存，数值随重训变化）：C0（20000 步、release）Brier 0.0614、ECE 0.0241；C1（200 局训练、32 局评测）胜率 0.19、均差 −0.59（同配置换种子在 0.09–0.41 之间跳，短训练 + 小评测集，方差大）；C2（1000 局训练、200 个留出种子）平均名次 4.00 ±0.23，启发式锚点 4.57，名次优势 +0.57 ±0.26（95% 置信区间不跨 0），吃鸡率 5%、前四率 67%；正式的 2000 种子复核尚未跑。

贪心解码下策略仍有大量零收益决策：16 局诊断里 10900 次决策有 9029 次是 `Rearrange`，298 个回合里 244 个打满 `DECISIONS_PER_TURN`。棋盘能堆到 7 个随从、血量 +6.2，说明它不再妨碍构筑，只是白刷决策；训练侧的 `STEP_COST` 只有 0.0005，量级远小于价值头误差，消不掉这个循环。要压掉它得动决策上限或奖励尺度，都会改变对局长度，需要单独重训验证。

候选特征归一化：位置类（打牌插入点、站位目标）按最大合法下标 6 归一化（`BOARD_POSITION_DIVISOR`），发现/进化选项按「实际候选数 − 1」归一化；最大合法取值恒为 1.0。观测里的新字段同口径：技能剩余次数按 `uses_per_turn` 上限 2 归一（`MAX_USES_PER_TURN`），关键词层数按 3 归一（`KEYWORD_LAYER_DIVISOR`），目标待选候选数按候选上限 23（7 棋盘 + 10 手牌 + 6 商店）归一（`MAX_TARGET_CANDIDATES`）。

动作空间不变量：`legal_actions` 只枚举真正推进状态的动作——冻结只枚举会改变冻结标记的槽位（已经冻结的不再冻结），解冻完全不进候选，目标待选有候选时不枚举「放弃目标」。解冻与放弃目标都只是客户端的「撤销误点」，意图层照旧接受（客户端重复点击不报错），但 AI 要冻结子集直接逐槽冻结、要指定目标直接选目标即可。只过滤空操作不够：冻结某槽再解冻同槽、不带目标出牌再放弃目标，每一步都真的改变状态，策略照样能在两个状态之间无限切换（实测 C2 训练里整回合都在轮换解冻、后期又在「出牌 → 放弃目标」之间打转），所以这类可逆动作整体退出候选。`ai-train` 的 `DECISIONS_PER_TURN = 40` 只是兜底上限：正常回合十几次决策就结束，打满即强制结束招募。

时间成本：站位（`Rearrange`）、刷新这类零成本可逆动作仍然留在候选里——它们是真实战略动作，不能删。代价是策略会在一个回合里反复挪位置，实测 C2 每回合 40 次决策里八成是 `Rearrange`，训练数据被无效动作淹没。因此学习者的每个非 `End` 决策扣 `STEP_COST = 0.0005`（`train_c2::decision_reward`，C1/C2 两个采样循环同口径）：正常回合十几次决策、打转回合四十次，一局约差 0.2 回报，相当于一到两个名次档，小到不会改变「买牌优于不买」这类真实偏好。这是训练侧的奖励，推理侧不需要任何配套机制。

权重数值门槛：`Weights::save` 与 `Weights::load` 都拒绝非有限值。训练发散得到的 NaN 权重不会被落盘、也不会被当成模型加载——推理侧的 ReLU 会把 NaN 吃成 0，让无效权重给出一个看似正常的评测分数。训练中途还有一道更早的拦截：每个决策校验观测、logit 与价值，每次更新校验全部权重，报错带上局号、座位、回合或更新序号；同时按更新打印名次、策略损失、价值损失、熵与 clip 比例，避免整轮训练静默跑完才发现坏权重。PPO 侧另有一个数值前提：掩码 log-softmax 必须先减最大值再算 logsumexp，否则 logit 超过 f32 指数上界（约 88.7）时 `exp` 溢出成 inf，反传得到 NaN。

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

`Observation` 已覆盖 spec §5.1 的全部字段：框头（座位、回合、阶段、英雄、玩法掩码、血量、金币、时代）、商店/手牌/棋盘槽位、其余 7 人与本人亲自见过的最近棋盘、商店可购买性（引擎 affordance 口径）、棋盘关键词 16 位 multi-hot 与 `breed`/`assemble`/`armor` 层数、英雄与随从主动技能的剩余次数与可用性、发现候选卡索引与目标待选候选数、本回合对手座位。

观测布局变化会改 `OBS_DIM`（`observation_layout_is_stable` 的金样本基准），必须同时递增 `ai_core::SCHEMA_VERSION` 与 `ai_model::WEIGHTS_VERSION` 并重训；旧权重因版本不匹配被拒绝加载。当前版本：`OBS_DIM = 431`、`SCHEMA_VERSION = 2`、`WEIGHTS_VERSION = 2`。

**已知观测缺口（C3 范围）**：观测里有「本人亲自对战过的对手最近一次棋盘」（`History`）。训练环境在战斗结算时记录它，但房间服务目前不重建，`NeuralPolicy` 始终用全 `-1` 的 `History` 推理，因此这 56 维在线上是训练分布外的常数。影响是神经策略在这部分信息上偏弱，不影响正确性；按 spec 的 C3（接入房间服务后的真实对局复核）补服务端重建。在 C3 完成前，神经策略只作为非默认选项，默认仍是启发式。

延迟实测（M1 Max，release，64×2、嵌入 16，开局满合法动作）：`plan` p50 0.10 ms、p99 0.13 ms（1000 次）；debug 构建 p50 2.79 ms、p99 50.89 ms，因此延迟门禁只在 release 下断言。

## 6. 重训

1. 卡表或规则变化后按 `make ai-data` → C0 → C1 → C2 顺序重跑；`catalog_hash` 变化即说明旧权重与旧棋盘对都不再对应。`train-c2` 每 `--checkpoint-every`（默认 20）次更新把权重写到 `output/models/c2/snapshots/`，崩溃后可从最近检查点继续评测或作为续训初始化；检查点只含权重，不含优化器状态，因此不是逐位可复现的断点续训。
2. 权重格式或观测布局变化时递增 `ai-model::WEIGHTS_VERSION`（观测布局同时递增 `ai_core::SCHEMA_VERSION`），加载端按版本拒绝；`OBS_DIM` 改变后 `output/models` 下的旧权重全部作废，必须按 C0 → C1 → C2 重训。
3. 训练可复现：初始化种子化（`seed_weights`），同一 seed 与步数得到同一份权重。
4. 检查点选择只看固定种子集上的评测指标（平均名次），不看训练损失或训练内胜率。
5. 选英雄独立头启用后，C1/C2 的旧权重需要重训（缺 `hero_head.*`）；C1/C2 的全局分布对手来自 `distribution_boards`，按「玩法 + 随从集合（卡、金色、当前攻血）」去重，只在英雄或实例编号上不同的棋盘算同一份对手。
