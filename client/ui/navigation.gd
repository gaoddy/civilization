class_name UINavigation
extends RefCounted
## 页面路由：场景路径常量、页面导航与错误码去向。
##
## 页面不硬编码跳转目标；新增页面时只在这里登记路径。

const PAGE_LOGIN := "res://pages/login/login.tscn"
const PAGE_LOBBY := "res://pages/lobby/lobby.tscn"
const PAGE_ROOM := "res://pages/room/room.tscn"
const PAGE_GAME := "res://pages/game/game.tscn"
const PAGE_RESULT := "res://pages/result/result.tscn"
const PAGE_GALLERY := "res://pages/gallery/gallery.tscn"
const PAGE_ACCOUNT := "res://pages/account/account.tscn"
const PAGE_LEADERBOARD := "res://pages/leaderboard/leaderboard.tscn"
const PAGE_PROFILE := "res://pages/profile/profile.tscn"

const ROUTE_LOGIN := "login"
const ROUTE_LOBBY := "lobby"
const ROUTE_NONE := ""


## 页面导航入口。change_scene_to_file 会立即把当前场景摘下，摘下后的页面仍连着
## GameStore 等 autoload 信号，回调再取场景树会拿到 null；这里以「是否仍在树上」
## 作为唯一判定，离树页面的导航直接丢弃。
static func navigate(page: Node, path: String) -> bool:
    if page == null or not is_instance_valid(page) or not page.is_inside_tree():
        return false
    var tree := page.get_tree()
    if tree == null:
        return false
    return tree.change_scene_to_file(path) == OK


## 错误码对应的页面去向：ROUTE_LOGIN、ROUTE_LOBBY 或 ROUTE_NONE（留在当前页面）。
static func error_route(code: String) -> String:
    if code in ["AUTH_FAILED", "TOKEN_EXPIRED", "SESSION_REPLACED"]:
        return ROUTE_LOGIN
    if code in ["NOT_IN_ROOM", "ROOM_NOT_FOUND"]:
        return ROUTE_LOBBY
    return ROUTE_NONE
