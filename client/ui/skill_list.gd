class_name SkillList
extends RefCounted
## 英雄技能列表：每条技能一个块，技能名（有名称时）与触发标签同行，
## 完整文本在下一行；主动技能追加费用与每回合次数。
##
## 只渲染能解析到的技能；全部 id 无效时显示空态，不渲染空块。


static func build(parent: VBoxContainer, skill_ids: Array) -> void:
    var infos: Array[Dictionary] = []
    for skill_id in skill_ids:
        var info := CardDB.skill_info(str(skill_id))
        if bool(info.get("exists", false)):
            infos.append(info)
    if infos.is_empty():
        parent.add_child(UIControls.empty_hint(UIWording.translated_or_empty("ui.skill.empty")))
        return
    var index := 0
    for info in infos:
        var block := VBoxContainer.new()
        block.name = "SkillBlock%d" % index
        block.add_theme_constant_override("separation", UITheme.space("space-2xs"))
        parent.add_child(block)

        var name_text := str(info.get("name", ""))
        var trigger_text := str(info.get("trigger_label", ""))
        if name_text != "" or trigger_text != "":
            var header := HBoxContainer.new()
            header.name = "SkillHeader"
            header.add_theme_constant_override("separation", UITheme.space("space-sm"))
            block.add_child(header)
            if name_text != "":
                var name_label := UIControls.label(name_text, UITheme.font_size("font-body"), false)
                name_label.name = "SkillName"
                header.add_child(name_label)
            if trigger_text != "":
                var trigger := UIControls.pill_label(trigger_text, "info")
                trigger.name = "SkillTrigger"
                header.add_child(trigger)

        var body_text := str(info.get("text", ""))
        if body_text != "":
            var body := UIControls.label(body_text, UITheme.font_size("font-body-sm"))
            body.name = "SkillText"
            body.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
            block.add_child(body)

        if bool(info.get("is_active", false)):
            var parts := PackedStringArray()
            var cost_text := UIWording.translated_or_empty("ui.skill.cost")
            var uses_text := UIWording.translated_or_empty("ui.skill.uses")
            var cost := int(info.get("cost", -1))
            var uses := int(info.get("uses", -1))
            if cost >= 0 and cost_text != "":
                parts.append(cost_text % cost)
            if uses >= 0 and uses_text != "":
                parts.append(uses_text % uses)
            if not parts.is_empty():
                var meta := UIControls.label(" · ".join(parts), UITheme.font_size("font-caption"), false)
                meta.name = "SkillCost"
                meta.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
                block.add_child(meta)
        index += 1
