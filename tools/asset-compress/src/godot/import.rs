//! `.import` 生成与 uid 规则。
//!
//! uid 语义（阶段 0 实测，见 `output/gpu-texture-phase0/UID_VERIFICATION.md`）：
//!
//! - 省略 `uid=` → Godot 重写 `.import`、分配新 uid 并重编该资源。
//! - 自造唯一 uid → 原样接受，0 变更。
//! - 与其他资源冲突 → Godot 重编并保留冲突值，不报错。
//!
//! 因此 writer 必须写入稳定且唯一的 uid：已有 `.import` 时原样复用，否则生成
//! 并持久化。生成走 `crate::uid::UidBook::allocate`，它先扫描项目内全部
//! `.import` 的 uid、撞上已占用值时确定性加盐重算；`new_uid` 只负责由 seed
//! 派生单个候选值。

const UID_ALPHABET: &[u8; 16] = b"abcdefghijklmnop";

pub fn new_uid(seed: &str) -> String {
    let hex = crate::godot::md5::source_md5(seed.as_bytes());
    let mut out = String::from("uid://");
    for b in hex.bytes().take(13) {
        // hex 字符 0-9a-f → 16 进制索引 → Godot uid 的小写字母表
        let v = match b {
            b'0'..=b'9' => b - b'0',
            _ => b - b'a' + 10,
        };
        out.push(UID_ALPHABET[v as usize] as char);
    }
    out
}

pub fn existing_uid(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        line.strip_prefix("uid=\"")
            .and_then(|rest| rest.strip_suffix('"'))
            .map(str::to_string)
    })
}

/// `params` 必须是完整的 `[params]` 段（含段头与末尾换行）。
pub fn render_import(
    source_res: &str,
    uid: &str,
    bptc_res: &str,
    astc_res: &str,
    params: &str,
) -> String {
    format!(
        "[remap]\n\
         \n\
         importer=\"texture\"\n\
         type=\"CompressedTexture2D\"\n\
         uid=\"{uid}\"\n\
         path.bptc=\"{bptc_res}\"\n\
         path.astc=\"{astc_res}\"\n\
         metadata={{\n\
         \"imported_formats\": [\"s3tc_bptc\", \"etc2_astc\"],\n\
         \"vram_texture\": true\n\
         }}\n\
         \n\
         [deps]\n\
         \n\
         source_file=\"{source_res}\"\n\
         dest_files=[\"{bptc_res}\", \"{astc_res}\"]\n\
         \n\
         {params}"
    )
}
