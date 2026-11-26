# Godot 客户端

本文是客户端技术文档的入口，列出本目录文档与代码位置；不重复正文。

| 文档 | 内容 | 代码位置 |
| --- | --- | --- |
| [Godot 客户端](client.md) | 工程结构、`GameStore`/`Net`/`CardDB` 职责、六场景、生成物与命令、运行方式、验收、桌面端与移动端导出、美术素材层与音效层 | `client/`、`Makefile` |
| [纹理登记册](textures.md) | 运行时纹理槽、铺纹理的组件、替换与新增流程、自动校验 | `client/assets/`、`scripts/validate_textures.py` |
| [客户端图片素材索引](image-asset-index.md) | 卡面与头像之外各图的页面位置、尺寸、画面目的、构图裁切与重新生成方式 | `client/assets/ui/`、`client/icon.png`、`scripts/gen_ui_assets.py` |
| [页面背景出图记录](page-background-prompts.md) | 9 个页面两层背景的模型参数、提示词原文、后处理与已知偏差 | `client/assets/ui/bg/`、`client/ui/page_backgrounds.gd` |
| [应用图标](app-icon.md) | 图标构图、矢量源、导出步骤、验收与版本记录 | `client/icon.svg`、`client/icon.png`、`client/project.godot` |
| [跨设备一致性](cross-device-consistency.md) | 桌面、iPhone 与 iPad 的布局、内容框、安全区与页面背景行为 | `client/ui/layout.gd`、`client/ui/screen_profile.gd` |
| [卡片封面美术生成](card-cover-art.md) | 已落盘卡面素材的风格基线、prompt 模板、主题符号映射与去水印约束 | `scripts/process_assets.py`、`client/assets/` |

客户端为 Godot 4 + GDScript，页面入口见 [Godot 客户端](client.md)；遗留事项见 [v0.1 待确认事项](../../../versions/v0.1/open-questions.md)。

## 相关文档

- [技术现状索引](../README.md)
- [客户端 UI](../../product/ui/README.md)
