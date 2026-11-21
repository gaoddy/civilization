class_name SeatDetailSection
extends RefCounted
## 座位详情：名字、英雄、生命（当前/上限）、时代、随从数与状态。


static func render(box: VBoxContainer, player: Dictionary, hp_cap: int, kit: DetailKit) -> void:
    box.add_child(kit.name_label(CardText.optional_text(player.get("displayName"))))
    box.add_child(kit.row(tr("ui.game.detail.hero"), CardDB.display_name(CardText.optional_text(player.get("heroCardId")))))
    box.add_child(kit.row(tr("ui.game.detail.hp"), "%d/%d" % [int(player.get("hp", 0)), maxi(hp_cap, 0)]))
    box.add_child(kit.row(tr("ui.game.detail.era"), CardDB.era_name(int(player.get("era", 1)))))
    box.add_child(kit.row(tr("ui.game.detail.minions"), "%d" % int(player.get("boardCount", 0))))
    var alive := bool(player.get("alive", true))
    box.add_child(kit.row(
        tr("ui.game.detail.status"),
        tr("ui.game.detail.alive") if alive else tr("ui.game.detail.eliminated"),
    ))
    if alive:
        box.add_child(kit.row(
            tr("ui.game.detail.recruit"),
            tr("ui.game.detail.recruited") if bool(player.get("recruited", false)) else tr("ui.game.detail.not_recruited"),
        ))
