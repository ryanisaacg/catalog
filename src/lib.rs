mod memtree;
mod tree;

pub use memtree::BTree;

#[cfg(test)]
mod tests {
    use super::tree::BTree;

    type IntTree = BTree<i32, i32>;
    type IntMemTree<'a> = super::memtree::BTree<'a, i32, i32>;

    #[test]
    fn empty_tree() {
        let tree = IntTree::new();
        let children: Vec<_> = tree.iter().collect();
        assert_eq!(&children[..], &[]);
    }

    #[test]
    fn insert_value() {
        let mut tree = IntTree::new();
        tree.insert(1, 2);
        let children: Vec<_> = tree.iter().map(|(k, v)| (*k, *v)).collect();
        assert_eq!(&children[..], &[(1, 2)]);
    }

    #[test]
    fn insert_mem_value() {
        let mut buffer = vec![0u8; 1024];
        let mut tree = IntMemTree::new(&mut buffer[..]);

        tree.insert(1, 2);
        assert_eq!(tree.get(&1), Some(&2));
    }

    #[test]
    fn get_value() {
        let mut tree = IntTree::new();
        tree.insert(1, 2);
        let val = tree.get(&1);
        assert_eq!(val, Some(&2));
    }

    #[test]
    fn insert_many() {
        let mut tree = IntTree::new();
        for i in (0..32).rev() {
            tree.insert(i, i.pow(2));
        }
        for i in (0..32i32).rev() {
            assert_eq!(Some(&(i.pow(2))), tree.get(&i));
        }
    }

    #[test]
    fn insert_mem_many() {
        let mut buffer = vec![0u8; 1024];
        let mut tree = IntMemTree::new(&mut buffer[..]);
        for i in (0..32).rev() {
            tree.insert(i, i.pow(2));
        }
        for i in (0..32i32).rev() {
            assert_eq!(Some(&(i.pow(2))), tree.get(&i));
        }
    }

    #[test]
    fn get_mut() {
        let mut tree = IntTree::new();
        for i in 0..10 {
            tree.insert(i, i);
        }
        for i in 0..10 {
            let val = tree.get_mut(&i).unwrap();
            if *val > 5 {
                *val = 10;
            } else {
                *val = 0;
            }
        }
        for i in 0..10 {
            assert_eq!(*tree.get(&i).unwrap(), if i > 5 { 10 } else { 0 });
        }
    }

    #[test]
    fn remove_value() {
        let mut tree = IntTree::new();
        tree.insert(1, 2);
        let val = tree.remove(&1);
        assert_eq!(val, Some(2));
        let children: Vec<_> = tree.iter().collect();
        assert_eq!(&children[..], &[]);
    }

    #[test]
    fn remove_many() {
        let mut tree = IntTree::new();
        for i in 0..25 {
            tree.insert(i, i);
        }
        for i in 0..25 {
            if i < 15 {
                assert_eq!(tree.remove(&i), Some(i));
            }
        }
        for i in 0..10 {
            assert_eq!(tree.get(&i), if i < 15 { None } else { Some(&i) });
        }
    }
}
