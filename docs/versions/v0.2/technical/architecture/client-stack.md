# 客户端技术基线增量

> 状态：已实现（Android 导出、返回路由与设备判定；Android 真机对局未验证）
> 依赖：[v0.1 客户端技术基线](../../../../current/technical/architecture/client-stack.md)、[联网与技术架构](../../../../current/technical/architecture/network-architecture.md)
> 实现现状：[Godot 客户端](../../../../current/technical/client.md) §9 Android 端

本文记录 v0.2 客户端相对 v0.1 的增量：Android 端适配与验收；不重复 v0.1 已确定的引擎、语言、协议与 iOS 适配。

## 1. 目标平台

| 项 | 选择 | 说明 |
| --- | --- | --- |
| 桌面端 | 沿用 v0.1 | 不改动选型与运行边界 |
| Android 端 | 同一份 Godot 4 工程与协议 | 覆盖手机与平板；不维护第二套界面；复用 iOS 与桌面端已实现的页面、缩放与触控规则 |

## 2. 布局适配

- 桌面端、iOS 与 Android 共用同一份工程；固定横屏，不提供竖屏布局，旋转设备不重排页面。
- Android 覆盖手机与平板，沿用 iOS 已定的 960×540 手机设计基准、16:9 内容框、设备档位与安全区规则；系统返回键与软键盘处理与 iOS 一致，产品规则见[Android 端适配](../../product/ui/android-adaptation.md)。
- 规则只在服务端运行、客户端不做本地预演的约束不变。
- `ScreenProfile.is_native_touch()` 与 `safe_insets()` / `keyboard_inset()` 把 Android 纳入真机触控平台，安全区、软键盘高度与命中尺寸继续走既有 `DisplayServer` 调用，不为 Android 另写一套换算。
- Android 设备档位按屏幕短边与长宽比判定：短边不小于 600dp 或长宽比低于 1.5 归平板，其余归手机；密度取 `DisplayServer.screen_get_scale()` 与 `screen_get_dpi() / 160` 中较大的值。iOS 仍按机型前缀与屏幕比例判定。
- `project.godot` 用 `display/window/handheld/orientation=4`（`SCREEN_SENSOR_LANDSCAPE`）锁定左右横屏；`application/config/quit_on_go_back=false` 关闭引擎默认的「返回即退出」；`input_devices/pointing/emulate_mouse_from_touch=true` 保留触摸到鼠标的 GUI 映射，`emulate_touch_from_mouse=false`（桌面模拟触控档位由 `ScreenProfile` 在运行时开启）。

## 3. 系统返回键与返回手势

- autoload `SystemBack` 监听 `NOTIFICATION_WM_GO_BACK_REQUEST`，按顺序路由：
  1. 可安全关闭的浮层先收起：`FriendsPanel`、`GalleryFilterPanel`、`GalleryDetailLayer`；
  2. 当前场景声明 `system_back() -> bool` 时调用：账号页执行常驻返回按钮动作，图鉴页回大厅，对局页收起图鉴覆盖层；
  3. 以上都没有时忽略，不退出应用。
- 邀请弹窗、匹配排队浮层、发现 / 进化选择与状态模态含待处理业务状态，返回键不代替用户做出选择。
- 大厅没有返回按钮，也没有页面返回动作，返回键在大厅忽略；对局页除图鉴覆盖层外不执行返回，避免误触退出对局。

## 4. 验收

- Android 手机与平板各可完成完整对局；
- Android 与桌面端、iOS 在会战与对局控制流上行为一致，差异只允许存在于布局与输入。
- 验证记录（2026-11-23，本机 macOS + Godot 4.7.2 + Android SDK 35 / 36）：`make client-check` 退出码 0（目标内包含 `client-import`）；APK 与 AAB 导出链路见 §5 的实测结果；Android 手机与平板真机、系统返回手势与软键盘手测未验证（本机无已连接 Android 设备）。

## 5. 导出与分发

- `client/export_presets.cfg` 新增两个预设：`Android`（APK，`gradle_build/export_format=0`，沿用导出模板的 SDK 版本）与 `Android AAB`（`gradle_build/export_format=1`，走 Gradle 构建，`minSdk 24`、`targetSdk 35`）。两者共用包名 `com.civilization.banner`、应用名 `Civilization`、arm64-v8a、沉浸式与 edge-to-edge 屏幕、`package/signed=true`。
- `make client-export PLATFORM=android` 产出 `output/client/Civilization-android-<构建号>.apk`；`make client-export PLATFORM=android AAB=1` 产出 `output/client/Civilization-android-<构建号>-aab.aab`。
- 签名参数只从环境变量读取：`ANDROID_KEYSTORE_PATH`、`ANDROID_KEYSTORE_USER`（alias）、`ANDROID_KEYSTORE_PASSWORD`；任一缺失或 keystore 文件不存在时目标直接失败，不产出未签名包。口令不写入仓库或命令行。
- Godot 只从工程内的 `export_presets.cfg` 读取签名配置，`scripts/android_export.sh` 在导出期间把环境变量临时写入预设，退出时（含失败）用备份恢复，仓库文件不保留口令。AAB 首次导出时临时安装 Gradle 构建模板，导出后清理生成的 `client/android/`。
- 两种产物使用同一自管理 keystore 签名，跨渠道安装同一应用；keystore 与签名口令不进入仓库。
- 导出前置条件：Godot 编辑器设置 `export/android/java_sdk_path` 必须指向 JDK 17+（本机系统 Java 8 不满足，验证时使用 Android Studio 自带 JBR 21），并配置 `export/android/android_sdk_path`；APK 需要 `android_release.apk` 模板，AAB 另需 `android_source.zip`。导出目标不自动修改编辑器设置。
- 已知问题：`client/addons/godotsteam/godotsteam.gdextension` 声明了 Android / Linux / Windows 库路径，但仓库只保留 macOS 二进制；Android 导出因此包含 0 字节的 `libgodotsteam*.so` 与 `libsteam_api.so`，并打印无法打开对应的错误。不影响 APK / AAB 产出；Android 启动时会尝试加载该扩展并失败，当前 Android 预设不是 Steam 渠道、业务代码不取用 Steam 单例，实际运行影响未验证，需在后续决定是否移除非 macOS 声明或按平台排除该扩展。
- 本机实测：APK 与 AAB 均成功产出，签名与构建链路可用；未在真实 Android 设备上安装与完成对局。

## 相关文档

- [v0.1 客户端技术基线](../../../../current/technical/architecture/client-stack.md)
- [Android 端适配](../../product/ui/android-adaptation.md)
