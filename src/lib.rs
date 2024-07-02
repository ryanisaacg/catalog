mod tree;

pub use tree::BTree;

#[cfg(test)]
mod tests {
    use super::tree::BTree;

    type IntTree = BTree<i32, i32>;

    #[test]
    fn empty_tree() {
        let tree = IntTree::new();
        let children: Vec<_> = tree.iter().collect();
        assert_eq!(&children[..], &[]);
    }

    #[test]
    #[should_panic]
    fn insert_value() {
        let mut tree = IntTree::new();
        tree.insert(1, 2);
        let children: Vec<_> = tree.iter().map(|(k, v)| (*k, *v)).collect();
        assert_eq!(&children[..], &[(1, 2)]);
    }

    #[test]
    #[should_panic]
    fn get_value() {
        let mut tree = IntTree::new();
        tree.insert(1, 2);
        let val = tree.get(&1);
        assert_eq!(val, Some(&2));
    }

    #[test]
    #[should_panic]
    fn remove_value() {
        let mut tree = IntTree::new();
        tree.insert(1, 2);
        let val = tree.remove(&1);
        assert_eq!(val, Some(2));
        let children: Vec<_> = tree.iter().collect();
        assert_eq!(&children[..], &[]);
    }
}
