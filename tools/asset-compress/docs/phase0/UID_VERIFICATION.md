# uid 行为验证（Godot 4.7.2）

日期：2026-11-23
环境：worktree `.worktrees/feature-gpu-texture-compress`，从主仓复制 `.godot/imported` 与 `client/**/*.import`，导入命令 `Godot --headless --path client --import`。
判定方法：导入前后对比 `client/.godot/imported/**` 的 mtime(ns) 与 md5，以及目标 `.import` 是否被重写。稳定态基线：2.47–2.58s，0 个文件变更。

## 结论

**writer 必须写入唯一且稳定的 uid，不能省略。**

| 场景 | `.import` | `.ctex` | 耗时 | uid |
| --- | --- | --- | --- | --- |
| 稳态（不动） | 不重写 | 0 变更 | 2.5s | 保持 |
| 删除 `uid=` 行 | **重写** | 重编 2 个 + 重写 `.md5`（字节相同） | 2.92s（+0.34s） | **改为一个新生成的 uid**（与旧值不同） |
| 自造唯一 uid `uid://a1b2c3d4e5f6g` | 不重写 | 0 变更 | 2.33s | 原样保留 |
| uid 改成另一张卡的 uid | 重写 | 重编 2 个 + 重写 `.md5` | 3.14s | **保留冲突 uid，无报错** |

## 对 writer 的约束

1. `.import` 已存在：保留其 `uid`，其余字段按现有模板；内容无变化时不写文件。
2. `.import` 不存在（全新 checkout）：生成 13 字符 `uid://<base32>` 并持久化；后续运行必须复用同一个值（写进 `.import` 后即从文件读取）。
3. 生成前扫描项目内已有 `.import` 的 uid，避免重复：冲突不会报错，只会让 Godot 重编该资源并留下歧义引用。
4. 删除 uid 后 Godot 分配的新 uid 与旧值不同，会打断场景/脚本里对旧 uid 的引用，因此「先让 Godot 生成一次 .import」的混合模式也可行，但 writer 自己的 uid 已足够。

## 附带确认

- 同一源 PNG 重复导入两次，`.ctex` 字节完全相同（md5 一致），说明 Godot 的 CPU 编码是确定性的；这也意味着阶段 1 的 golden 对拍可行。
- 只有目标资源的 3 个文件（`.bptc.ctex`、`.astc.ctex`、`.md5`）变化，说明判重粒度是单资源。
