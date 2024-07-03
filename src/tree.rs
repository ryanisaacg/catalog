#[derive(Debug)]
pub struct BTree<K, V> {
    root: BNode<K, V>,
}

#[derive(Clone, Debug)]
enum BNode<K, V> {
    Branch {
        intervals: Vec<K>,
        children: Vec<BNode<K, V>>,
    },
    Leaf(Vec<(K, V)>),
}

struct Unsized<K, V> {
    children: [(K, BNode<K, V>)],
}

impl<K: Ord + Eq + Clone, V: Clone> Default for BNode<K, V> {
    fn default() -> Self {
        Self::Leaf(Vec::default())
    }
}

impl<K, V> Default for BTree<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> BTree<K, V> {
    pub fn new() -> Self {
        BTree {
            root: BNode::Branch {
                intervals: Vec::new(),
                children: Vec::new(),
            },
        }
    }

    pub fn iter(&self) -> BTreeIter<'_, K, V> {
        BTreeIter {
            stack: vec![(&self.root, 0)],
        }
    }
}

impl<K: Ord, V> BTree<K, V> {
    pub fn get(&self, key: &K) -> Option<&V> {
        self.root.get(key)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.root.get_mut(key)
    }
}

impl<K: Ord + Eq + Clone, V: Clone> BTree<K, V> {
    pub fn insert(&mut self, key: K, val: V) -> Option<V>
    where
        K: std::fmt::Debug,
        V: std::fmt::Debug,
    {
        self.root.insert(key, val)
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.root.remove(key)
    }
}

const MIN_ITEMS_IN_NODE: usize = 2;
const MAX_ITEMS_IN_NODE: usize = 4;

impl<K: Ord, V> BNode<K, V> {
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

                let interval_halfway = children_halfway - 1;
                let split_interval = intervals.drain((interval_halfway + 1)..).collect();
                intervals.remove(interval_halfway);

                self.debug_validate_intervals();

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

    fn debug_validate_intervals(&self) {
        #[cfg(debug_assertions)]
        match self {
            BNode::Branch {
                intervals,
                children,
            } => {
                debug_assert_eq!(intervals.len() + 1, children.len());
                for i in 0..intervals.len() {
                    debug_assert!(intervals[i] == children[i + 1].first().unwrap().0);
                }
            }
            BNode::Leaf(_) => {}
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

impl<K: Ord + Clone, V: Clone> BNode<K, V> {
    fn insert(&mut self, key: K, mut val: V) -> Option<V> {
        match self {
            BNode::Branch {
                intervals,
                children,
            } => {
                if children.is_empty() {
                    children.push(BNode::Leaf(vec![(key, val)]));
                    return None;
                }

                let idx = find_idx_from_interval(intervals, &key);
                let previous_val = children[idx].insert(key, val);
                if children[idx].len() > MAX_ITEMS_IN_NODE {
                    let new_node = children[idx].split();
                    new_node.debug_validate_intervals();
                    let (new_first_key, _) = new_node.first().unwrap();
                    // TODO: can we avoid cloning here by storing references?
                    intervals.insert(idx, new_first_key.clone());
                    children.insert(idx + 1, new_node);
                }
                debug_assert!(children[idx].len() <= MAX_ITEMS_IN_NODE);

                if children.len() > MAX_ITEMS_IN_NODE {
                    let new_node = self.split();
                    new_node.debug_validate_intervals();
                    let old_node = std::mem::take(self);
                    let (new_first_key, _) = new_node.first().unwrap();
                    *self = BNode::Branch {
                        // TODO: can we avoid cloning here by storing references?
                        intervals: vec![new_first_key.clone()],
                        children: vec![old_node, new_node],
                    };
                }

                previous_val
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

    fn remove(&mut self, key: &K) -> Option<V> {
        match self {
            BNode::Branch {
                intervals,
                children,
            } => {
                if children.is_empty() {
                    return None;
                }

                let idx = find_idx_from_interval(intervals, key);
                let previous = children[idx].remove(key);

                if children[idx].len() < MIN_ITEMS_IN_NODE {
                    if idx > 0 {
                        // TODO: This could be an expensive clone
                        children[idx] = children[idx - 1].merged(&children[idx]);
                        children.remove(idx - 1);
                        intervals.remove(idx - 1);
                    } else if idx + 1 < children.len() {
                        // TODO: This could be an expensive clone
                        children[idx] = children[idx].merged(&children[idx + 1]);
                        children.remove(idx + 1);
                        intervals.remove(idx);
                    }
                }
                if children.len() > 1 {
                    debug_assert!(children[idx].len() >= MIN_ITEMS_IN_NODE);
                }

                previous
            }
            BNode::Leaf(children) => {
                match children.binary_search_by(|child_key| child_key.0.cmp(key)) {
                    Ok(idx) => Some(children.remove(idx).1),
                    Err(_) => None,
                }
            }
        }
    }

    fn merged(&self, other: &Self) -> Self {
        let Some(other_first) = other.first() else {
            return self.clone();
        };
        match (self, other) {
            (
                BNode::Branch {
                    children: a_children,
                    intervals: a_intervals,
                },
                BNode::Branch {
                    children: b_children,
                    intervals: b_intervals,
                },
            ) => {
                let mut children = Vec::new();
                children.extend(a_children.iter().cloned());
                children.extend(b_children.iter().cloned());
                let mut intervals = Vec::new();
                intervals.extend(a_intervals.iter().cloned());
                intervals.push(other_first.0.clone());
                intervals.extend(b_intervals.iter().cloned());
                BNode::Branch {
                    intervals,
                    children,
                }
            }
            (
                BNode::Branch {
                    intervals,
                    children,
                },
                BNode::Leaf(_),
            ) => {
                let mut intervals = intervals.clone();
                let mut children = children.clone();
                intervals.push(other_first.0.clone());
                children.push(other.clone());
                BNode::Branch {
                    intervals,
                    children,
                }
            }
            (BNode::Leaf(_), BNode::Branch { .. }) => todo!(),
            (BNode::Leaf(_), BNode::Leaf(_)) => BNode::Branch {
                intervals: vec![other_first.0.clone()],
                children: vec![self.clone(), other.clone()],
            },
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

pub struct BTreeIter<'a, K, V> {
    stack: Vec<(&'a BNode<K, V>, usize)>,
}

impl<'a, K, V> Iterator for BTreeIter<'a, K, V> {
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
