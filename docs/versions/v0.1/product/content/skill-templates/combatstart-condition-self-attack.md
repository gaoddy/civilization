---
id: combatstart-condition-self-attack
kind: effect
trigger: OnCombatStart
op: ModifyStats
selector: self
arg.attack: int
arg.taxon: taxon
arg.count: int
params: attack={attack},duration=permanent,condition=count:friendly:taxon={taxon}:>={count}
text: 战斗开始时，若你控制至少 {count} 个{taxon}，获得 +{attack} 攻击力
---
