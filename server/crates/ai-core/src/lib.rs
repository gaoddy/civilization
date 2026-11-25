//! 观测与动作编码：训练与推理共用的唯一一份实现。
//!
//! 本 crate 只做纯数据转换，不读时钟、环境变量或文件，也不依赖任何 ML 运行时。

/// 观测/动作编码的 schema 版本；字段顺序或语义变更时递增。
pub const SCHEMA_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    #[test]
    fn schema_version_is_one() {
        assert_eq!(super::SCHEMA_VERSION, 1);
    }
}
