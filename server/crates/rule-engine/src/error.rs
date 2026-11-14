use std::error::Error;
use std::fmt;

/// 卡牌目录加载失败的原因。
#[derive(Debug)]
pub enum CatalogError {
    /// 某个 JSON 文件解析失败，附带文件名以便定位。
    File(&'static str, serde_json::Error),
    /// 数据自身不合法（重复 id、悬空引用等），消息中包含可定位的 id。
    Rule(String),
}

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File(name, err) => write!(f, "{name}: {err}"),
            Self::Rule(message) => f.write_str(message),
        }
    }
}

impl Error for CatalogError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::File(_, err) => Some(err),
            Self::Rule(_) => None,
        }
    }
}

/// 引擎动作被拒绝的原因，消息面向日志与调试。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineError(pub String);

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for EngineError {}
