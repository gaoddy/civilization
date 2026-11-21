class_name UIFormat
extends RefCounted
## 界面格式化：把协议里的字符串转成可计算的本地值。


## RFC3339 时间转 Unix 秒；共享给需要按 `phaseDeadline` 反推进度的视图。
static func unix_from_rfc3339(value: String) -> float:
    var text := value
    if text.ends_with("Z"):
        text = text.substr(0, text.length() - 1)
    var dot := text.find(".")
    if dot >= 0:
        text = text.substr(0, dot)
    return Time.get_unix_time_from_datetime_string(text)
