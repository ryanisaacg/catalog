mod context;

use std::fmt::Debug;
use std::mem::MaybeUninit;

pub use context::{BNodeContext, NodeId};

use crate::memtree::context::LeafEntry;

use self::context::{BranchEntry, NodeMut, NodeRef};

pub struct BTree<'a, K, V> {
    ctx: BNodeContext<'a, K, V>,
    root: NodeId,
}

impl<K: Ord + Clone + Debug, V: Clone + Debug> BTree<'_, K, V> {
    pub fn new(buffer: &mut [u8]) -> Self {
        let ctx = BNodeContext::new(buffer);
        let (root, _) = unsafe { ctx.alloc_branch(0) };
        BTree { ctx, root }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        get(&self.ctx, &self.root, key)
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let (new_root, old_value) = insert(&self.ctx, &self.root, key, value);
        if let Some(mut new_root) = new_root {
            std::mem::swap(&mut self.root, &mut new_root);
            unsafe {
                self.ctx.free(new_root);
            }
        }
        old_value
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let (new_root, old_value) = remove(&self.ctx, &self.root, key);
        if let Some(mut new_root) = new_root {
            std::mem::swap(&mut self.root, &mut new_root);
            unsafe {
                self.ctx.free(new_root);
            }
        }
        old_value
    }
}

fn get<'a, K: Ord + Debug, V: Debug>(
    ctx: &'a BNodeContext<'_, K, V>,
    node_id: &NodeId,
    key: &K,
) -> Option<&'a V> {
    match dbg!(unsafe { ctx.node(node_id) }) {
        NodeRef::Branch(branch) => {
            let idx = find_idx_from_interval(&branch.children[..], key);
            if idx >= branch.children.len() {
                None
            } else {
                let child_id = &branch.children[idx].node_id;
                get(ctx, child_id, key)
            }
        }
        NodeRef::Leaf(leaf) => {
            let idx = leaf
                .children
                .binary_search_by(|entry| entry.key.cmp(key))
                .ok()?;
            Some(&leaf.children[idx].value)
        }
    }
}

fn insert<'a, K: Ord + Clone + Debug, V: Clone + Debug>(
    ctx: &'a BNodeContext<'_, K, V>,
    node_id: &NodeId,
    key: K,
    mut value: V,
) -> (Option<NodeId>, Option<V>) {
    match dbg!(unsafe { ctx.node_mut(node_id) }) {
        NodeMut::Branch(branch) => {
            if branch.children.len() == 0 {
                let new_child_node_id = unsafe {
                    let (new_node_id, new_node) = ctx.alloc_leaf(1);
                    new_node.children[0] = MaybeUninit::new(LeafEntry {
                        key: key.clone(),
                        value,
                    });

                    new_node_id
                };
                let new_root_node_id = unsafe {
                    let (new_root_node_id, new_root) = ctx.alloc_branch(1);
                    new_root.children[0] = MaybeUninit::new(BranchEntry {
                        interval: key,
                        node_id: new_child_node_id,
                    });

                    new_root_node_id
                };

                return (Some(new_root_node_id), None);
            }
            let idx = find_idx_from_interval(&branch.children[..], &key);
            let child_node_id = &branch.children[idx].node_id;
            let (new_child_id, previous_val) = insert(ctx, child_node_id, key, value);

            if let Some(mut new_child_id) = new_child_id {
                // TODO: this might cause a new interval
                std::mem::swap(&mut branch.children[idx].node_id, &mut new_child_id);
                unsafe {
                    ctx.free(new_child_id);
                }
            }

            /*if let Some(new_child_id) = new_child_id {
                if children[idx].len() > MAX_ITEMS_IN_NODE {
                    let new_node = children[idx].split();
                    new_node.debug_validate_intervals();
                    let (new_first_key, _) = new_node.first().unwrap();
                    // TODO: can we avoid cloning here by storing references?
                    intervals.insert(idx, new_first_key.clone());
                    children.insert(idx + 1, new_node);
                }
                debug_assert!(children[idx].len() <= MAX_ITEMS_IN_NODE);
            }

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
            }*/

            (None, previous_val)
        }
        NodeMut::Leaf(leaf) => match leaf.children.binary_search_by(|entry| entry.key.cmp(&key)) {
            Ok(idx) => {
                let child_value = &mut leaf.children[idx].value;
                std::mem::swap(&mut value, child_value);
                (None, Some(value))
            }
            Err(insertion_idx) => {
                let new_node_id = unsafe {
                    let (new_node_id, new_leaf) = ctx.alloc_leaf(leaf.children.len() + 1);
                    for (i, child) in leaf.children.iter().enumerate() {
                        // TODO: get rid of this clone somehow
                        let new_leaf_idx = if i < insertion_idx { i } else { i + 1 };
                        new_leaf.children[new_leaf_idx] = MaybeUninit::new(child.clone());
                    }
                    new_leaf.children[insertion_idx] = MaybeUninit::new(LeafEntry { key, value });

                    new_node_id
                };
                (Some(new_node_id), None)
            }
        },
    }
}

fn remove<'a, K: Ord + Clone + Debug, V: Clone + Debug>(
    ctx: &'a BNodeContext<'_, K, V>,
    node_id: &NodeId,
    key: &K,
) -> (Option<NodeId>, Option<V>) {
    match dbg!(unsafe { ctx.node_mut(node_id) }) {
        NodeMut::Branch(branch) => {
            if branch.children.is_empty() {
                return (None, None);
            }

            let idx = find_idx_from_interval(&branch.children[..], key);
            let child_node_id = &branch.children[idx].node_id;
            // TODO: intervals can change
            let (_new_node_id, previous_val) = remove(ctx, child_node_id, key);

            /*if children[idx].len() < MIN_ITEMS_IN_NODE {
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
            }*/

            (None, previous_val)
        }
        NodeMut::Leaf(leaf) => {
            match leaf
                .children
                .binary_search_by(|child_key| child_key.key.cmp(key))
            {
                Ok(idx) => (None, Some(leaf.remove(idx).value)),
                Err(_) => (None, None),
            }
        }
    }
}

fn find_idx_from_interval<K: Ord>(entries: &[BranchEntry<K>], key: &K) -> usize {
    find_idx_from_interval_recursive(&entries[1..], key)
}

fn find_idx_from_interval_recursive<K: Ord>(entries: &[BranchEntry<K>], key: &K) -> usize {
    if entries.is_empty() {
        0
    } else {
        let halfway = entries.len() / 2;
        match key.cmp(&entries[halfway].interval) {
            std::cmp::Ordering::Less => find_idx_from_interval_recursive(&entries[0..halfway], key),
            std::cmp::Ordering::Equal => halfway + 1,
            std::cmp::Ordering::Greater => {
                halfway + 1 + find_idx_from_interval_recursive(&entries[(halfway + 1)..], key)
            }
        }
    }
}
