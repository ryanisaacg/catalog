mod tree;

pub use tree::BTree;

#[cfg(test)]
mod tests {
    use super::tree::BTree;

    type IntTree = BTree<i32, i32>;

    #[test]
    fn empty_tree() {
        let tree: IntTree = BTree::new();
        let children: Vec<_> = tree.iter().collect();
        assert_eq!(&children[..], &[]);
    }
}
