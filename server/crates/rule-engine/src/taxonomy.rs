//! 物种分类树：节点、祖先匹配与路径。
//!
//! 数据来自 `server/data/v0.1/taxonomy.json`，节点按 id 去重。
//! 祖先匹配沿父链上溯，深度不超过 5；240 张卡规模下无需缓存。

use std::collections::BTreeMap;

use serde::Deserialize;

/// 分类层级；仅用于表达分类树的结构，不参与匹配。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rank {
    Kingdom,
    Phylum,
    Class,
    Order,
    Family,
    Technique,
    /// 科学、人文与社会根下的学科叶子。
    Discipline,
}

/// 单个分类节点。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxonNode {
    pub parent: Option<String>,
    pub rank: Rank,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TaxonEntry {
    id: String,
    parent: Option<String>,
    rank: Rank,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TaxonomyFile {
    nodes: Vec<TaxonEntry>,
    /// 玩法 id 清单；引擎以 [`crate::data::Playstyle`] 为准，这里只做结构接受。
    #[serde(default)]
    #[allow(dead_code)]
    playstyles: Vec<String>,
}

/// 只读分类树，按节点 id 索引。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Taxonomy {
    nodes: BTreeMap<String, TaxonNode>,
}

impl Taxonomy {
    /// 解析 `taxonomy.json`；重复 id 视为非法。
    pub fn from_json(raw: &str) -> Result<Self, serde_json::Error> {
        let file: TaxonomyFile = serde_json::from_str(raw)?;
        let mut nodes = BTreeMap::new();
        for entry in file.nodes {
            if nodes.contains_key(&entry.id) {
                return Err(serde::de::Error::custom(format!(
                    "taxonomy.json: 重复的节点 id: {}",
                    entry.id
                )));
            }
            nodes.insert(
                entry.id,
                TaxonNode {
                    parent: entry.parent,
                    rank: entry.rank,
                },
            );
        }
        Ok(Self { nodes })
    }

    /// 节点是否存在。
    pub fn contains(&self, id: &str) -> bool {
        self.nodes.contains_key(id)
    }

    /// 是否为叶子节点（没有任何子节点）。
    ///
    /// 未知节点返回 `false`。只在加载期调用，线性扫描可接受。
    pub fn is_leaf(&self, id: &str) -> bool {
        self.nodes.contains_key(id)
            && !self
                .nodes
                .values()
                .any(|node| node.parent.as_deref() == Some(id))
    }

    /// `leaf` 是否为 `ancestor` 本身或其后代。
    ///
    /// 沿父链上溯，最多 5 层；任一节点不存在时返回 `false`。
    pub fn is_within(&self, leaf: &str, ancestor: &str) -> bool {
        let mut current = leaf;
        for _ in 0..=5 {
            if current == ancestor {
                return true;
            }
            match self
                .nodes
                .get(current)
                .and_then(|node| node.parent.as_deref())
            {
                Some(parent) => current = parent,
                None => return false,
            }
        }
        false
    }

    /// 从叶子到根的节点路径（含自身）；未知节点返回空列表。
    pub fn path<'a>(&'a self, leaf: &'a str) -> Vec<&'a str> {
        let mut path = Vec::new();
        let mut current = leaf;
        while let Some(node) = self.nodes.get(current) {
            path.push(current);
            match node.parent.as_deref() {
                Some(parent) => current = parent,
                None => break,
            }
        }
        path
    }
}
