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
use crate::uid::UidBook;

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
    /// 从输入目录向上找不到 `project.godot`。
    NoProject(PathBuf),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{e}"),
            Error::Image(e) => write!(f, "解码 PNG 失败：{e}"),
            Error::Encode(e) => write!(f, "{e}"),
            Error::Ctex(e) => write!(f, "{e}"),
            Error::BadPath(p) => write!(f, "无法从路径取名：{}", p.display()),
            Error::NoProject(p) => write!(
                f,
                "在 {} 的上级目录里找不到 project.godot，无法确定 res:// 路径",
                p.display()
            ),
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
    let decoded = decode_png_bytes(&source)?;
    let bc7 = enc.encode(&decoded.rgba, decoded.width, decoded.height, Format::Bc7)?;
    let astc = enc.encode(&decoded.rgba, decoded.width, decoded.height, Format::Astc4x4)?;
    let mut uids = UidBook::new(root_of(out_dir));
    write_outputs(
        src_png,
        &source,
        res_path,
        out_dir,
        decoded.width,
        decoded.height,
        [&bc7, &astc],
        &mut uids,
    )
}

/// `out_dir` 是 `<root>/.godot/imported`，向上两级即项目根；取不到时退回
/// `out_dir` 本身（扫描不到 uid，等价于不做去重）。
fn root_of(out_dir: &Path) -> PathBuf {
    out_dir
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| out_dir.to_path_buf())
}

/// 解码后的像素。
pub struct Decoded {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub fn decode_png(src_png: &Path) -> Result<Decoded, Error> {
    decode_png_bytes(&std::fs::read(src_png)?)
}

fn decode_png_bytes(source: &[u8]) -> Result<Decoded, Error> {
    let img = image::load_from_memory(source)?.to_rgba8();
    let (width, height) = (img.width(), img.height());
    Ok(Decoded {
        rgba: img.into_raw(),
        width,
        height,
    })
}

/// 写出一张图的全部产物；`blocks` 顺序固定为 `[BC7, ASTC]`。
#[allow(clippy::too_many_arguments)]
pub fn write_outputs(
    src_png: &Path,
    source: &[u8],
    res_path: &str,
    out_dir: &Path,
    w: u32,
    h: u32,
    blocks: [&[u8]; 2],
    uids: &mut UidBook,
) -> Result<RunOutcome, Error> {
    let dests = [
        ctex::encode_ctex(Format::Bc7, w, h, blocks[0])?,
        ctex::encode_ctex(Format::Astc4x4, w, h, blocks[1])?,
    ];

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

    let source_md5 = md5::source_md5(source);
    let dest_md5 = md5::dest_md5(&dests);

    let import_path = import_path(src_png);
    let existing = std::fs::read_to_string(&import_path).ok();
    let uid = existing
        .as_deref()
        .and_then(import::existing_uid)
        .unwrap_or_else(|| uids.allocate(&format!("{res_path}:{source_md5}")));
    let params = existing
        .as_deref()
        .and_then(params_of)
        .unwrap_or(DEFAULT_PARAMS);
    let text = import::render_import(res_path, &uid, &bptc_res, &astc_res, params);
    let import_changed = existing.as_deref() != Some(text.as_str());
    if import_changed {
        write_atomic(&import_path, text.as_bytes())?;
    }

    // `.md5` 是「产物完整」的提交标记，必须最后落盘：中途失败时它缺席，
    // 下次运行就会重编，而不是永久跳过一份残缺产物。
    let md5_text = md5::render_md5(&source_md5, &dest_md5);
    write_atomic(&out_dir.join(format!("{base}.md5")), md5_text.as_bytes())?;

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

// ---------------------------------------------------------------------------
// 批量执行

use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct RunReport {
    pub encoded: usize,
    pub skipped: usize,
    pub wall: Duration,
}

/// 从 `input` 向上找含 `project.godot` 的目录。
pub fn project_root(input: &Path) -> Option<PathBuf> {
    let mut dir = if input.is_dir() {
        input.to_path_buf()
    } else {
        input.parent()?.to_path_buf()
    };
    loop {
        if dir.join("project.godot").exists() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// 递归收集 PNG，按路径排序（保证每次运行顺序稳定）。
pub fn find_pngs(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "png") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

fn res_path_of(root: &Path, file: &Path) -> String {
    let rel = file.strip_prefix(root).unwrap_or(file);
    format!("res://{}", rel.to_string_lossy().replace('\\', "/"))
}

fn out_base(root: &Path, file: &Path) -> (PathBuf, String, String) {
    let res = res_path_of(root, file);
    let name = file.file_name().and_then(|s| s.to_str()).unwrap_or_default();
    let base = format!("{name}-{}", md5::source_md5(res.as_bytes()));
    (root.join(".godot/imported"), res, base)
}

/// 是否需要重编：源 md5 与现有 `.md5` 一致，且两个产物与 `.import` 都在。
pub fn needs_encode(root: &Path, file: &Path, source: &[u8]) -> Result<bool, Error> {
    let (out_dir, _, base) = out_base(root, file);
    let existing = std::fs::read_to_string(out_dir.join(format!("{base}.md5"))).ok();
    if matches!(
        crate::plan::decide(source, existing.as_deref()),
        crate::plan::Work::Skip
    ) && out_dir.join(format!("{base}.bptc.ctex")).exists()
        && out_dir.join(format!("{base}.astc.ctex")).exists()
        && import_path(file).exists()
    {
        return Ok(false);
    }
    Ok(true)
}

/// 目录批量编码。按固定张数分批，避免全量 RGBA 同时驻留。
pub fn run_all(enc: &dyn Encoder, input: &Path, jobs: usize) -> Result<RunReport, Error> {
    use rayon::prelude::*;

    let started = Instant::now();
    let root = project_root(input).ok_or_else(|| Error::NoProject(input.to_path_buf()))?;
    let out_dir = root.join(".godot/imported");
    std::fs::create_dir_all(&out_dir)?;

    let mut pending: Vec<(PathBuf, Vec<u8>)> = Vec::new();
    let mut skipped = 0usize;
    for file in find_pngs(input) {
        let source = std::fs::read(&file)?;
        if needs_encode(&root, &file, &source)? {
            pending.push((file, source));
        } else {
            skipped += 1;
        }
    }

    let chunk = (jobs * 8).max(8);
    let mut encoded = 0usize;
    let mut uids = UidBook::new(&root);
    for group in pending.chunks(chunk) {
        let decoded: Vec<Result<Decoded, Error>> = group
            .par_iter()
            .map(|(_, source)| decode_png_bytes(source))
            .collect();
        let decoded = decoded.into_iter().collect::<Result<Vec<_>, _>>()?;
        let refs: Vec<(&[u8], u32, u32)> = decoded
            .iter()
            .map(|d| (d.rgba.as_slice(), d.width, d.height))
            .collect();
        let bc7 = enc.encode_batch(&refs, Format::Bc7);
        let astc = enc.encode_batch(&refs, Format::Astc4x4);
        for (i, (file, source)) in group.iter().enumerate() {
            let bc7 = bc7[i].as_ref().map_err(|e| Error::Encode(clone_err(e)))?;
            let astc = astc[i].as_ref().map_err(|e| Error::Encode(clone_err(e)))?;
            let (out_dir, res, _) = out_base(&root, file);
            write_outputs(
                file,
                source,
                &res,
                &out_dir,
                decoded[i].width,
                decoded[i].height,
                [bc7, astc],
                &mut uids,
            )?;
            encoded += 1;
        }
    }

    Ok(RunReport {
        encoded,
        skipped,
        wall: started.elapsed(),
    })
}

fn clone_err(e: &crate::encode::EncodeError) -> crate::encode::EncodeError {
    match e {
        crate::encode::EncodeError::BadSize { w, h } => crate::encode::EncodeError::BadSize {
            w: *w,
            h: *h,
        },
        crate::encode::EncodeError::Astc(m) => crate::encode::EncodeError::Astc(m.clone()),
        crate::encode::EncodeError::Bc7(m) => crate::encode::EncodeError::Bc7(m.clone()),
    }
}
