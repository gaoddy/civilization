class_name SellerDetailSection
extends RefCounted
## 售货员详情：营业状态与可执行的商店操作，替代常驻文字。


static func render(box: VBoxContainer, kit: DetailKit) -> void:
    box.add_child(kit.name_label(tr("ui.game.detail.seller")))
    box.add_child(kit.row(tr("ui.game.detail.status"), tr("ui.game.detail.shop_open")))
    var hint := UIControls.label(
        tr("ui.game.detail.shop_hint"),
        UITheme.font_size("font-body-sm"),
    )
    hint.custom_minimum_size = Vector2(kit.text_width, 0)
    hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
    hint.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    box.add_child(hint)
