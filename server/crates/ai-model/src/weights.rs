//! 权重格式：JSON 清单 + 扁平 `f32` 数组。
//!
//! 清单记录版本、三张嵌入表的条目数与每个张量的元素个数；数据按清单顺序
//! 以小端 `f32` 连续存放。嵌入表条目数变化（目录变化）时拒绝加载。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use ai_core::CardIndex;
use serde::{Deserialize, Serialize};

use crate::ModelError;

/// 权重格式版本；张量语义或布局变更时递增。
/// 权重格式版本；观测布局或张量语义变化时递增。
///
/// 2：观测维度 250 → 431（补齐商店可购买性、关键词、技能次数、待选与对手座位）。
pub const WEIGHTS_VERSION: u32 = 2;

/// 一份模型权重。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Weights {
    pub tensors: BTreeMap<String, Vec<f32>>,
    /// 卡牌嵌入表条目数。
    pub card_count: usize,
    /// 英雄嵌入表条目数。
    pub hero_count: usize,
    /// 技能嵌入表条目数。
    pub skill_count: usize,
}

#[derive(Serialize, Deserialize)]
struct Manifest {
    version: u32,
    card_count: usize,
    hero_count: usize,
    skill_count: usize,
    /// 张量名 → 元素个数；顺序即 `.bin` 中的排列顺序。
    tensors: BTreeMap<String, usize>,
}

impl Weights {
    /// 写入权重；`path` 为 JSON 清单，同名 `.bin` 存放数据。
    ///
    /// 写入前先校验所有张量都是有限值：训练发散得到的 NaN 权重一旦落盘，加载端
    /// 的 ReLU 会把 NaN 吃掉，评测会给出一个看起来正常的分数，掩盖真正的故障。
    pub fn save(&self, path: &Path) -> Result<(), ModelError> {
        for (name, data) in &self.tensors {
            if let Some(index) = data.iter().position(|value| !value.is_finite()) {
                return Err(ModelError::NonFinite(format!("{name}（第 {index} 项）")));
            }
        }
        let manifest = Manifest {
            version: WEIGHTS_VERSION,
            card_count: self.card_count,
            hero_count: self.hero_count,
            skill_count: self.skill_count,
            tensors: self
                .tensors
                .iter()
                .map(|(name, data)| (name.clone(), data.len()))
                .collect(),
        };
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_vec_pretty(&manifest)?)?;
        let mut bytes = Vec::new();
        for data in self.tensors.values() {
            for value in data {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        fs::write(data_path(path), bytes)?;
        Ok(())
    }

    /// 读取权重并校验清单与数据长度一致。
    pub fn load(path: &Path) -> Result<Self, ModelError> {
        let manifest: Manifest = serde_json::from_slice(&fs::read(path)?)?;
        if manifest.version != WEIGHTS_VERSION {
            return Err(ModelError::Version(manifest.version));
        }
        let bytes = fs::read(data_path(path))?;
        let total: usize = manifest.tensors.values().sum();
        if bytes.len() != total * 4 {
            return Err(ModelError::Shape(format!(
                "权重数据长度 {} 与清单 {} 不符",
                bytes.len(),
                total * 4
            )));
        }
        let mut tensors = BTreeMap::new();
        let mut offset = 0;
        for (name, len) in manifest.tensors {
            let mut data = Vec::with_capacity(len);
            for chunk in bytes[offset..offset + len * 4].chunks_exact(4) {
                data.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
            }
            // 长度对得上不代表内容有效：发散训练的产物会在这里被挡住。
            if let Some(index) = data.iter().position(|value| !value.is_finite()) {
                return Err(ModelError::NonFinite(format!("{name}（第 {index} 项）")));
            }
            tensors.insert(name, data);
            offset += len * 4;
        }
        Ok(Self {
            tensors,
            card_count: manifest.card_count,
            hero_count: manifest.hero_count,
            skill_count: manifest.skill_count,
        })
    }

    /// 嵌入表条目数必须与目录索引一致，否则拒绝使用。
    pub fn validate(&self, index: &CardIndex) -> Result<(), ModelError> {
        let expected = (index.card_count(), index.hero_count(), index.skill_count());
        let actual = (self.card_count, self.hero_count, self.skill_count);
        if expected != actual {
            return Err(ModelError::Shape(format!(
                "嵌入表 {actual:?} 与目录 {expected:?} 不符"
            )));
        }
        Ok(())
    }

    /// 设置或替换一个张量。
    pub fn set(&mut self, name: &str, data: Vec<f32>) {
        self.tensors.insert(name.to_string(), data);
    }

    /// 读取一个张量。
    pub fn get(&self, name: &str) -> Option<&[f32]> {
        self.tensors.get(name).map(Vec::as_slice)
    }
}

fn data_path(path: &Path) -> PathBuf {
    let mut path = path.to_path_buf();
    path.set_extension("bin");
    path
}
