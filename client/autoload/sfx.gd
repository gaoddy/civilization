extends Node
## 对局音效：多路播放池、资源路径约定与卡牌音效回落。
##
## 资源约定：
## - 场景音效：`res://assets/audio/sfx/<cue>.wav`；
## - 卡牌音效：`res://assets/audio/cards/<cardId>/<cue>.wav`，
##   文件缺失时回落到 `res://assets/audio/cards/default/<cue>.wav`。
##
## 逐卡音效只需按目录放入文件并重新导入，不改代码；缺资源时静默，不报错。
## `cue_log` 记录每次播放尝试，供无头冒烟断言事件到音效的映射。

const CUE_REROLL := "reroll"
const CUE_FREEZE := "freeze"
const CUE_UNFREEZE := "unfreeze"
const CUE_SHOP_ENTER := "shop_enter"
const CUE_COMBAT_START := "combat_start"
const CUE_COMBAT_END := "combat_end"
const CUE_CARD_BUY := "buy"
const CUE_CARD_ATTACK := "attack"
const CUE_CARD_DEATH := "death"

const SFX_ROOT := "res://assets/audio/sfx/"
const CARD_ROOT := "res://assets/audio/cards/"
const CARD_DEFAULT_DIR := "default"

## 播放池大小：覆盖同帧多张卡阵亡、攻击与刷新叠加。
const POOL_SIZE := 8
## 公共音效统一压低，避免占位音效过响。
const VOLUME_DB := -6.0

## 每次播放尝试：`{cue, cardId, path, played}`。
var cue_log: Array = []

var _players: Array[AudioStreamPlayer] = []
var _next := 0
var _streams: Dictionary = {}


func _ready() -> void:
    for index in POOL_SIZE:
        var player := AudioStreamPlayer.new()
        player.name = "Voice%d" % index
        player.volume_db = VOLUME_DB
        add_child(player)
        _players.append(player)


## 播放场景音效（刷新、冻结、商店入场等）。
func play_cue(cue: String) -> void:
    _play(sfx_path(cue), cue, "")


## 播放与卡牌挂钩的音效；没有逐卡资源时使用默认公共音效。
func play_card(card_id: String, cue: String) -> void:
    _play(card_path(card_id, cue), cue, card_id)


## 场景音效的固定路径。
func sfx_path(cue: String) -> String:
    return SFX_ROOT + cue + ".wav"


## 卡牌音效路径：逐卡覆盖优先，其次默认目录。
func card_path(card_id: String, cue: String) -> String:
    var override := CARD_ROOT + card_id + "/" + cue + ".wav"
    if card_id != "" and ResourceLoader.exists(override):
        return override
    return CARD_ROOT + CARD_DEFAULT_DIR + "/" + cue + ".wav"


## 清空播放记录，供冒烟在单个用例内断言。
func clear_log() -> void:
    cue_log.clear()


## 当前记录中的 cue 列表。
func cues() -> Array:
    var result: Array = []
    for entry in cue_log:
        result.append(str(entry.get("cue", "")))
    return result


## 取一条 cue 的记录条数。
func count_cue(cue: String) -> int:
    var total := 0
    for entry in cue_log:
        if str(entry.get("cue", "")) == cue:
            total += 1
    return total


func _play(path: String, cue: String, card_id: String) -> bool:
    var stream := _stream(path)
    var played := stream != null
    if played:
        var player := _players[_next]
        _next = (_next + 1) % _players.size()
        player.stream = stream
        player.play()
    cue_log.append({
        "cue": cue,
        "cardId": card_id,
        "path": path,
        "played": played,
    })
    return played


## 加载并缓存音频；缺失或类型不符返回 null，不打印加载错误。
func _stream(path: String) -> AudioStream:
    if _streams.has(path):
        return _streams[path]
    if not ResourceLoader.exists(path):
        return null
    var loaded = load(path)
    if not (loaded is AudioStream):
        return null
    _streams[path] = loaded
    return loaded
