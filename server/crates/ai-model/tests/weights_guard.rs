//! 权重文件必须拒绝非有限值：训练发散时导出的 NaN 权重如果被静默接受，
//! 评测会给出一个看起来很正常的分数（ReLU 会把 NaN 吃掉），掩盖真正的故障。

use std::collections::BTreeMap;
use std::path::PathBuf;

use ai_model::Weights;

fn weights_with(value: f32) -> Weights {
    let mut tensors = BTreeMap::new();
    tensors.insert("trunk.0.weight".to_string(), vec![value; 4]);
    Weights {
        tensors,
        card_count: 1,
        hero_count: 1,
        skill_count: 1,
    }
}

fn scratch(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("ai-model-nonfinite-{}-{name}", std::process::id()))
}

#[test]
fn save_rejects_non_finite_tensor() {
    let path = scratch("save").join("weights.json");
    let error = weights_with(f32::NAN)
        .save(&path)
        .expect_err("含 NaN 的权重不应写入磁盘");
    assert!(
        error.to_string().contains("trunk.0.weight"),
        "错误信息应指出张量名：{error}"
    );
    assert!(!path.exists(), "校验失败时不应留下清单文件");

    let mut data = path.clone();
    data.set_extension("bin");
    assert!(!data.exists(), "校验失败时不应留下数据文件");
}

#[test]
fn load_rejects_non_finite_data() {
    let path = scratch("load").join("weights.json");
    weights_with(0.5).save(&path).expect("有效权重应能写入");

    let mut data = path.clone();
    data.set_extension("bin");
    let mut bytes = std::fs::read(&data).expect("读取数据文件");
    bytes[..4].copy_from_slice(&f32::INFINITY.to_le_bytes());
    std::fs::write(&data, bytes).expect("写入数据文件");

    let error = Weights::load(&path).expect_err("含非有限值的权重不应被加载");
    assert!(
        error.to_string().contains("trunk.0.weight"),
        "错误信息应指出张量名：{error}"
    );
}
