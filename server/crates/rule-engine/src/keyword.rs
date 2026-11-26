//! 关键词位图集合。
//!
//! 战斗热路径原先每次都把关键词字符串收集到 `BTreeSet`；本模块用位图表示
//! 已知关键词，并单独保存需要累加层数的 `breed` / `assemble` / `armor`。
//! 未知关键词仍以原字符串保存在 `unknown` 中，保证与字符串集合口径一致。
//!
//! 字符串版 [`crate::combat::keywords`] 保持不变，视图与测试继续使用；
//! 只有战斗内部判定改用 [`KeywordSet`]。

/// 已知关键词基名；`decompose` 不在加载期校验的基名内，但数据中实际出现。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Keyword {
    Battlecry,
    Deathrattle,
    Reborn,
    Taunt,
    DivineShield,
    Windfury,
    Contagion,
    IgnoreTaunt,
    Sow,
    Evolve,
    Breed,
    Assemble,
    Poison,
    Armor,
    Stealth,
    Decompose,
}

/// 全部已知关键词，顺序即位图位序。
pub const ALL: [Keyword; 16] = [
    Keyword::Battlecry,
    Keyword::Deathrattle,
    Keyword::Reborn,
    Keyword::Taunt,
    Keyword::DivineShield,
    Keyword::Windfury,
    Keyword::Contagion,
    Keyword::IgnoreTaunt,
    Keyword::Sow,
    Keyword::Evolve,
    Keyword::Breed,
    Keyword::Assemble,
    Keyword::Poison,
    Keyword::Armor,
    Keyword::Stealth,
    Keyword::Decompose,
];

impl Keyword {
    /// 数据与协议中的基名。
    pub fn id(self) -> &'static str {
        match self {
            Keyword::Battlecry => "battlecry",
            Keyword::Deathrattle => "deathrattle",
            Keyword::Reborn => "reborn",
            Keyword::Taunt => "taunt",
            Keyword::DivineShield => "divine_shield",
            Keyword::Windfury => "windfury",
            Keyword::Contagion => "contagion",
            Keyword::IgnoreTaunt => "ignore_taunt",
            Keyword::Sow => "sow",
            Keyword::Evolve => "evolve",
            Keyword::Breed => "breed",
            Keyword::Assemble => "assemble",
            Keyword::Poison => "poison",
            Keyword::Armor => "armor",
            Keyword::Stealth => "stealth",
            Keyword::Decompose => "decompose",
        }
    }

    /// 由基名解析；未知基名返回 `None`。
    pub fn from_id(id: &str) -> Option<Keyword> {
        ALL.into_iter().find(|keyword| keyword.id() == id)
    }

    /// 位图中使用的单个二进制位。
    fn bit(self) -> u16 {
        1u16 << (self as u8)
    }

    /// 位序，与 [`ALL`] 下标一致；观测编码按此顺序排位。
    pub fn index(self) -> usize {
        self as usize
    }
}

/// 解析关键词字符串；`breed N` / `assemble N` / `armor N` 携带层数，无数字按 1。
///
/// 不属于关键词、或非层数关键词带多余层数的字符串返回 `None`，交由调用方
/// 原样保留。层数解析失败按 0 处理，与旧 `armor_layer` 口径一致。
pub fn parse_keyword(raw: &str) -> Option<(Keyword, u8)> {
    let mut parts = raw.split_whitespace();
    let keyword = Keyword::from_id(parts.next()?)?;
    let layer = match keyword {
        Keyword::Breed | Keyword::Assemble | Keyword::Armor => match parts.next() {
            None => 1,
            Some(text) => {
                // 多余的分段不是合法关键词形态，按未知处理。
                if parts.next().is_some() {
                    return None;
                }
                match text.parse::<u32>() {
                    // 旧 `armor_layer` 按 `i32` 解析层数；这里把单段层数钳到
                    // `u8::MAX`。当前数据层数 ≤3、动态授予只有 `armor 2`，
                    // 超过 255 的输入不可达。
                    Ok(value) => value.min(u32::from(u8::MAX)) as u8,
                    Err(_) => 0,
                }
            }
        },
        _ => {
            if parts.next().is_some() {
                return None;
            }
            1
        }
    };
    Some((keyword, layer))
}

/// 关键词位图集合；与字符串集合同口径，但查询不分配。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeywordSet {
    bits: u16,
    breed: u32,
    assemble: u32,
    armor: u32,
    unknown: Vec<String>,
}

impl KeywordSet {
    /// 由关键词字符串列表构建集合；测试与边界适配使用。
    pub fn from_names<'a>(names: impl IntoIterator<Item = &'a str>) -> Self {
        let mut set = Self::default();
        for name in names {
            set.insert(name);
        }
        set
    }

    /// 并入一条关键词字符串：已知关键词置位，层数类同时累加。
    pub fn insert(&mut self, raw: &str) {
        match parse_keyword(raw) {
            Some((keyword, layer)) => {
                self.bits |= keyword.bit();
                match keyword {
                    Keyword::Breed => {
                        self.breed = self.breed.saturating_add(u32::from(layer));
                    }
                    Keyword::Assemble => {
                        self.assemble = self.assemble.saturating_add(u32::from(layer));
                    }
                    Keyword::Armor => {
                        self.armor = self.armor.saturating_add(u32::from(layer));
                    }
                    _ => {}
                }
            }
            None => {
                if !self.unknown.iter().any(|known| known == raw) {
                    self.unknown.push(raw.to_string());
                }
            }
        }
    }

    /// 是否带某个已知关键词。
    pub fn contains(&self, keyword: Keyword) -> bool {
        self.bits & keyword.bit() != 0
    }

    /// 按原始字符串查询；已知关键词按基名匹配，未知关键词按原文匹配。
    pub fn contains_name(&self, raw: &str) -> bool {
        match parse_keyword(raw) {
            Some((keyword, _)) => self.contains(keyword),
            None => self.unknown.iter().any(|known| known == raw),
        }
    }

    /// 减伤层数之和；溢出时饱和到 `i32::MAX`。
    pub fn armor(&self) -> i32 {
        i32::try_from(self.armor).unwrap_or(i32::MAX)
    }

    /// 累计的繁衍层数。
    pub fn breed(&self) -> u32 {
        self.breed
    }

    /// 累计的装配层数。
    pub fn assemble(&self) -> u32 {
        self.assemble
    }

    /// 是否不含任何已知或未知关键词。
    pub fn is_empty(&self) -> bool {
        self.bits == 0 && self.unknown.is_empty()
    }

    /// 按字符串集合口径展开全部关键词，供视图、哈希与跨模块边界使用。
    ///
    /// 已知关键词写成数据与协议中的基名；`breed` / `assemble` / `armor`
    /// 写成 `基名 层数`（位图已把同一关键词的层数累加）。结果按字典序排序，
    /// 与旧 `BTreeSet<String>` 的遍历顺序一致。
    pub fn names(&self) -> Vec<String> {
        let mut names = Vec::with_capacity(self.unknown.len() + 4);
        for keyword in ALL {
            if !self.contains(keyword) {
                continue;
            }
            names.push(match keyword {
                Keyword::Breed => format!("{} {}", keyword.id(), self.breed),
                Keyword::Assemble => format!("{} {}", keyword.id(), self.assemble),
                Keyword::Armor => format!("{} {}", keyword.id(), self.armor),
                _ => keyword.id().to_string(),
            });
        }
        names.extend(self.unknown.iter().cloned());
        names.sort_unstable();
        names
    }
}
