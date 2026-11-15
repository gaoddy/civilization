//! 协议 Rust 契约生成器。
//!
//! 读 `protocol/schema/v1/*.schema.json`，把各文件的 `$defs` 合并成单一 bundle，
//! 交给 typify 生成 Rust 类型，再用 prettyplease 格式化写出。
//!
//! 用法：
//! - 默认：重新生成 `protocol/generated/protocol.rs`。
//! - `--check`：只比较内存生成结果与磁盘内容，不一致时打印首个差异行并以 1 退出。

use std::collections::BTreeMap;
use std::process::ExitCode;

use serde_json::{Map, Value, json};

/// 参与 bundle 的 schema 文件，顺序固定以保持可复现。
const SCHEMA_FILES: [&str; 5] = [
    "errors.schema.json",
    "intents.schema.json",
    "envelope.schema.json",
    "messages.schema.json",
    "view.schema.json",
];

const GENERATED_HEADER: &str = "// 本文件由 `cargo run -p protocol-gen` 生成，不要手改。\n";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let check = std::env::args().skip(1).any(|arg| arg == "--check");
    let repo_root = repo_root();
    let schema_dir = repo_root.join("protocol/schema/v1");
    let output_path = repo_root.join("protocol/generated/protocol.rs");

    let bundle = build_bundle(&schema_dir)?;
    let generated = generate(&bundle)?;

    if check {
        let existing = std::fs::read_to_string(&output_path)
            .map_err(|err| format!("读取 {} 失败：{err}", output_path.display()))?;
        if existing != generated {
            let line = first_different_line(&existing, &generated);
            return Err(format!(
                "{} 与生成结果不一致（首个差异在第 {line} 行），请运行 `make protocol-rs`",
                output_path.display()
            ));
        }
        Ok(())
    } else {
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("创建 {} 失败：{err}", parent.display()))?;
        }
        std::fs::write(&output_path, generated)
            .map_err(|err| format!("写入 {} 失败：{err}", output_path.display()))?;
        Ok(())
    }
}

/// 仓库根目录：`server/crates/protocol-gen` 上溯三层。
fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("解析仓库根目录失败")
}

/// 合并各 schema 的 `$defs`，并把跨文件 `$ref` 改写为同文件引用。
fn build_bundle(schema_dir: &std::path::Path) -> Result<Value, String> {
    let mut defs: BTreeMap<String, Value> = BTreeMap::new();

    for file in SCHEMA_FILES {
        let path = schema_dir.join(file);
        let text = std::fs::read_to_string(&path)
            .map_err(|err| format!("读取 {} 失败：{err}", path.display()))?;
        let document: Value = serde_json::from_str(&text)
            .map_err(|err| format!("解析 {} 失败：{err}", path.display()))?;
        let file_defs = document
            .get("$defs")
            .and_then(Value::as_object)
            .ok_or_else(|| format!("{} 缺少 $defs 对象", path.display()))?;

        for (name, schema) in file_defs {
            let mut schema = schema.clone();
            rewrite_refs(&mut schema);
            if defs.insert(name.clone(), schema).is_some() {
                return Err(format!("$defs 名重复：{name}（出现在 {file}）"));
            }
        }
    }

    let mut merged = Map::new();
    for (name, schema) in defs {
        merged.insert(name, schema);
    }

    Ok(json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$defs": Value::Object(merged),
    }))
}

/// 把 `xxx.schema.json#/$defs/Name` 改写为 `#/$defs/Name`。
fn rewrite_refs(value: &mut Value) {
    match value {
        Value::String(text) => {
            if let Some(fragment) = text.split_once("#/").map(|(_, rest)| rest)
                && text.contains(".schema.json#/")
            {
                *text = format!("#/{fragment}");
            }
        }
        Value::Array(items) => items.iter_mut().for_each(rewrite_refs),
        Value::Object(map) => map.values_mut().for_each(rewrite_refs),
        _ => {}
    }
}

/// 用 typify 生成代码，并用 prettyplease 格式化。
fn generate(bundle: &Value) -> Result<String, String> {
    let root_schema: schemars::schema::RootSchema = serde_json::from_value(bundle.clone())
        .map_err(|err| format!("bundle 转换为 RootSchema 失败：{err}"))?;

    let settings = typify::TypeSpaceSettings::default();
    let mut type_space = typify::TypeSpace::new(&settings);
    type_space
        .add_root_schema(root_schema)
        .map_err(|err| format!("typify 生成失败：{err}"))?;

    let stream = type_space.to_stream();
    let file: syn::File =
        syn::parse2(stream).map_err(|err| format!("typify 输出不是合法 Rust 词法：{err}"))?;
    let code = prettyplease::unparse(&file);

    let mut out = String::new();
    out.push_str(GENERATED_HEADER);
    out.push_str(&code);
    Ok(out)
}

/// 返回两个字符串首个不同行的行号（1 起）。
fn first_different_line(left: &str, right: &str) -> usize {
    let mut index = 1;
    for (a, b) in left.lines().zip(right.lines()) {
        if a != b {
            return index;
        }
        index += 1;
    }
    index
}
