#![forbid(unsafe_code)]

use std::rc::{Rc, Weak};

use pui_cell::{IdCell, IdentifierExt};

pui_core::scalar_allocator! {
    pub thread_local struct NodeId;
}

type Id = pui_core::dynamic::Dynamic<NodeId, NodeId>;
type Token = pui_core::dynamic::DynamicToken<NodeId>;

pub struct NodeInner<T: ?Sized> {
    pub next: Vec<Rc<Node<T>>>,
    pub prev: Vec<Weak<Node<T>>>,
    pub value: T,
}

pub struct Node<T: ?Sized> {
    inner: IdCell<NodeInner<T>, Token>,
}

impl<T> Node<T> {
    pub fn new(value: T) -> Rc<Self> {
        Rc::new(Self {
            inner: IdCell::new(NodeInner {
                next: Vec::new(),
                prev: Vec::new(),
                value,
            }),
        })
    }

    pub fn push_next(self: &Rc<Self>, next: Rc<Self>) {
        let mut id = NodeId::reuse();
        id.get_mut(&next.inner).prev.push(Rc::downgrade(self));
        id.get_mut(&self.inner).next.push(next);
    }

    pub fn inner<'a>(&'a self, id: &'a Id) -> &'a NodeInner<T> { id.get(&self.inner) }

    pub fn inner_mut<'a>(&'a self, id: &'a mut Id) -> &'a mut NodeInner<T> { id.get_mut(&self.inner) }
}

#[test]
fn basic() {
    let x = Node::new(10);
    let y = Node::new(20);

    x.push_next(y.clone());

    let id = NodeId::reuse();
    let x = (*x).inner(&id);
    let y = (*y).inner(&id);

    assert_eq!(x.next.len(), 1);
    assert_eq!(id.get(&x.next[0].inner).value, 20);
    assert!(y.next.is_empty());
}
