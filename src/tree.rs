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
        self.root.get_mut(key)
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
    fn get(&self, key: &K) -> Option<&V> {
        match self {
            BNode::Branch {
                intervals,
                children,
            } => children[find_idx_from_interval(intervals, key)].get(key),
            BNode::Leaf(children) => {
                let idx = children
                    .binary_search_by(|(child_key, _)| child_key.cmp(key))
                    .ok()?;
                Some(&children[idx].1)
            }
        }
    }

    fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        match self {
            BNode::Branch {
                intervals,
                children,
            } => children[find_idx_from_interval(intervals, key)].get_mut(key),
            BNode::Leaf(children) => {
                let idx = children
                    .binary_search_by(|(child_key, _)| child_key.cmp(key))
                    .ok()?;
                Some(&mut children[idx].1)
            }
        }
    }

    fn insert(&mut self, key: K, mut val: V) -> Option<V> {
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
                        let idx = find_idx_from_interval(intervals, &key);
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

                if children.len() > MAX_ITEMS_IN_NODE {
                    let new_node = self.split();
                    let old_node = std::mem::take(self);
                    let (new_first_key, _) = new_node.first().unwrap();
                    *self = BNode::Branch {
                        // TODO: can we avoid cloning here by storing references?
                        intervals: vec![new_first_key.clone()],
                        children: vec![old_node, new_node],
                    };
                }

                val
            }
            BNode::Leaf(children) => {
                match children.binary_search_by(|child_key| child_key.0.cmp(&key)) {
                    Ok(idx) => {
                        let (_, child_value) = &mut children[idx];
                        std::mem::swap(&mut val, child_value);
                        Some(val)
                    }
                    Err(idx) => {
                        children.insert(idx, (key, val));
                        None
                    }
                }
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
            } => {
                let children_halfway = children.len() / 2;
                let split_children = children.drain(children_halfway..).collect();

                let interval_halfway = intervals.len() / 2;
                let split_interval = intervals.drain(interval_halfway..).collect();

                BNode::Branch {
                    intervals: split_interval,
                    children: split_children,
                }
            }
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

fn find_idx_from_interval<K: Ord>(intervals: &[K], key: &K) -> usize {
    if intervals.is_empty() {
        0
    } else {
        let halfway = intervals.len() / 2;
        match key.cmp(&intervals[halfway]) {
            std::cmp::Ordering::Less => find_idx_from_interval(&intervals[0..halfway], key),
            std::cmp::Ordering::Equal => halfway + 1,
            std::cmp::Ordering::Greater => {
                halfway + 1 + find_idx_from_interval(&intervals[(halfway + 1)..], key)
            }
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
