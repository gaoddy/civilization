extends Node
## 只读卡牌展示表：启动时从服务端下载并按内容版本缓存，不参与任何规则判定。
##
## 数据来自服务端 `GET /content`；文本（卡名、技能文案、时代名）仍由本地文本表提供。
## 未取得数据时查询回退为空字典，界面按 id 展示；页面可监听 `content_changed` 重建。

## 内容可用或更新后发出；页面据此重建依赖卡牌数据的列表。
signal content_changed
## 无缓存且请求失败时发出，参数为失败原因。
signal content_failed(reason: String)

const WsClientScript = preload("res://autoload/ws_client.gd")

## 本地调试、冒烟、预览与线框图用：直接读该目录下的内容文件，不发请求。
const CONTENT_DIR_ENV := "CIVILIZATION_CONTENT_DIR"
const CONTENT_FILE := "client-content.json"
const CACHE_PATH := "user://content_cache.json"
const CACHE_NAME := "content_cache.json"
const CACHE_TMP_NAME := "content_cache.json.tmp"
const REQUEST_TIMEOUT_SECONDS := 8.0

enum State { LOADING, READY, FAILED }

var cards: Dictionary = {}
## 英雄展示表：只含身份与技能；与 `cards` 中的同名随从卡分属两个集合。
var heroes: Dictionary = {}
## 技能展示表：只取费用等展示字段，不参与规则判定。
var skills: Dictionary = {}
## 科学分类树：节点 id → {id, parent, rank}。
var taxonomy: Dictionary = {}
## 玩法 id 顺序，与 taxonomy.md §2 一致，供图鉴分类目录使用。
var playstyles: Array = []
## 美术资源清单：card_id → {portrait, avatar, card_face, display}。缺图时回退程序化绘制。
var art: Dictionary = {}

var _state := State.LOADING
var _version := ""
## 缓存可用但刷新失败：保留缓存内容继续展示。
var _stale := false
var _error := ""
var _http: HTTPRequest = null
var _in_flight := false
var _net_hooked := false

## 时代序号到文本 id，与服务端 `Era` 枚举一致（1–7）。
const ERA_IDS := {
    1: "prehistoric",
    2: "primitive",
    3: "classical",
    4: "feudal",
    5: "industrial",
    6: "modern",
    7: "future",
}


func _ready() -> void:
    _load_art("res://assets/asset_manifest.json")
    var override_dir := OS.get_environment(CONTENT_DIR_ENV).strip_edges()
    if override_dir != "":
        _load_from_dir(override_dir)
        return
    _load_cache()
    _setup_http()
    call_deferred("_hook_net")
    fetch()


## 数据是否可用（来自网络或缓存）。
func is_loaded() -> bool:
    return _state == State.READY


## 无任何可用数据且请求失败。
func has_failed() -> bool:
    return _state == State.FAILED


## 当前展示的是缓存，最近一次刷新失败。
func is_stale() -> bool:
    return _stale


func last_error() -> String:
    return _error


func content_version() -> String:
    return _version


## 请求最新内容；已有请求在途或使用本地目录时忽略。
func fetch() -> void:
    if _http == null or _in_flight:
        return
    var headers := PackedStringArray(["Accept: application/json"])
    if _version != "":
        headers.append("If-None-Match: \"%s\"" % _version)
    _in_flight = true
    var err := _http.request(content_url(), headers, HTTPClient.METHOD_GET)
    if err != OK:
        _in_flight = false
        _fail("request_failed:%d" % err)


## 内容地址：由 WebSocket 地址推导，`ws://` → `http://`、`wss://` → `https://`。
static func content_url_for(ws_url: String) -> String:
    var base := ws_url.strip_edges()
    if base.ends_with("/ws"):
        base = base.substr(0, base.length() - 3)
    if base.begins_with("wss://"):
        base = "https://" + base.substr(6)
    elif base.begins_with("ws://"):
        base = "http://" + base.substr(5)
    return base.trim_suffix("/") + "/content"


func content_url() -> String:
    return content_url_for(WsClientScript.default_url())


func _load_from_dir(dir: String) -> void:
    var path := dir.trim_suffix("/") + "/" + CONTENT_FILE
    var file := FileAccess.open(path, FileAccess.READ)
    if file == null:
        _fail("content_read_failed:%s" % path)
        return
    if not _apply(file.get_as_text()):
        _fail("content_format_error:%s" % path)


func _load_cache() -> void:
    if not FileAccess.file_exists(CACHE_PATH):
        return
    var text := FileAccess.get_file_as_string(CACHE_PATH)
    if text == "" or not _apply(text):
        _error = "cache_corrupt"


func _setup_http() -> void:
    _http = HTTPRequest.new()
    _http.timeout = REQUEST_TIMEOUT_SECONDS
    _http.request_completed.connect(_on_request_completed)
    add_child(_http)


## 连接建立后补拉一次：启动时服务端未就绪或请求失败时由这里恢复。
func _hook_net() -> void:
    if _net_hooked:
        return
    var net := get_node_or_null("/root/Net")
    if net == null or not net.has_signal("connected"):
        return
    net.connected.connect(_on_net_connected)
    _net_hooked = true


func _on_net_connected() -> void:
    if _state != State.READY or _stale:
        fetch()


func _on_request_completed(
    result: int, response_code: int, _headers: PackedStringArray, body: PackedByteArray
) -> void:
    _in_flight = false
    if result != HTTPRequest.RESULT_SUCCESS:
        _fail("connect_failed:%d" % result)
        return
    if response_code == HTTPClient.RESPONSE_NOT_MODIFIED:
        _state = State.READY
        _stale = false
        _error = ""
        content_changed.emit()
        return
    if response_code != HTTPClient.RESPONSE_OK:
        _fail("HTTP %d" % response_code)
        return
    var text := body.get_string_from_utf8()
    if not _apply(text):
        _fail("content_format_error")
        return
    _write_cache(text)


## 应用下发内容；格式不完整时返回 false 且不改动已有数据。
func _apply(text: String) -> bool:
    var parsed = JSON.parse_string(text)
    if not (parsed is Dictionary):
        return false
    var bundle: Dictionary = parsed
    var card_list = bundle.get("cards")
    var hero_list = bundle.get("heroes")
    var skill_list = bundle.get("skills")
    var taxonomy_data = bundle.get("taxonomy")
    if not (card_list is Array and hero_list is Array and skill_list is Array):
        return false
    if not (taxonomy_data is Dictionary):
        return false

    cards.clear()
    for entry in card_list:
        if entry is Dictionary:
            cards[str(entry.get("id", ""))] = entry
    heroes.clear()
    for entry in hero_list:
        if entry is Dictionary:
            heroes[str(entry.get("id", ""))] = entry
    skills.clear()
    for entry in skill_list:
        if entry is Dictionary:
            skills[str(entry.get("id", ""))] = entry
    taxonomy.clear()
    for node in taxonomy_data.get("nodes", []):
        if node is Dictionary:
            taxonomy[str(node.get("id", ""))] = node
    playstyles.clear()
    for entry in taxonomy_data.get("playstyles", []):
        if entry is String:
            playstyles.append(entry)

    _version = str(bundle.get("version", ""))
    _state = State.READY
    _stale = false
    _error = ""
    content_changed.emit()
    return true


func _write_cache(text: String) -> void:
    var file := FileAccess.open(CACHE_TMP_NAME, FileAccess.WRITE)
    if file == null:
        return
    file.store_string(text)
    file.close()
    var dir := DirAccess.open("user://")
    if dir == null:
        return
    dir.remove(CACHE_NAME)
    dir.rename(CACHE_TMP_NAME, CACHE_NAME)


func _fail(reason: String) -> void:
    _error = reason
    _stale = _state == State.READY
    if _state != State.READY:
        _state = State.FAILED
    push_warning("卡牌内容不可用：%s" % reason)
    content_failed.emit(reason)


## 美术清单是单对象 `{version, assets}`；按 card_id 建索引。文件缺失不报错，
## 让全部界面回退到程序化绘制（无素材时游戏照常可玩）。
func _load_art(path: String) -> void:
    if not FileAccess.file_exists(path):
        return
    var file := FileAccess.open(path, FileAccess.READ)
    if file == null:
        return
    var parsed = JSON.parse_string(file.get_as_text())
    if not (parsed is Dictionary):
        return
    for id in parsed.get("assets", {}):
        if parsed["assets"][id] is Dictionary:
            art[str(id)] = parsed["assets"][id]


func card(id: String) -> Dictionary:
    return cards.get(id, {})


## 是否进入商店卡池、共享牌池、三连与发现；缺字段按进池处理。
func in_shop_pool(entry: Dictionary) -> bool:
    return bool(entry.get("in_shop_pool", true))


## 英雄卡条目；英雄只用于开局选人，不参与商店与牌池。
func hero(id: String) -> Dictionary:
    return heroes.get(id, {})


func is_hero(id: String) -> bool:
    return heroes.has(id)


func skill(id: String) -> Dictionary:
    return skills.get(id, {})


## 技能展示费用；技能不存在、缺少 `cost` 或 `cost` 为 null（触发式/常驻技能）时返回 -1，
## 供界面隐藏费用角标。
func skill_cost(id: String) -> int:
    var value: Variant = skill(id).get("cost")
    if value == null:
        return -1
    return int(value)


## 技能展示信息：名称、文本、触发、费用与次数，供技能列表逐条渲染。
## 不参与规则判定；缺失的数值字段返回 -1。
func skill_info(skill_id: String) -> Dictionary:
    var entry := skill(skill_id)
    if entry.is_empty():
        return {
            "name": "", "text": "", "trigger": "", "trigger_label": "",
            "cost": -1, "uses": -1, "is_active": false, "exists": false,
        }
    var trigger := ""
    for effect in entry.get("effects", []):
        if effect is Dictionary and str(effect.get("trigger", "")) != "":
            trigger = str(effect.get("trigger", ""))
            break
    var keyword: Variant = entry.get("keyword")
    var has_keyword := keyword != null and str(keyword) != ""
    var trigger_label := ""
    if trigger != "":
        trigger_label = trigger_name(trigger)
    elif has_keyword:
        trigger_label = trigger_name("")
    else:
        var passive_key := "ui.skill.trigger.passive"
        var passive := tr(passive_key)
        trigger_label = "" if passive == passive_key else passive
    var text_key := "skill.%s.text" % skill_id
    var text := tr(text_key)
    if text == text_key:
        text = ""
    return {
        "name": skill_name(skill_id),
        "text": text,
        "trigger": trigger,
        "trigger_label": trigger_label,
        "cost": int(entry.get("cost")) if entry.get("cost") != null else -1,
        "uses": int(entry.get("uses_per_turn")) if entry.get("uses_per_turn") != null else -1,
        "is_active": trigger == "on_activate",
        "exists": true,
    }


## 触发类型标签：读 `ui.skill.trigger.<trigger>`；空触发视为关键词技能，
## 文案键为 `ui.skill.trigger.keyword`。未知触发不冒充常驻，回退
## `ui.skill.trigger.unknown`；标签缺键时返回空串，避免内部 id 泄露到界面。
func trigger_name(trigger: String) -> String:
    var key := "ui.skill.trigger.%s" % ("keyword" if trigger == "" else trigger)
    var text := tr(key)
    if text != key:
        return text
    if trigger == "":
        return ""
    var unknown_key := "ui.skill.trigger.unknown"
    var unknown := tr(unknown_key)
    return "" if unknown == unknown_key else unknown


## 展示名：取 `card.<id>.name`；缺键回退为实体 id。
func display_name(id: String) -> String:
    var key := "card.%s.name" % id
    var text := tr(key)
    return id if text == key else text


## 英雄介绍：取 `card.<id>.introduction`；缺键返回空串。
func hero_introduction(id: String) -> String:
    var key := "card.%s.introduction" % id
    var text := tr(key)
    return "" if text == key else text


## 技能文本按技能顺序拼接；缺键的技能跳过。
## `golden` 为真时优先取金色文案，缺键回落普通文案。
func display_text(id: String, golden := false) -> String:
    return _skill_text(card(id).get("skills", []), golden)


## 英雄技能文本：只读英雄卡自己的技能列表，不回落到同名随从卡。
func hero_text(id: String) -> String:
    return _skill_text(hero(id).get("skills", []))


func _skill_text(skill_ids: Array, golden := false) -> String:
    var lines: Array = []
    for skill_id in skill_ids:
        var key := "skill.%s.text%s" % [str(skill_id), ".golden" if golden else ""]
        var line := tr(key)
        if line == key and golden:
            key = "skill.%s.text" % str(skill_id)
            line = tr(key)
        if line == key:
            continue
        lines.append(line)
    return "\n".join(lines)


func display_cost(id: String) -> int:
    return int(card(id).get("display_cost", 0))


## 技能展示名：取 `skill.<id>.name`，缺键返回空串，避免内部 id 泄露到界面。
func skill_name(id: String) -> String:
    if id == "":
        return ""
    var key := "skill.%s.name" % id
    var text := tr(key)
    return "" if text == key else text


## 时代展示名；未知序号回退为数字文本，缺键回退为文本 id。
func era_name(era: int) -> String:
    var id: String = ERA_IDS.get(era, "")
    if id == "":
        return str(era)
    var key := "enum.era.%s" % id
    var text := tr(key)
    return id if text == key else text


## 玩法展示名；空 id 返回空串，缺键回退为 id。
func playstyle_name(id: String) -> String:
    if id == "":
        return ""
    var key := "enum.playstyle.%s" % id
    var text := tr(key)
    return id if text == key else text


## 玩法归属：读 `categories` 的 `play` 条目，按玩法顺序去重排列。
## 缺失 `categories`（非池卡）时返回空，由调用方显示「未定」。
func play_categories(entry: Dictionary) -> Array[String]:
    var values := category_values(entry, "play")
    var out: Array[String] = []
    for id in playstyles:
        var playstyle := str(id)
        if values.has(playstyle) and not out.has(playstyle):
            out.append(playstyle)
    for playstyle in values:
        if not out.has(playstyle):
            out.append(playstyle)
    return out


## 主玩法：取第一条 `play:`，供取色与单值展示。
## 卡表缺失玩法时返回 `neutral`，与服务端筛选哨兵一致。
func primary_playstyle(entry: Dictionary) -> String:
    var plays := play_categories(entry)
    return plays[0] if not plays.is_empty() else "neutral"


## 关联分类取值：`kind` 为 play / prey / host，保持数据顺序。
func category_values(entry: Dictionary, kind: String) -> Array[String]:
    var out: Array[String] = []
    var categories: Variant = entry.get("categories", [])
    if not (categories is Array):
        return out
    for item in categories:
        if not (item is Dictionary):
            continue
        if str(item.get("kind", "")) != kind:
            continue
        var value := str(item.get("value", ""))
        if value != "":
            out.append(value)
    return out


## 科学分类节点展示名；空 id 返回空串，缺键回退为 id。
func taxon_name(id: String) -> String:
    if id == "":
        return ""
    var key := "enum.taxon.%s" % id
    var text := tr(key)
    return id if text == key else text


## 从叶子沿 parent 上溯到根，返回「界 → 科」顺序的展示名数组。
## 最多 7 层；未知节点或空 id 返回空数组。
func taxon_path(id: String) -> Array:
    var names: Array = []
    var current := id
    while current != "" and taxonomy.has(current) and names.size() < 7:
        names.append(taxon_name(current))
        var parent = taxonomy[current].get("parent")
        current = "" if parent == null else str(parent)
    names.reverse()
    return names


## 全部英雄卡 id，按展示名排序。
func heroes_array() -> Array:
    var out: Array = []
    for id in heroes:
        out.append(str(id))
    out.sort_custom(func(a: String, b: String) -> bool:
        var a_name := display_name(a)
        var b_name := display_name(b)
        if a_name != b_name:
            return a_name < b_name
        return a < b)
    return out


## 该卡是否已有美术资源；无资源时调用方应回退程序化绘制。
func has_art(id: String) -> bool:
    return art.has(id)


## 取某类资源路径（kind: portrait / avatar / card_face）；无资源返回空串。
func art_path(id: String, kind: String) -> String:
    if not art.has(id):
        return ""
    return str(art[id].get(kind, ""))
