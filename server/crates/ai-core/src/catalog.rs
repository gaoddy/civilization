//! 卡牌/英雄/技能的整数索引：跨进程稳定，训练与服务端共用。

use std::collections::{BTreeMap, BTreeSet};

use rule_engine::data::Catalog;

/// 目录指纹：卡牌、英雄与技能 id 的 FNV-1a 哈希，写成 16 位十六进制。
///
/// 训练产物清单里记录它，加载权重时与当前卡表比对，避免旧权重大摇大摆地
/// 套到新卡表上。索引按 id 字典序生成，因此同一份目录在任何进程里哈希一致。
pub fn catalog_hash(catalog: &Catalog) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let mut feed = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    };
    for id in catalog.cards.keys() {
        feed(id.as_bytes());
    }
    for id in catalog.heroes.keys() {
        feed(id.as_bytes());
    }
    for skill in catalog.skills() {
        feed(skill.id.as_bytes());
    }
    format!("{hash:016x}")
}

/// 目录索引；找不到的 id 一律返回 `-1`。
///
/// 编号按目录的 `BTreeMap` 顺序（即 id 字典序）生成，技能按被引用的 id 排序，
/// 因此同一份目录在任何进程里都得到同一套编号。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CardIndex {
    cards: BTreeMap<String, i32>,
    heroes: BTreeMap<String, i32>,
    skills: BTreeMap<String, i32>,
}

impl CardIndex {
    /// 按目录构建索引。
    pub fn build(catalog: &Catalog) -> Self {
        let cards = catalog
            .cards
            .keys()
            .enumerate()
            .map(|(index, id)| (id.clone(), index as i32))
            .collect();
        let heroes = catalog
            .heroes
            .keys()
            .enumerate()
            .map(|(index, id)| (id.clone(), index as i32))
            .collect();

        // 技能表本身不公开遍历，按卡牌与英雄实际引用的技能 id 排序建索引；
        // 未被任何卡引用的技能对策略没有意义。
        let mut referenced: BTreeSet<String> = BTreeSet::new();
        for card in catalog.cards.values() {
            for skill in &card.skills {
                referenced.insert(catalog.skill(*skill).id.clone());
            }
        }
        for hero in catalog.heroes.values() {
            for skill in &hero.skills {
                referenced.insert(catalog.skill(*skill).id.clone());
            }
        }
        let skills = referenced
            .into_iter()
            .enumerate()
            .map(|(index, id)| (id, index as i32))
            .collect();

        Self {
            cards,
            heroes,
            skills,
        }
    }

    pub fn card(&self, id: &str) -> i32 {
        self.cards.get(id).copied().unwrap_or(-1)
    }

    pub fn hero(&self, id: &str) -> i32 {
        self.heroes.get(id).copied().unwrap_or(-1)
    }

    pub fn skill(&self, id: &str) -> i32 {
        self.skills.get(id).copied().unwrap_or(-1)
    }

    pub fn card_count(&self) -> usize {
        self.cards.len()
    }

    pub fn hero_count(&self) -> usize {
        self.heroes.len()
    }

    pub fn skill_count(&self) -> usize {
        self.skills.len()
    }
}
