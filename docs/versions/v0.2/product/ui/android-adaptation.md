# Android 端适配

> 状态：方案未实施
> 依赖：[移动端适配（iOS）](../../../../current/product/ui/mobile-adaptation.md)、[客户端技术基线](../../technical/architecture/client-stack.md)

本文记录 v0.2 的 Android 端相对 iOS 已实现适配的增量：目标设备、系统交互、分发形态与验收标准；不重复 iOS 已定的页面、缩放与触控规则。

## 1. 目标设备与方向

- 覆盖 Android 手机与平板，档位与 iOS 的 iPhone、iPad 对齐。
- 与 iOS 一致固定横屏，支持左横与右横；不提供竖屏布局，旋转设备不重排页面。
- 与桌面端、iOS 共用同一套页面、规则与服务端；不做 Android 独立工程。

## 2. 系统交互

- 系统返回键与返回手势执行与页面常驻返回按钮相同的返回动作；当前页面没有返回按钮时不执行返回，不退出应用。
- 软键盘弹出时，页面底部让出键盘越过内容框底边的高度；让位与恢复规则与 iOS 一致。
- 触控映射、命中尺寸与安全区沿用 iOS 已定规则。

## 3. 分发形态

- 同时提供 APK 侧载与 Google Play AAB 两种产物。
- 两种产物使用同一自管理 keystore 签名，同一应用可跨渠道覆盖安装与升级。

## 4. 验收标准

- Android 手机与平板各可完成完整对局，不出现竖屏布局或页面裁切。
- 系统返回键、返回手势与软键盘行为符合第 2 节。
- APK 与 AAB 产物均可安装并完成完整对局。

## 相关文档

- [移动端适配（iOS）](../../../../current/product/ui/mobile-adaptation.md)
- [客户端技术基线](../../technical/architecture/client-stack.md)
