# GodotSteam GDExtension 来源

- 上游：GodotSteam 4.24（GDExtension，Steamworks SDK 1.65）
- 发布：<https://codeberg.org/godotsteam/godotsteam/releases/tag/v4.24-gde>
- 包：`godotsteam-4.24-gdextension-plugin-4.4.zip`
- SHA-256：`d4358888959a03f4df0c71aafbcdca8f34b8cb9d4522aa5c8454f0e9e3debbf7`
- 兼容下限：Godot 4.4（`godotsteam.gdextension` 的 `compatibility_minimum`）

仓库只保留 macOS 渠道所需的二进制（`osx/`），Linux / Windows / Android 平台文件随
`sdl`/`ios` 等版本再补；`godotsteam.gdextension` 只声明已落盘的 macOS 平台，避免
导出其他平台时打入 0 字节库。补入新平台二进制时同步新增对应声明。
