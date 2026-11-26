class_name PageBackgrounds
extends RefCounted
## 页面背景图映射：外层氛围图与内容区结构框架图按页面名集中登记。
##
## 外层图整窗 cover，只提供氛围，不承载结构；内容区图是 16:9 结构框架，
## 必须与该页控件矩形对齐。图片缺失时返回空串，调用方回退到统一底图与语义面板。
## 手机档位布局比例另有差异的页面登记到 PHONE_SURFACE，未登记时复用桌面图。

const AMBIENT := {
    "login": "res://assets/ui/bg/bg_login.png",
    "lobby": "res://assets/ui/bg/bg_lobby.png",
    "room": "res://assets/ui/bg/bg_room.png",
    "game": "res://assets/ui/bg/bg_game.png",
    "result": "res://assets/ui/bg/bg_result.png",
    "gallery": "res://assets/ui/bg/bg_gallery.png",
    "leaderboard": "res://assets/ui/bg/bg_leaderboard.png",
    "profile": "res://assets/ui/bg/bg_profile.png",
    "account": "res://assets/ui/bg/bg_account.png",
}

const SURFACE := {
    "login": "res://assets/ui/bg/bg_login_surface.png",
    "lobby": "res://assets/ui/bg/bg_lobby_surface.png",
    "room": "res://assets/ui/bg/bg_room_surface.png",
    "game": "res://assets/ui/bg/bg_game_desktop.png",
    "result": "res://assets/ui/bg/bg_result_surface.png",
    "gallery": "res://assets/ui/bg/bg_gallery_surface.png",
    "leaderboard": "res://assets/ui/bg/bg_leaderboard_surface.png",
    "profile": "res://assets/ui/bg/bg_profile_surface.png",
    "account": "res://assets/ui/bg/bg_account_surface.png",
}

const PHONE_SURFACE := {
    "lobby": "res://assets/ui/bg/bg_lobby_surface_phone.png",
    "game": "res://assets/ui/bg/bg_game_phone.png",
}


static func ambient(page: String) -> String:
    return _existing(String(AMBIENT.get(page, "")))


static func surface(page: String) -> String:
    if ScreenProfile.device_kind() == ScreenProfile.DEVICE_PHONE:
        var phone := _existing(String(PHONE_SURFACE.get(page, "")))
        if phone != "":
            return phone
    return _existing(String(SURFACE.get(page, "")))


static func _existing(path: String) -> String:
    if path != "" and ResourceLoader.exists(path):
        return path
    return ""
