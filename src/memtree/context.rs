use std::{
    alloc::{GlobalAlloc, Layout},
    marker::PhantomData,
    mem::MaybeUninit,
    ptr,
};

use linked_list_allocator::LockedHeap;

// TODO: branch and leaf children are always MaybeUninit, and it's just part of the safety contract
// to initialize them?

// TODO: track capacity in the node header to allow nodes to grow and shrink a bit

#[repr(C)]
#[derive(Debug)]
struct NodeHeader {
    tag: NodeTag,
    len: usize,
}

#[repr(C)]
#[derive(Debug)]
pub struct Branch<K> {
    header: NodeHeader,
    pub children: [BranchEntry<K>],
}

#[repr(C)]
#[derive(Debug)]
pub struct BranchMaybeUninit<K> {
    header: NodeHeader,
    pub children: [MaybeUninit<BranchEntry<K>>],
}

#[derive(Debug)]
pub struct BranchEntry<K> {
    pub interval: K,
    pub node_id: NodeId,
}

impl<K: Clone> Clone for BranchEntry<K> {
    fn clone(&self) -> Self {
        BranchEntry {
            interval: self.interval.clone(),
            node_id: NodeId(self.node_id.0),
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct Leaf<K, V> {
    header: NodeHeader,
    pub children: [LeafEntry<K, V>],
}

impl<K, V> Leaf<K, V> {
    pub fn remove(&mut self, idx: usize) -> LeafEntry<K, V> {
        assert!(idx < self.children.len());

        // infallible
        let ret;
        unsafe {
            // the place we are taking from.
            let ptr = self.children.as_mut_ptr().add(idx);
            // copy it out, unsafely having a copy of the value on
            // the stack and in the vector at the same time.
            ret = ptr::read(ptr);

            // Shift everything down to fill in that spot.
            ptr::copy(ptr.add(1), ptr, self.header.len - idx - 1);
        }
        self.header.len -= 1;

        ret
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct LeafMaybeUninit<K, V> {
    header: NodeHeader,
    pub children: [MaybeUninit<LeafEntry<K, V>>],
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct LeafEntry<K, V> {
    pub key: K,
    pub value: V,
}

#[derive(Debug)]
#[repr(transparent)]
pub struct NodeId(usize);

pub struct BNodeContext<'a, K, V> {
    allocator: &'a LockedHeap,
    buffer: *mut u8,
    _k: PhantomData<K>,
    _v: PhantomData<V>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
enum NodeTag {
    Branch = 0x0B,
    Leaf = 0x0C,
}

#[derive(Clone, Debug)]
pub enum NodeRef<'a, K, V> {
    Branch(&'a Branch<K>),
    Leaf(&'a Leaf<K, V>),
}

#[derive(Debug)]
pub enum NodeMut<'a, K, V> {
    Branch(&'a mut Branch<K>),
    Leaf(&'a mut Leaf<K, V>),
}

impl<K, V> BNodeContext<'_, K, V> {
    pub fn new(buffer: &mut [u8]) -> Self {
        let heap = LockedHeap::empty();
        let size = buffer.len();
        let heap_start = buffer.as_mut_ptr();

        let allocator = unsafe {
            heap.lock().init(heap_start, size);

            let memory_region = heap.alloc(Layout::new::<LockedHeap>()) as *mut LockedHeap;
            *memory_region = heap;

            memory_region.as_ref().unwrap()
        };

        BNodeContext {
            allocator,
            buffer: heap_start,
            _k: PhantomData,
            _v: PhantomData,
        }
    }

    /// # Safety
    /// You must initialize all data in the BranchMaybeUninit immediately before calling any other
    /// methods on BNodeContext
    pub unsafe fn alloc_branch(&self, len: usize) -> (NodeId, &mut BranchMaybeUninit<K>) {
        let header = NodeHeader {
            tag: NodeTag::Branch,
            len,
        };
        let layout = self.branch_layout(len);
        unsafe {
            let ptr = self.allocator.alloc(layout);
            assert!(!ptr.is_null());
            let header_ptr = ptr as *mut NodeHeader;
            header_ptr.write(header);

            let node_id = NodeId(
                ptr.offset_from(self.buffer)
                    .try_into()
                    .expect("allocations must be within buffer"),
            );

            let ptr_slice = ptr::slice_from_raw_parts(ptr, layout.size());
            let reference = (ptr_slice as *mut BranchMaybeUninit<K>).as_mut().unwrap();

            (node_id, reference)
        }
    }

    /// # Safety
    /// You must initialize all data in the LeafMaybeUninit immediately before calling any other
    /// methods on BNodeContext
    pub unsafe fn alloc_leaf(&self, len: usize) -> (NodeId, &mut LeafMaybeUninit<K, V>) {
        let header = NodeHeader {
            tag: NodeTag::Leaf,
            len,
        };
        let layout = self.leaf_layout(len);
        unsafe {
            let ptr = self.allocator.alloc(layout);
            assert!(!ptr.is_null());
            let header_ptr = ptr as *mut NodeHeader;
            header_ptr.write(header);

            let node_id = NodeId(
                ptr.offset_from(self.buffer)
                    .try_into()
                    .expect("allocations must be within buffer"),
            );

            let ptr_slice = ptr::slice_from_raw_parts(ptr, layout.size());
            let reference = (ptr_slice as *mut LeafMaybeUninit<K, V>).as_mut().unwrap();

            (node_id, reference)
        }
    }

    /// # Safety
    /// You must not free the same node_id twice
    pub unsafe fn free(&self, node_id: NodeId) {
        let ptr = self.buffer.add(node_id.0);
        let header_ptr = ptr as *const NodeHeader;
        let header = header_ptr.read();
        let layout = match header.tag {
            NodeTag::Branch => self.branch_layout(header.len),
            NodeTag::Leaf => self.leaf_layout(header.len),
        };
        self.allocator.dealloc(ptr, layout);
    }

    fn branch_layout(&self, len: usize) -> Layout {
        let size = std::mem::size_of::<NodeHeader>() + len * std::mem::size_of::<BranchEntry<K>>();
        Layout::from_size_align(
            size,
            std::mem::align_of::<NodeHeader>().max(std::mem::align_of::<BranchEntry<K>>()),
        )
        .unwrap()
    }

    fn leaf_layout(&self, len: usize) -> Layout {
        let size = std::mem::size_of::<NodeHeader>() + len * std::mem::size_of::<LeafEntry<K, V>>();
        Layout::from_size_align(
            size,
            std::mem::align_of::<NodeHeader>().max(std::mem::align_of::<LeafEntry<K, V>>()),
        )
        .unwrap()
    }

    fn alloc<T>(&self, value: T) -> NodeId {
        let layout = Layout::new::<T>();

        unsafe {
            let ptr = self.allocator.alloc(layout);
            assert!(!ptr.is_null());
            (ptr as *mut T).write(value);
            NodeId(
                ptr.offset_from(self.buffer)
                    .try_into()
                    .expect("allocations must be within buffer"),
            )
        }
    }

    /// # Safety
    /// node_id must have been generated by this context and not yet freed
    unsafe fn header(&self, node_id: &NodeId) -> *mut NodeHeader {
        self.buffer.add(node_id.0) as *mut NodeHeader
    }

    pub unsafe fn node(&self, node_id: &NodeId) -> NodeRef<'_, K, V> {
        let header_ptr = self.header(node_id);
        let header = header_ptr.read();
        match header.tag {
            NodeTag::Branch => NodeRef::Branch(to_branch(header_ptr).as_ref().unwrap()),
            NodeTag::Leaf => NodeRef::Leaf(to_leaf(header_ptr).as_ref().unwrap()),
        }
    }

    pub unsafe fn node_mut(&self, node_id: &NodeId) -> NodeMut<'_, K, V> {
        let header_ptr = self.header(node_id);
        let header = header_ptr.read();
        match header.tag {
            NodeTag::Branch => NodeMut::Branch(to_branch(header_ptr).as_mut().unwrap()),
            NodeTag::Leaf => NodeMut::Leaf(to_leaf(header_ptr).as_mut().unwrap()),
        }
    }
}

/// # Safety
/// header_ptr must be a pointer to a valid Leaf
unsafe fn to_leaf<K, V>(header_ptr: *mut NodeHeader) -> *mut Leaf<K, V> {
    let header = header_ptr.read();
    assert_eq!(header.tag, NodeTag::Leaf);
    let wide_ptr = ptr::slice_from_raw_parts(header_ptr as *mut u8, header.len);
    wide_ptr as *mut Leaf<K, V>
}

/// # Safety
/// header_ptr must be a pointer to a valid Branch
unsafe fn to_branch<K>(header_ptr: *mut NodeHeader) -> *mut Branch<K> {
    let header = header_ptr.read();
    assert_eq!(header.tag, NodeTag::Branch);
    let wide_ptr = ptr::slice_from_raw_parts(header_ptr as *mut u8, header.len);
    wide_ptr as *mut Branch<K>
}
