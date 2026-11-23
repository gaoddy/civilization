//! 单图端到端：PNG → BC7 + ASTC → `.ctex` / `.md5` / `.import`。
//!
//! 产物布局与 Godot 一致：导入文件名 `<源文件名>-<md5(res 路径)>.<fmt>.ctex`，
//! 放在项目的 `.godot/imported/`；`.import` 写在源 PNG 旁边。写入先落临时文件
//! 再 rename，避免中途失败留下半截产物。

use std::io;
use std::path::{Path, PathBuf};

use crate::encode::{EncodeError, Encoder};
use crate::format::Format;
use crate::godot::{ctex, import, md5};

/// 没有现成 `.import` 时使用的 Godot 4 默认纹理参数（与现网产物一致）。
pub const DEFAULT_PARAMS: &str = "[params]\n\n\
compress/mode=2\n\
compress/high_quality=true\n\
compress/lossy_quality=0.7\n\
compress/uastc_level=0\n\
compress/rdo_quality_loss=0.0\n\
compress/hdr_compression=1\n\
compress/normal_map=0\n\
compress/channel_pack=0\n\
mipmaps/generate=false\n\
mipmaps/limit=-1\n\
roughness/mode=0\n\
roughness/src_normal=\"\"\n\
process/channel_remap/red=0\n\
process/channel_remap/green=1\n\
process/channel_remap/blue=2\n\
process/channel_remap/alpha=3\n\
process/fix_alpha_border=true\n\
process/premult_alpha=false\n\
process/normal_map_invert_y=false\n\
process/hdr_as_srgb=false\n\
process/hdr_clamp_exposure=false\n\
process/size_limit=0\n\
detect_3d/compress_to=1\n";

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Image(image::ImageError),
    Encode(EncodeError),
    Ctex(ctex::CtexError),
    /// 源文件没有可用的文件名。
    BadPath(PathBuf),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{e}"),
            Error::Image(e) => write!(f, "解码 PNG 失败：{e}"),
            Error::Encode(e) => write!(f, "{e}"),
            Error::Ctex(e) => write!(f, "{e}"),
            Error::BadPath(p) => write!(f, "无法从路径取名：{}", p.display()),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}
impl From<image::ImageError> for Error {
    fn from(e: image::ImageError) -> Self {
        Error::Image(e)
    }
}
impl From<EncodeError> for Error {
    fn from(e: EncodeError) -> Self {
        Error::Encode(e)
    }
}
impl From<ctex::CtexError> for Error {
    fn from(e: ctex::CtexError) -> Self {
        Error::Ctex(e)
    }
}

#[derive(Debug, Clone)]
pub struct RunOutcome {
    pub source_md5: String,
    pub dest_md5: String,
    /// `.import` 内容是否发生变化（增量校验用）。
    pub import_changed: bool,
}

/// 编码单张 PNG 并写出 Godot 全部产物。
///
/// `res_path` 是源文件在 Godot 项目里的 `res://` 路径；`out_dir` 是项目的
/// `.godot/imported/` 绝对路径。
pub fn run_one(
    src_png: &Path,
    res_path: &str,
    out_dir: &Path,
    enc: &dyn Encoder,
) -> Result<RunOutcome, Error> {
    let source = std::fs::read(src_png)?;
    let img = image::load_from_memory(&source)?.to_rgba8();
    let (w, h) = (img.width(), img.height());

    let mut dests = Vec::with_capacity(2);
    for fmt in [Format::Bc7, Format::Astc4x4] {
        let blocks = enc.encode(img.as_raw(), w, h, fmt)?;
        dests.push(ctex::encode_ctex(fmt, w, h, &blocks)?);
    }

    let file_name = src_png
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| Error::BadPath(src_png.to_path_buf()))?;
    let base = format!("{file_name}-{}", md5::source_md5(res_path.as_bytes()));
    let bptc_res = format!("res://.godot/imported/{base}.bptc.ctex");
    let astc_res = format!("res://.godot/imported/{base}.astc.ctex");

    std::fs::create_dir_all(out_dir)?;
    write_atomic(&out_dir.join(format!("{base}.bptc.ctex")), &dests[0])?;
    write_atomic(&out_dir.join(format!("{base}.astc.ctex")), &dests[1])?;

    let source_md5 = md5::source_md5(&source);
    let dest_md5 = md5::dest_md5(&dests);
    let md5_text = md5::render_md5(&source_md5, &dest_md5);
    write_atomic(&out_dir.join(format!("{base}.md5")), md5_text.as_bytes())?;

    let import_path = import_path(src_png);
    let existing = std::fs::read_to_string(&import_path).ok();
    let uid = existing
        .as_deref()
        .and_then(import::existing_uid)
        .unwrap_or_else(|| import::new_uid(&format!("{res_path}:{source_md5}")));
    let params = existing
        .as_deref()
        .and_then(params_of)
        .unwrap_or(DEFAULT_PARAMS);
    let text = import::render_import(res_path, &uid, &bptc_res, &astc_res, params);
    let import_changed = existing.as_deref() != Some(text.as_str());
    if import_changed {
        write_atomic(&import_path, text.as_bytes())?;
    }

    Ok(RunOutcome {
        source_md5,
        dest_md5,
        import_changed,
    })
}

/// `.import` 与源 PNG 同目录，文件名为 `<文件名>.import`。
pub fn import_path(src_png: &Path) -> PathBuf {
    let mut os = src_png.as_os_str().to_owned();
    os.push(".import");
    PathBuf::from(os)
}

fn params_of(text: &str) -> Option<&str> {
    let i = text.find("[params]")?;
    Some(&text[i..])
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}
