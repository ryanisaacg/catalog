pub struct BTree<K: Ord + Eq, V> {
    root: BNode<K, V>,
}

enum BNode<K: Ord + Eq, V> {
    Branch {
        intervals: Vec<K>,
        children: Vec<BNode<K, V>>,
    },
    Leaf(Vec<(K, V)>),
}

impl<K: Ord + Eq, V> BTree<K, V> {
    pub fn new() -> Self {
        BTree {
            root: BNode::Branch {
                intervals: Vec::new(),
                children: Vec::new(),
            },
        }
    }

    pub fn insert(&mut self, key: K, val: V) -> Option<V> {
        None
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        None
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        None
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        None
    }

    pub fn iter<'a>(&'a self) -> BTreeIter<'a, K, V> {
        BTreeIter {
            stack: vec![(&self.root, 0)],
        }
    }
}

pub struct BTreeIter<'a, K: Ord + Eq, V> {
    stack: Vec<(&'a BNode<K, V>, usize)>,
}

impl<'a, K: Ord + Eq, V> Iterator for BTreeIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}
