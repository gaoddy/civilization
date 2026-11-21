class_name LoginField
extends VBoxContainer
## 登录表单一行：标题与输入框；输入框由页面读取，组件不持有登录状态。

var edit: LineEdit


func _init(title := "", value := "") -> void:
    add_child(UIControls.label(title, UITheme.font_size("font-body")))
    edit = LineEdit.new()
    edit.text = value
    add_child(edit)
