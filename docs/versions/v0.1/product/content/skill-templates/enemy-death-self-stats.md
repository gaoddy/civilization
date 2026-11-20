---
id: enemy-death-self-stats
kind: effect
trigger: OnEnemyDeath
op: ModifyStats
selector: self
arg.attack: int
arg.health: int
params: attack={attack},health={health},duration=permanent
text: 每当一个敌方随从死亡，获得 +{attack}/+{health}
---
