//! 网络结构与权重格式：纯 Rust 手写前向，训练与推理共用同一套权重。
//!
//! 不依赖 `candle` 或其他 ML 运行时；推理端只需字节级加载权重与一次前向。

/// 权重格式版本；张量名或布局变更时递增。
pub const WEIGHTS_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    #[test]
    fn weights_version_is_one() {
        assert_eq!(super::WEIGHTS_VERSION, 1);
    }
}
