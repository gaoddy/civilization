mod common;

use common::test_catalog;

#[test]
fn is_within_follows_path() {
    let catalog = test_catalog();
    let taxonomy = &catalog.taxonomy;

    for leaf in ["hominidae", "aves", "plantae", "bacteria"] {
        let path = taxonomy.path(leaf);
        assert!(!path.is_empty(), "{leaf} 应在分类树中");
        for ancestor in path {
            assert!(
                taxonomy.is_within(leaf, ancestor),
                "{leaf} 应位于 {ancestor} 之下"
            );
        }
    }

    assert!(!taxonomy.is_within("hominidae", "plantae"));
    assert!(!taxonomy.is_within("missing", "animalia"));
    assert!(!taxonomy.is_within("animalia", "missing"));
    // 两侧同时缺失也返回 false：文档契约要求任一节点不存在即不匹配。
    assert!(!taxonomy.is_within("missing", "missing"));
}

#[test]
fn contains_and_leaf_semantics_unchanged() {
    let catalog = test_catalog();
    let taxonomy = &catalog.taxonomy;

    assert!(taxonomy.contains("hominidae"));
    assert!(!taxonomy.contains("missing"));
    assert!(!taxonomy.is_leaf("animalia"));
    assert!(taxonomy.is_leaf("hominidae"));
}
