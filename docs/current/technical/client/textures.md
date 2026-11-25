# 纹理登记册

> 状态：已实现

本文登记客户端运行时消费的全部纹理槽，以及每个把纹理铺到界面上的组件；整体替换视觉资源时按本表逐项覆盖。纹理的生成、压缩与导入流程见 [Godot 客户端](client.md#10-美术素材层)。新增或替换纹理后运行 `make data-check`，未登记的源码、资源或槽位会让校验失败。

## 1. 纹理槽

| 槽位 | 资源 | 像素尺寸 | 取用入口 | 无图回退 |
| --- | --- | --- | --- | --- |
| 软件背景 | `client/assets/ui/bg/bg_main.png` | 512×512 | `UIControls.background()` | `color-bg` 纯色 |
| 对局分区背景（桌面） | `client/assets/ui/bg/bg_game_desktop.png` | 1280×720 | `game.gd::_page_texture()` → `UILayout.safe_area(page, path)` | 统一语义面板 `panel_style("page")` |
| 对局分区背景（手机） | `client/assets/ui/bg/bg_game_phone.png` | 960×540 | 同上，`ScreenProfile.device_kind()` 为 `DEVICE_PHONE` 时取此图 | 同上 |
| 大厅主视觉 | `client/assets/ui/lobby/lobby_hall.png` | 1600×1000 | `LobbyHallImage.HALL_PATH` | `sunken` 面板底色 |
| 卡面 | 每张卡 `client/assets/cards/faces/card_<id>_face.png`，清单键 `card_face` | 448×584 | `CardDB.art_path(id, "card_face")` | 玩法色封面 + 首字 |
| 头像 | 每个英雄 `client/assets/characters/avatars/ch_<id>_ava.png`，清单键 `avatar` | 256×256 | `CardDB.art_path(id, "avatar")` | 玩法色封面 + 首字 |
| 售货员头像 | `client/assets/characters/avatars/seller_ava.png` | 256×256 | `GameShopView.SELLER_ART` | 玩法色封面 + 首字 |
| 立绘（不渲染） | 每个英雄 `client/assets/characters/portraits/ch_<id>_full.png`，清单键 `portrait` | 1024×1536 | 运行时不取用；`make assets` 由它派生头像与卡面 | — |

- 卡面与头像的路径和展示尺寸登记在 `client/assets/asset_manifest.json`，`CardDB` 启动时加载为 `art`，提供 `has_art(id)` 与 `art_path(id, kind)`。
- `portrait` 是派生源而非运行时纹理：当前没有组件调用 `art_path(id, "portrait")`；替换源立绘后重跑 `make assets` 即可更新头像与卡面。
- 卡面展示框宽高比固定 112:146，只给宽度时高度由 `UITheme.card_face_height(width)` 计算；头像位不参与卡面比例，方形头像统一由 `AvatarFrame` 按 `radius-md` 裁切。

## 2. 组件登记

| 组件 | 入口 | 槽位 | 纹理节点 | 铺法 | 源码 | 使用位置 |
| --- | --- | --- | --- | --- | --- | --- |
| 共享取图 | `UIArt.texture` / `UIArt.layer` | 全部 | 由调用方命名 | 由调用方指定 | `client/ui/art.gd` | 下列所有组件 |
| 软件背景 | `UIControls.background` | 软件背景 | `PageBackground` | `STRETCH_KEEP_ASPECT_COVERED` | `client/ui/controls.gd` | 登录、大厅、房间、对局、图鉴、结算、排行榜、账号、个人资料共 9 个页面 |
| 对局页面背景 | `UIControls.page_surface` | 对局分区背景 | `PageSurface` | `STRETCH_SCALE` | `client/ui/controls.gd` | 对局页，由 `UILayout.safe_area` 传入设备档位图 |
| 大厅主视觉 | `LobbyHallImage` | 大厅主视觉 | `HallTexture` | `STRETCH_KEEP_ASPECT_COVERED` | `client/pages/lobby/lobby_hall_image.gd` | 大厅左栏 |
| 卡面 | `CardFaceBase.create_face` | 卡面 | `CoverArt` | `STRETCH_KEEP_ASPECT_COVERED` | `client/ui/card_face_base.gd` | `ShopCardFace`、`HandCardFace`、`BoardCardFace`、`GalleryCardFace`（图鉴网格与图鉴英雄视图） |
| 图鉴详情封面 | `GalleryDetail._build_preview` | 卡面 | `GalleryDetailArt` | `STRETCH_KEEP_ASPECT_COVERED` | `client/ui/gallery_detail.gd` | 图鉴详情浮层 |
| 选英雄候选卡 | `HeroPickCard._build` | 卡面 | `CoverArt` | `STRETCH_KEEP_ASPECT_COVERED` | `client/pages/game/hero_pick_card.gd` | 选英雄覆盖层 |
| 头像框 | `AvatarFrame.set_art` | 头像 | `AvatarFrameArt` | `STRETCH_KEEP_ASPECT_COVERED` | `client/ui/avatar_frame.gd` | 以下头像组件 |
| 头像块 | `Avatar.create` | 头像 | `AvatarFrameArt` | `STRETCH_KEEP_ASPECT_COVERED` | `client/ui/avatar.gd` | 对局左侧座位列表 |
| 对局英雄卡 | `GameHeroCard.refresh` | 头像 | `AvatarFrameArt` | `STRETCH_KEEP_ASPECT_COVERED` | `client/pages/game/game_hero_card.gd` | 英雄行、战斗对手身份块 |
| 对局头像块 | `GamePortrait.setup` | 头像 | `AvatarFrameArt` | `STRETCH_KEEP_ASPECT_COVERED` | `client/pages/game/game_portrait.gd` | 战斗开场两侧、商店售货员 |
| 售货员入口 | `GameShopView.SELLER_ART` | 售货员头像 | `AvatarFrameArt` | `STRETCH_KEEP_ASPECT_COVERED` | `client/pages/game/game_shop_view.gd` | 商店操作行 |

- 卡面纹理向内收 `card-cover-inset`（3px），方形四角落在卡框圆角以内；有卡面纹理时隐藏首字水印，无图时保留玩法色封面与首字。
- 头像纹理由 `AvatarFrame.set_art` 铺在裁切层索引 0，随框的 `radius-md` 一起裁到圆角；有图时隐藏首字，路径为空或加载失败时恢复首字。
- 战斗开场侧栏在 `client/pages/game/combat_intro.gd` 取 `avatar` 并调用 `GamePortrait.setup`。
- 房间座位格与大厅玩家卡只显示名字首字，不消费纹理，不在登记范围。

## 3. 页面挂载点

| 挂载点 | 纹理节点 | 源码 |
| --- | --- | --- |
| 软件背景 | `PageBackground` | `client/ui/card_gallery.gd`、`client/pages/account/account.gd`、`client/pages/gallery/gallery.gd`、`client/pages/game/game.gd`、`client/pages/leaderboard/leaderboard.gd`、`client/pages/lobby/lobby.gd`、`client/pages/login/login.gd`、`client/pages/profile/profile.gd`、`client/pages/result/result.gd`、`client/pages/room/room.gd` |
| 页面容器背景 | `PageSurface` | `client/ui/layout.gd` |

## 4. 替换与新增

1. 换软件背景、对局分区背景或大厅主视觉：替换同名文件 → `make asset-compress` → `make client-import`，再按第 2 节对应的使用位置检查。
2. 换卡面：人物与随从走 `make assets`，效果牌走 `make assets-covers`，两者都会重写 `asset_manifest.json` 与 PNG；随后 `make asset-compress`、`make client-import`。
3. 换头像：替换 `characters/avatars/ch_<id>_ava.png`，或替换源立绘后重跑 `make assets`。
4. 新增纹理组件或槽位：先在第 1、2、3 节补行，再运行 `make data-check`；校验通过后才算完成。

## 5. 自动校验

`scripts/validate_textures.py` 随 `make data-check` 执行，扫描 `client/**/*.gd`（排除 `client/tests/` 与 `client/.godot/`）：

- 纹理入口：`UIArt.texture`、`UIArt.layer`、`UIControls.background`、`UIControls.page_surface`、`CardDB.art_path`、`TextureRect.new`、`set_art`。
- 资源字面量：`res://assets/**/*.png`。
- 清单槽位：`CardDB.art_path(id, "<kind>")` 的 `kind`。

任一源码路径、资源路径或槽位名未出现在本文，脚本以非零退出码结束并列出缺失项；本文登记的源码路径与静态资源路径必须真实存在。`client/tests/` 下的测试与预览脚本不参与登记。

## 相关文档

- [Godot 客户端](client.md)：素材生成、纹理压缩、导入与渲染后端。
- [卡片封面美术生成](card-cover-art.md)：卡面风格基线、prompt 模板与去水印约束。
- [跨设备一致性](cross-device-consistency.md)：页面背景与内容框在多种尺寸下的行为。
