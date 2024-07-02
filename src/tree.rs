#[derive(Debug)]
pub struct BTree<K: Ord + Eq + Clone, V> {
    root: BNode<K, V>,
}

#[derive(Debug)]
enum BNode<K: Ord + Eq + Clone, V> {
    Branch {
        intervals: Vec<K>,
        children: Vec<BNode<K, V>>,
    },
    Leaf(Vec<(K, V)>),
}

impl<K: Ord + Eq + Clone, V> Default for BNode<K, V> {
    fn default() -> Self {
        Self::Leaf(Vec::default())
    }
}

impl<K: Ord + Eq + Clone, V> BTree<K, V> {
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

const MAX_ITEMS_IN_NODE: usize = 4;

impl<K: Ord + Eq + Clone, V> BNode<K, V> {
    pub fn get(&self, key: &K) -> Option<&V> {
        match self {
            BNode::Branch {
                intervals,
                children,
            } => {
                let mut idx = 0;
                // TODO: binary search
                while idx < intervals.len() && key >= &intervals[idx] {
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
            } => {
                let val = match children.len() {
                    0 => {
                        children.push(BNode::Leaf(vec![(key, val)]));
                        None
                    }
                    _ => {
                        let mut idx = 0;
                        while idx < intervals.len() && &key >= &intervals[idx] {
                            idx += 1;
                        }
                        let previous_val = children[idx].insert(key, val);
                        if children[idx].len() > MAX_ITEMS_IN_NODE {
                            let new_node = children[idx].split();
                            let (new_first_key, _) = new_node.first().unwrap();
                            // TODO: can we avoid cloning here by storing references?
                            intervals.insert(idx, new_first_key.clone());
                            children.insert(idx + 1, new_node);
                        }
                        debug_assert!(children[idx].len() <= MAX_ITEMS_IN_NODE);
                        previous_val
                    }
                };

                val
            }
            BNode::Leaf(children) => {
                let mut intended_idx = children.len();
                for idx in 0..children.len() {
                    let (child_key, child_value) = &mut children[idx];
                    if &key == child_key {
                        std::mem::swap(&mut val, child_value);
                        return Some(val);
                    } else if &key < child_key {
                        intended_idx = idx;
                        break;
                    }
                }
                children.insert(intended_idx, (key, val));
                None
            }
        }
    }

    fn first(&self) -> Option<&(K, V)> {
        match self {
            BNode::Branch {
                intervals: _,
                children,
            } => children.first().and_then(|child| child.first()),
            BNode::Leaf(children) => children.first(),
        }
    }

    fn split(&mut self) -> Self {
        match self {
            BNode::Branch {
                intervals,
                children,
            } => todo!(),
            BNode::Leaf(children) => {
                let halfway = children.len() / 2;
                let split_children = children.drain(halfway..).collect();
                BNode::Leaf(split_children)
            }
        }
    }

    fn len(&self) -> usize {
        match self {
            BNode::Branch {
                intervals: _,
                children,
            } => children.len(),
            BNode::Leaf(children) => children.len(),
        }
    }
}

pub struct BTreeIter<'a, K: Ord + Eq + Clone, V> {
    stack: Vec<(&'a BNode<K, V>, usize)>,
}

impl<'a, K: Ord + Eq + Clone, V> Iterator for BTreeIter<'a, K, V> {
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
