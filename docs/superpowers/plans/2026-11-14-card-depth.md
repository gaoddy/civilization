# 卡牌深化与平衡测算 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把关键词从 10 个扩到 13 个，重做 70 张新增卡使机械/植物/生物各具专属动词，并产出一份手工平衡测算。

**Architecture:** 先改规则文本（00 → 03 → 08），再改数据（05 + CSV），最后用数据写平衡测算（11）。素材卡 50 张的卡名、数值、关键词一律不动。

**Tech Stack:** Markdown + CSV（UTF-8）。不是 git 仓库，不含提交步骤。

**Spec:** `/Users/gaoddy/Projects/Civilization/docs/superpowers/specs/2026-11-14-card-depth-design.md`

## Global Constraints

- 总数 120、时代 24/24/24/30/18、流派 24/22/24/12/10/28、星级 18/24/26/22/18/12 全部不变。
- 50 张 `source=素材` 的卡：`name`、`attack`、`health`、`keywords` 不改；只允许改 `text` 以修正不合规表述。
- 学者 `attack` 恒 0。
- 新词只允许出现在对应流派：装配→机械、播种→植物、进化→生物。
- 每张卡仍须满足 08 分册的星级区间与折算上限。
- 学者不因新词改变规则。

## Review Focus

1. 新词越界：装配出现在非机械卡、播种出现在非植物卡、进化出现在非生物卡。
2. 数值越界：为了塞新词而把攻击/生命改出星级区间。
3. 学者输出：新增卡的文本让学者间接造成伤害。
4. 时代关键词表与新词的冲突：流派专属词不得被写成"按时代解锁"。
5. 平衡表与 CSV 不一致：矩阵里引用的数值必须与数据表一致。

---

### Task 1: 总纲与词表

**Files:**
- Modify: `docs/00-设计总纲.md`
- Modify: `docs/03-关键词与机制.md`
- Modify: `docs/08-数值与经济.md`

**Interfaces:**
- Produces: 三个新词的名称、精确判定、折算值；后续任务只引用，不重新定义。

- [ ] **Step 1: 改 00 §6.8**

标题改为「关键词与区域（首版 13 个关键词）」，表格追加三行：装配 N、播种、进化。加一句：流派专属词不参与时代解锁。

- [ ] **Step 2: 改 03**

关键词表追加三行，列结构不变。新增「§1.1 流派专属关键词」说明：装配只在机械、播种只在植物、进化只在生物；不按时代解锁。结算顺序 §2.2 补"装配"，§2.3 补"播种与进化"。

- [ ] **Step 3: 改 08 §5 折算表**

追加：装配 ≈ 2、播种 ≈ 2、进化 ≈ 3。

- [ ] **Step 4: 校验**

Run: `grep -c '装配\|播种\|进化' docs/00-设计总纲.md docs/03-关键词与机制.md docs/08-数值与经济.md`
Expected: 三个文件均 > 0

---

### Task 2: 重做 70 张新增卡

**Files:**
- Modify: `docs/data/卡牌数据.csv`
- Modify: `docs/05-卡牌规范与数据.md`

**Interfaces:**
- Consumes: Task 1 的三个词。
- Produces: 更新后的 120 行数据与文档中的全量卡表。

- [ ] **Step 1: 改写 CSV 中 70 行 `source=新增` 的卡**

装配出现在 14 张机械卡；播种出现在 9 张植物卡；进化出现在 6 张生物卡；其余新增卡用条件触发。所有 `source=素材` 行保持不变。

- [ ] **Step 2: 更新 05 的规范与总览**

§2 关键词规则加入三个流派专属词；§3 数据总览的"按来源"不变；§4.1/§4.3 不变；§6 全量卡表按新 CSV 重新生成。

- [ ] **Step 3: 校验**

Run: `python3 -c "
import csv,re,collections
r=list(csv.DictReader(open('docs/data/卡牌数据.csv',encoding='utf-8')))
R={1:(1,2,2,4),2:(2,3,3,5),3:(3,5,4,7),4:(4,7,6,9),5:(6,9,8,12),6:(8,12,10,16)}
kw=re.compile(r'^(无|战吼|亡语|复生|嘲讽|护盾|连击|传染|最优攻击|繁衍 \d+|装配 \d+|播种|进化)$')
bad=[]
for c in r:
    lv=int(c['level']);a=int(c['attack']);h=int(c['health'])
    if c['is_scholar']=='true' and a!=0: bad.append(('scholar',c['name']))
    if c['is_scholar']=='false':
        lo_a,hi_a,lo_h,hi_h=R[lv]
        if not(lo_a<=a<=hi_a and lo_h<=h<=hi_h): bad.append(('stat',c['name']))
    for t in c['keywords'].split('；'):
        if not kw.match(t.strip()): bad.append(('kw',c['name'],t))
print(len(r),len(bad))
"`
Expected: `120 0`

Run: `python3 -c "
import csv
r=list(csv.DictReader(open('docs/data/卡牌数据.csv',encoding='utf-8')))
for k,t in [('装配','机械'),('播种','植物'),('进化','生物')]:
    n=[x['name'] for x in r if k in x['keywords'] and x['tribe']!=t]
    m=sum(1 for x in r if k in x['keywords'])
    print(k,m,'wrong_tribe',len(n))
"`
Expected: 装配 ≥8、播种 ≥5、进化 ≥6，且 `wrong_tribe 0`

---

### Task 3: 平衡测算

**Files:**
- Create: `docs/11-平衡测算.md`

**Interfaces:**
- Consumes: 05 的 CSV 与 08 的区间表。
- Produces: 流派×星级矩阵、6 套阵容推演、失衡判据。

- [ ] **Step 1: 写流派×星级强度矩阵**

6 行（流派）× 6 列（星级），每格为该流派在该星级的平均"身材 + 关键词折算"，并对照 08 的参考白板。

- [ ] **Step 2: 写 6 套阵容的 8 回合推演**

每套阵容给：第 1/3/5/8 回合的金币、时代、场上阵容（卡名+攻/血）、战斗结果与掉血。必须使用 CSV 中的真实数值。

- [ ] **Step 3: 写失衡判据**

列出哪些组合会超出 08 的伤害区间，以及调整优先级（先调卡、再调概率、最后调伤害公式）。

- [ ] **Step 4: 校验**

Run: `grep -c '^| ' docs/11-平衡测算.md`
Expected: 至少 40

---

### Task 4: 索引与全量复核

**Files:**
- Modify: `docs/README.md`
- Modify: `docs/00-设计总纲.md`（§1 表格加 11）

- [ ] **Step 1: README 增加 11 的条目与阅读顺序**

- [ ] **Step 2: 全量校验**

Run: `ls docs/*.md docs/data/*.csv | wc -l`
Expected: `15`

Run: `python3 -c "
import csv,collections
r=list(csv.DictReader(open('docs/data/卡牌数据.csv',encoding='utf-8')))
print(len(r), collections.Counter(x['era'] for x in r), collections.Counter(x['tribe'] for x in r))
"`
Expected: `120`，分布与会话前一致

Run: `grep -rn '10 个关键词' docs/*.md`
Expected: 无输出
