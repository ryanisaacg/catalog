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
        self.root.insert(key, val)
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.root.get(key)
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

impl<K: Ord + Eq, V> BNode<K, V> {
    pub fn get(&self, key: &K) -> Option<&V> {
        match self {
            BNode::Branch {
                intervals,
                children,
            } => {
                let mut idx = 0;
                // TODO: binary search
                while idx < intervals.len() && key < &intervals[idx] {
                    idx += 1;
                }
                children[idx].get(key)
            }
            BNode::Leaf(children) => {
                // TODO: binary search
                children.iter().find_map(|(child_key, child_value)| {
                    if key == child_key {
                        Some(child_value)
                    } else {
                        None
                    }
                })
            }
        }
    }

    pub fn insert(&mut self, key: K, mut val: V) -> Option<V> {
        match self {
            BNode::Branch {
                intervals,
                children,
            } => match children.len() {
                0 => {
                    children.push(BNode::Leaf(vec![(key, val)]));
                    None
                }
                _ => {
                    let mut idx = 0;
                    while idx < intervals.len() && &key < &intervals[idx] {
                        idx += 1;
                    }
                    children[idx].insert(key, val)
                } // TODO: assert that branch doesn't have too many children
            },
            BNode::Leaf(children) => {
                let mut intended_idx = children.len();
                for idx in 0..children.len() {
                    let (child_key, child_value) = &mut children[idx];
                    if &key == child_key {
                        std::mem::swap(&mut val, child_value);
                        return Some(val);
                    } else if &key > child_key {
                        intended_idx = idx;
                        break;
                    }
                }
                children.insert(intended_idx, (key, val));
                None
            }
        }
    }
}

pub struct BTreeIter<'a, K: Ord + Eq, V> {
    stack: Vec<(&'a BNode<K, V>, usize)>,
}

impl<'a, K: Ord + Eq, V> Iterator for BTreeIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        match self.stack.last_mut() {
            Some((node, idx)) => match node {
                BNode::Branch {
                    intervals: _,
                    children,
                } => {
                    let child_idx = *idx;
                    if child_idx < children.len() {
                        *idx += 1;
                        self.stack.push((&children[child_idx], 0));
                        self.next()
                    } else {
                        self.stack.pop();
                        self.next()
                    }
                }
                BNode::Leaf(children) => {
                    let child_idx = *idx;
                    if child_idx < children.len() {
                        *idx += 1;
                        let (key, val) = &children[child_idx];
                        Some((key, val))
                    } else {
                        self.stack.pop();
                        self.next()
                    }
                }
            },
            None => None,
        }
    }
}
