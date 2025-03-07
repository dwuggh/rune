use std::cmp::Ordering;
use std::fmt::{Arguments, Debug, Write};
use std::ptr;
pub mod range;
pub use range::TextRange;

fn safe_mut<'a, T>(n: *mut T) -> &'a mut T {
    unsafe { n.as_mut().unwrap() }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Color {
    Red = 0,
    Black,
}

impl Color {
    pub fn flip(&self) -> Color {
        match self {
            Color::Red => Color::Black,
            Color::Black => Color::Red,
        }
    }
}

#[derive(Clone)]
pub struct Node<T: Clone> {
    pub key: TextRange,
    pub val: T,
    left: MaybeNode<T>,
    right: MaybeNode<T>,
    color: Color,
    parent: *mut Node<T>,
    is_right_child: bool,
    n: usize,
}
pub type BoxedNode<T> = Box<Node<T>>;
pub type MaybeNode<T> = Option<BoxedNode<T>>;

impl<T: Clone> Node<T> {
    #[inline]
    pub fn red(node: &MaybeNode<T>) -> bool {
        node.as_ref().is_some_and(|n| n.color == Color::Red)
    }

    #[inline]
    pub fn new(key: TextRange, val: T) -> Self {
        Self::new_with_parent(key, val, ptr::null_mut(), false)
    }

    #[inline]
    pub fn new_with_parent(
        key: TextRange,
        val: T,
        parent: *mut Node<T>,
        is_right_child: bool,
    ) -> Node<T> {
        Self {
            key,
            val,
            left: None,
            right: None,
            color: Color::Red,
            n: 1,
            parent,
            is_right_child,
        }
    }

    #[inline]
    pub fn new_boxed(key: TextRange, val: T) -> BoxedNode<T> {
        Box::new(Self::new(key, val))
    }

    #[inline]
    pub fn new_boxed_with_parent(
        key: TextRange,
        val: T,
        parent: *mut Node<T>,
        is_right_child: bool,
    ) -> BoxedNode<T> {
        Box::new(Self::new_with_parent(key, val, parent, is_right_child))
    }

    #[inline]
    pub fn set_parent(&mut self, parent: &mut Node<T>) {
        self.parent = parent;
    }

    /// perform the following operation, \\ is the red link:
    ///      |            |
    ///      n            x
    ///     / \\        // \
    ///        x    =>  n
    ///       / \      / \
    ///      c            c
    pub fn rotate_left(node: &mut BoxedNode<T>) -> Option<&mut BoxedNode<T>> {
        let mut x = node.right.take()?;
        let mut c = x.left.take();
        if let Some(ref mut c) = c {
            c.parent = node.as_mut();
            c.is_right_child = true;
        }
        node.right = c;
        x.color = node.color;
        x.parent = node.parent;
        x.is_right_child = node.is_right_child;
        node.parent = x.as_mut();
        node.is_right_child = false;
        x.n = node.n;
        node.color = Color::Red;
        node.n = node.n();
        // node and x swapped content
        std::mem::swap(node, &mut x);
        node.left.replace(x);
        Some(node)
    }

    /// perform the following operation, \\ is the red link:
    ///      |            |
    ///      n            x
    ///    // \          / \\
    ///    x       =>       n
    ///   / \              / \
    ///      c            c
    pub fn rotate_right(node: &mut BoxedNode<T>) -> Option<&mut BoxedNode<T>> {
        let mut x = node.left.take()?;
        let mut c = x.right.take();
        if let Some(ref mut c) = c {
            c.parent = node.as_mut();
            c.is_right_child = false;
        }
        node.left = c;
        x.color = node.color;
        x.parent = node.parent;
        x.is_right_child = node.is_right_child;
        node.parent = x.as_mut();
        node.is_right_child = true;
        node.color = Color::Red;
        x.n = node.n;
        node.n = node.n();

        std::mem::swap(node, &mut x);
        node.right.replace(x);
        Some(node)
    }

    /// total number of items in this sub-tree
    #[inline]
    pub fn n(&self) -> usize {
        let l = match self.left {
            Some(ref n) => n.n,
            None => 0,
        };
        let r = match self.right {
            Some(ref n) => n.n,
            None => 0,
        };
        1 + l + r
    }

    fn min(&self) -> &Node<T> {
        if let Some(ref l) = self.left {
            l.min()
        } else {
            self
        }
    }

    fn min_mut(&mut self) -> &mut Node<T> {
        if let Some(ref mut l) = self.left {
            l.min_mut()
        } else {
            self
        }
    }

    #[inline]
    fn parent(&self) -> Option<&Node<T>> {
        unsafe { self.parent.as_ref() }
    }

    #[inline]
    fn parent_mut(&self) -> Option<&mut Node<T>> {
        unsafe { self.parent.as_mut() }
    }

    fn get_node_mut(&mut self, key: TextRange) -> Option<&mut Node<T>> {
        match key.cmp(&self.key) {
            Ordering::Equal => Some(self),
            Ordering::Less => self.left.as_mut().and_then(|n| n.get_node_mut(key)),
            Ordering::Greater => self.right.as_mut().and_then(|n| n.get_node_mut(key)),
        }
    }

    fn get(&self, key: TextRange) -> Option<T> {
        match key.cmp(&self.key) {
            Ordering::Equal => Some(self.val.clone()),
            Ordering::Less => self.left.as_ref().and_then(|n| n.get(key)),
            Ordering::Greater => self.right.as_ref().and_then(|n| n.get(key)),
        }
    }

    /* ------------------------------------------------------------ */
    /*                       insertion                              */
    /* ------------------------------------------------------------ */

    pub fn insert_at<'a, F: Fn(T, T) -> T>(
        node: &'a mut MaybeNode<T>,
        key: TextRange,
        val: T,
        parent: *mut Node<T>,
        is_right_child: bool,
        merge_fn: &F,
    ) -> Option<&'a mut BoxedNode<T>> {
        if key.start == key.end {
            return None;
        }
        match node {
            Some(ref mut n) => Node::insert_at_inner(n, key, val, merge_fn),
            None => {
                node.replace(Node::new_boxed_with_parent(key, val.clone(), parent, is_right_child));
                node.as_mut()
            }
        }
    }

    fn insert_at_inner<'a, F: Fn(T, T) -> T>(
        node: &'a mut BoxedNode<T>,
        mut key: TextRange,
        val: T,
        merge_fn: &F,
    ) -> Option<&'a mut BoxedNode<T>> {
        let intersect = key.intersects(node.key);
        // TODO too taunting
        let ptr: *mut Node<T> = node.as_mut();
        if intersect {
            if key.start < node.key.start {
                let key_left = key.split_at(node.key.start, true);
                Node::insert_at(&mut node.left, key_left, val.clone(), ptr, false, merge_fn);

                if key.end < node.key.end {
                    let key_right = node.key.split_at(key.end, false);
                    Node::insert_at(
                        &mut node.right,
                        key_right,
                        node.val.clone(),
                        ptr,
                        true,
                        merge_fn,
                    );
                } else if key.end > node.key.end {
                    let key_right = key.split_at(node.key.end, false);
                    Node::insert_at(&mut node.right, key_right, val.clone(), ptr, true, merge_fn);
                }
            } else {
                let key_left = node.key.split_at(key.start, true);
                Node::insert_at(&mut node.left, key_left, node.val.clone(), ptr, false, merge_fn);

                if key.end < node.key.end {
                    let key_right = node.key.split_at(key.end, false);
                    Node::insert_at(
                        &mut node.right,
                        key_right,
                        node.val.clone(),
                        ptr,
                        true,
                        merge_fn,
                    );
                } else if key.end > node.key.end {
                    let key_right = key.split_at(node.key.end, false);
                    Node::insert_at(&mut node.right, key_right, val.clone(), ptr, true, merge_fn);
                }
            }
        }
        if key.start == key.end {
            return None;
        }
        let cmp = key.cmp(&node.key);
        match cmp {
            Ordering::Less => {
                Node::insert_at(&mut node.left, key, val, ptr, false, merge_fn)?;
            }
            Ordering::Equal => {
                node.val = merge_fn(val, node.val.clone());
            }
            Ordering::Greater => {
                Node::insert_at(&mut node.right, key, val, ptr, true, merge_fn)?;
            }
        }

        // cond1: r_red && !l_red
        if Node::red(&node.right) && !Node::red(&node.left) {
            Node::rotate_left(node)?;
        }

        // cond2: l_red && ll_red
        let cond2 = match node.left {
            Some(ref nl) => nl.color == Color::Red && Node::red(&nl.left),
            None => false,
        };
        if cond2 {
            Node::rotate_right(node)?;
        }

        // cond3: l_red && r_red
        if let (Some(l), Some(r)) = (&mut node.left, &mut node.right) {
            if l.color == Color::Red && r.color == Color::Red {
                l.color = Color::Black;
                r.color = Color::Black;
                node.color = Color::Red;
            }
        }
        // update node's size
        node.n = node.n();
        Some(node)
    }

    /* ------------------------------------------------------------ */
    /*                        deletion                              */
    /* ------------------------------------------------------------ */

    /// if node.left and node.right are both red, mark them black turn node to red.
    fn flip_colors(n: &mut BoxedNode<T>) {
        if let Some(ref mut l) = n.left {
            l.color = l.color.flip();
        }
        if let Some(ref mut r) = n.right {
            r.color = r.color.flip();
        }
        n.color = n.color.flip();
    }

    /// rotate left if node.right is red
    fn balance(node: &mut BoxedNode<T>) -> Option<()> {
        if Node::red(&node.right) {
            Node::rotate_left(node)?;
        }
        Some(())
    }

    /// Assuming that h is red and both h.left and h.left.left
    /// are black, make h.left or one of its children red.
    fn move_red_left(node: &mut BoxedNode<T>) -> Option<()> {
        Node::flip_colors(node);
        // h.right.left == Red
        if let Some(ref mut nr) = node.right.as_mut() {
            if Node::red(&nr.left) {
                Node::rotate_right(nr)?;
                Node::rotate_left(node)?;
                Node::flip_colors(node);
            }
        }
        Some(())
    }

    fn move_red_right(node: &mut BoxedNode<T>) -> Option<()> {
        Node::flip_colors(node);
        // h.left.left == Red
        let cond = match node.left {
            Some(ref l) => Node::red(&l.left),
            None => false,
        };
        if cond {
            Node::rotate_right(node)?;
            Node::flip_colors(node);
        }
        Some(())
    }

    fn delete_min(node: &mut MaybeNode<T>) -> MaybeNode<T> {
        let n = node.as_mut()?;
        match n.left {
            Some(ref mut l) => {
                if l.color == Color::Black && !Node::red(&l.left) {
                    Node::move_red_left(n)?;
                }
            }
            None => {
                return node.take();
            }
        }
        let result = Node::delete_min(&mut n.left);
        Node::balance(n)?;
        result
    }

    fn delete_max(node: &mut MaybeNode<T>) -> MaybeNode<T> {
        let n = node.as_mut()?;
        if Node::red(&n.left) {
            Node::rotate_right(n);
        }
        let n = node.as_mut()?;
        match n.right {
            Some(ref mut r) => {
                if r.color == Color::Black && !Node::red(&r.left) {
                    Node::move_red_right(n)?;
                }
            }
            None => {
                return node.take();
            }
        }
        let result = Node::delete_max(&mut n.right);
        Node::balance(n)?;
        result
    }

    fn delete(node: &mut MaybeNode<T>, key: TextRange) -> MaybeNode<T> {
        let n = node.as_mut()?;
        let result = if key < n.key {
            // n.left != red && n.left.left != red
            if let Some(ref mut l) = n.left {
                if l.color == Color::Black && !Node::red(&l.left) {
                    Node::move_red_left(n).unwrap();
                }
            }
            Node::delete(&mut n.left, key)
        } else {
            if Node::red(&n.left) {
                Node::rotate_right(n).unwrap();
            }
            if key == n.key && n.right.is_none() {
                return node.take();
                // return None;
            }

            let cond = if let Some(ref r) = n.right {
                r.color == Color::Black && !Node::red(&r.left)
            } else {
                true
            };

            if cond {
                Node::move_red_right(n).unwrap();
            }

            if key == n.key {
                let mut result = Node::delete_min(&mut n.right);
                let r_min = result.as_mut().unwrap();
                r_min.left = n.left.take();
                r_min.right = n.right.take();
                r_min.color = n.color;
                r_min.is_right_child = n.is_right_child;
                r_min.parent = n.parent;
                r_min.n = n.n;
                let ptr: *mut Node<T> = r_min.as_mut();
                if let Some(n) = &mut r_min.left {
                    n.parent = ptr;
                }
                if let Some(n) = &mut r_min.right {
                    n.parent = ptr;
                }
                std::mem::swap(r_min, n);
                result
            } else {
                Node::delete(&mut n.right, key)
            }
        };

        Node::balance(n)?;
        result
    }

    /* ------------------------------------------------------------ */
    /*                        intersection                          */
    /* ------------------------------------------------------------ */

    pub fn next(&self) -> Option<&Node<T>> {
        let mut n = self;
        if let Some(ref r) = self.right {
            n = r;
            while let Some(ref l) = n.left {
                n = l;
            }
            return Some(n);
        }
        while let Some(parent) = n.parent() {
            if !n.is_right_child {
                return Some(parent);
            }
            n = parent;
        }
        None
    }

    pub fn next_raw(&mut self) -> Option<*mut Node<T>> {
        let mut n: *mut Node<T> = self;
        if let Some(r) = self.right.as_mut() {
            n = r.as_mut();
            while let Some(ref mut l) = safe_mut(n).left {
                n = l.as_mut();
            }
            return Some(n);
        }
        while let Some(parent) = (safe_mut(n)).parent_mut() {
            if !(safe_mut(n)).is_right_child {
                return Some(parent);
            }
            n = parent;
        }
        None
    }

    pub fn prev(&self) -> Option<&Node<T>> {
        let mut n = self;
        if let Some(ref l) = self.left {
            n = l;
            while let Some(ref r) = n.right {
                n = r;
            }
            return Some(n);
        }
        while let Some(parent) = n.parent() {
            if n.is_right_child {
                return Some(parent);
            }
            n = parent;
        }
        None
    }

    pub fn prev_raw(&mut self) -> Option<*mut Node<T>> {
        let mut n: *mut Node<T> = self;
        if let Some(l) = self.left.as_mut() {
            n = l.as_mut();
            while let Some(ref mut r) = safe_mut(n).right {
                n = r.as_mut();
            }
            return Some(n);
        }
        while let Some(parent) = (safe_mut(n)).parent_mut() {
            if (safe_mut(n)).is_right_child {
                return Some(parent);
            }
            n = parent;
        }
        None
    }

    pub fn find_intersects<'a>(&'a self, range: TextRange, results: &mut Vec<&'a Node<T>>) {
        let ord = range.strict_order(&self.key);
        match ord {
            Some(Ordering::Less) => {
                if let Some(ref l) = self.left {
                    l.find_intersects(range, results);
                }
            }
            Some(Ordering::Greater) => {
                if let Some(ref r) = self.right {
                    r.find_intersects(range, results);
                }
            }
            _ => {
                if let Some(ref l) = self.left {
                    l.find_intersects(range, results);
                }
                results.push(self);
                if let Some(ref r) = self.right {
                    r.find_intersects(range, results);
                }
            }
        }
    }

    fn find_intersect_min(&self, range: TextRange) -> Option<&Node<T>> {
        let ord = range.strict_order(&self.key);
        match ord {
            Some(Ordering::Less) => self.left.as_ref().and_then(|l| l.find_intersect_min(range)),
            Some(Ordering::Equal) => Some(self),
            Some(Ordering::Greater) => {
                self.right.as_ref().and_then(|r| r.find_intersect_min(range))
            }
            _ => self.left.as_ref().and_then(|l| l.find_intersect_min(range)).or(Some(self)),
        }
    }

    fn find_intersect_max(&self, range: TextRange) -> Option<&Node<T>> {
        let ord = range.strict_order(&self.key);
        match ord {
            Some(Ordering::Less) => self.left.as_ref().and_then(|l| l.find_intersect_max(range)),
            Some(Ordering::Equal) => Some(self),
            Some(Ordering::Greater) => {
                self.right.as_ref().and_then(|l| l.find_intersect_max(range))
            }
            _ => self.right.as_ref().and_then(|l| l.find_intersect_max(range)).or(Some(self)),
        }
    }

    /// Recursively applies a function to each node in the tree in order.
    /// f is mutable and has type `FnMut` because it may modify its parameters
    fn apply<F>(&self, f: &mut F)
    where
        F: FnMut(&Node<T>),
    {
        if let Some(ref l) = self.left {
            l.apply(f);
        }
        f(self);
        if let Some(ref r) = self.right {
            r.apply(f);
        }
    }

    /// Recursively applies a function to each node in the tree in order.
    /// The function may modify `Node`.
    fn apply_mut<F>(&mut self, f: &mut F)
    where
        F: FnMut(&mut Node<T>),
    {
        if let Some(ref mut l) = self.left {
            l.apply_mut(f);
        }
        f(self);
        if let Some(ref mut r) = self.right {
            r.apply_mut(f);
        }
    }

    pub fn advance(&mut self, position: usize, length: usize) {
        let cmp = self.key.start > position;
        if cmp {
            if let Some(ref mut l) = self.left {
                l.advance(position, length);
            }
            self.key.advance(length);
            if let Some(ref mut r) = self.right {
                r.apply_mut(&mut |n| n.key.advance(length));
            }
        } else {
            if self.key.end > position {
                // position is inside this interval
                self.key.end += length;
            }
            if let Some(ref mut l) = self.left {
                l.advance(position, length);
            }
            if let Some(ref mut r) = self.right {
                r.advance(position, length);
            }
        }
    }
}

#[derive(Default)]
/// A interval tree using red-black tree, whereas keys are intervals, and values are
/// plists in elisp.
///
/// All intervals are half-opened intervals, i.e. `I = [start, end)`.These intervals
/// are sorted by their starting point, then their ending point.
///
/// NOTE When inserting an interval I, if I intersects with some existing interval
/// J but I != J, then we split & merge I and J into sub-intervals. Thus all intervals
/// inside a interval tree will not overlap. Adjacant intervals with identical props
/// should be merged afterwards, maybe during redisplay.
pub struct IntervalTree<T: Clone> {
    root: MaybeNode<T>,
}

impl<T: Clone> IntervalTree<T> {
    /// Creates an empty interval tree.
    pub fn new() -> Self {
        Self { root: None }
    }

    /// Inserts a new interval with the specified `key` and `val` into the interval tree.
    ///
    /// If the interval `key` is degenerate (i.e., its start equals its end), the function
    /// returns `None` as such intervals are not allowed in the tree. Otherwise, it delegates
    /// the insertion to the underlying node structure.
    ///
    /// # Arguments
    ///
    /// * `key` - The text range representing the interval to insert.
    /// * `val` - The value associated with the interval.
    /// * `merge` - A closure that specifies how to merge intervals if they overlap
    ///
    /// # Returns
    ///
    /// An optional mutable reference to the newly inserted node, or `None` if the interval is
    /// degenerate.
    pub fn insert<F: Fn(T, T) -> T>(
        &mut self,
        key: impl Into<TextRange>,
        val: T,
        merge_fn: F,
    ) -> Option<&mut Box<Node<T>>> {
        let key = key.into();
        if key.start == key.end {
            return None;
        }
        let mut result =
            Node::insert_at(&mut self.root, key, val, ptr::null_mut(), false, &merge_fn);
        result.as_mut().unwrap().color = Color::Black;
        result
    }

    /// Finds the node with key `key` in the tree and returns its value if found.
    ///
    /// # Arguments
    ///
    /// * `key` - The text range representing the interval to search for.
    ///
    /// # Returns
    ///
    /// An optional value associated with the node if it exists, `None` otherwise.
    pub fn get(&self, key: impl Into<TextRange>) -> Option<T> {
        match self.root {
            Some(ref r) => r.get(key.into()),
            None => None,
        }
    }

    pub fn get_node_mut(&mut self, key: impl Into<TextRange>) -> Option<&mut Node<T>> {
        match self.root {
            Some(ref mut r) => r.get_node_mut(key.into()),
            None => None,
        }
    }

    /// Delete the node with key `key` from the tree. The `key` must excatly match
    /// an interval in the tree.
    ///
    /// If the root node is the only black node, then we have to make it red
    /// before deleting. Otherwise, the tree would become unbalanced.
    ///
    /// After deleting, we make sure the root node is black again.
    pub fn delete_exact(&mut self, key: impl Into<TextRange>) -> MaybeNode<T> {
        let key = key.into();
        let result = match self.root {
            Some(ref mut root) => {
                if !Node::red(&root.left) && !Node::red(&root.right) {
                    root.color = Color::Red;
                }

                Node::delete(&mut self.root, key)
            }
            None => None,
        };
        if let Some(ref mut root) = self.root {
            root.color = Color::Black;
        }
        result
    }

    /// Deletes the node with the minimum key from the interval tree.
    ///
    /// If the root node is the only black node, it is temporarily colored red
    /// to maintain tree balance during deletion. After deletion, the root node
    /// is recolored black to ensure the red-black tree properties are preserved.
    ///
    /// # Returns
    ///
    /// An optional `Node<T>` representing the removed node, or `None` if
    /// the tree is empty.
    pub fn delete_min(&mut self) -> MaybeNode<T> {
        let root = self.root.as_mut()?;
        if !Node::red(&root.left) && !Node::red(&root.right) {
            root.color = Color::Red;
        }
        let result = Node::delete_min(&mut self.root);
        if let Some(ref mut root) = self.root {
            root.color = Color::Black;
        }
        result
    }

    pub fn delete_max(&mut self) -> MaybeNode<T> {
        let root = self.root.as_mut()?;
        if !Node::red(&root.left) && !Node::red(&root.right) {
            root.color = Color::Red;
        }
        let result = Node::delete_max(&mut self.root);
        if let Some(ref mut root) = self.root {
            root.color = Color::Black;
        }
        result
    }

    /// Deletes intervals from the tree that intersect with the given range.
    ///
    /// The behavior depends on the `del_extend` parameter:
    /// - If `true`, deletes all intervals that intersect with the range
    /// - If `false`, only deletes the intersecting portions of intervals, preserving
    ///   non-intersecting parts by splitting them into new intervals
    ///
    /// # Arguments
    ///
    /// * `range` - The range to delete (can be any type that converts to `TextRange`)
    /// * `del_extend` - Whether to delete entire intersecting intervals or just the overlapping portions
    ///
    /// # Examples
    ///
    /// ```
    /// use interval_tree::{IntervalTree, TextRange};
    ///
    /// let mut tree = IntervalTree::new();
    /// tree.insert(TextRange::new(0, 10), 1, |a, _| a);
    ///
    /// // Delete only overlapping portion
    /// tree.delete(TextRange::new(5, 15), false);
    /// assert_eq!(tree.find_intersects(TextRange::new(0, 10)).len(), 1);
    ///
    /// let mut tree = IntervalTree::new();
    /// tree.insert(TextRange::new(0, 10), 1, |a, _| a);
    ///
    /// // Delete entire intersecting interval
    /// tree.delete(TextRange::new(5, 15), true);
    /// assert!(tree.find_intersects(TextRange::new(0, 10)).is_empty());
    /// ```
    pub fn delete(&mut self, range: impl Into<TextRange>, del_extend: bool) {
        let range: TextRange = range.into();
        for key in self.find_intersects(range).iter().map(|n| n.key).collect::<Vec<_>>() {
            if del_extend {
                self.delete_exact(key);
                continue;
            }
            // key right-intersect with range
            // if key is a subset of range, delete it
            if key.start >= range.start && key.end <= range.end {
                self.delete_exact(key);
            }

            // if key is not a subset of range but its start is within range,
            // split it into two parts, and delete the part that is within range
            if key.start < range.start {
                let n = self.get_node_mut(key).unwrap();
                n.key.end = range.start;
                if key.end > range.end {
                    let val = n.val.clone();
                    let f = |_, _| unreachable!(); // f will not be invoked anyway
                    self.insert(TextRange::new(range.start, key.end), val, f);
                }
            }

            // if key is not a subset of range but its end is within range,
            // split it into two parts, and delete the part that is within range
            if key.end > range.end {
                let n = self.get_node_mut(key).unwrap();
                n.key.start = range.end;
            }
            // unreachable!()
        }
    }

    /// Advances all intervals in the tree by `length`, starting at
    /// `position`. This is typically used to implement operations that insert
    /// or delete text in a buffer.
    pub fn advance(&mut self, position: usize, length: usize) {
        if let Some(ref mut node) = self.root {
            node.advance(position, length);
        }
    }

    /// Find the node whose interval contains the given `position`. If no interval
    /// contains the position, returns `None`.
    pub fn find(&self, position: usize) -> Option<&Node<T>> {
        let range = TextRange::new(position, position + 1);
        let res = self.find_intersects(range);
        res.first().copied()
    }

    /// Find all nodes in the tree whose intervals intersect the given
    /// `range`. The result is a vector of references to the found nodes.
    pub fn find_intersects(&self, range: impl Into<TextRange>) -> Vec<&Node<T>> {
        let mut result = Vec::new();
        let range = range.into();
        if range.start == range.end {
            return result;
        }
        if let Some(ref r) = self.root {
            r.find_intersects(range, &mut result);
        }
        result
    }

    /// Finds the node with the minimum key that intersects with the given range.
    ///
    /// This function searches for the leftmost node in the tree whose interval
    /// overlaps with the specified range. It's useful for finding the first
    /// intersecting interval in sorted order.
    ///
    /// # Arguments
    ///
    /// * `range` - The range to search for intersections (can be any type that converts to `TextRange`)
    ///
    /// # Returns
    ///
    /// An optional reference to the node with the minimum intersecting key, or `None`
    /// if no intersection is found.
    ///
    /// # Examples
    ///
    /// ```
    /// use interval_tree::{IntervalTree, TextRange};
    ///
    /// let mut tree = IntervalTree::new();
    /// tree.insert(TextRange::new(0, 5), 1, |a, _| a);
    /// tree.insert(TextRange::new(5, 10), 2, |a, _| a);
    ///
    /// let node = tree.find_intersect_min(TextRange::new(3, 7));
    /// assert_eq!(node.unwrap().key, TextRange::new(0, 5));
    /// ```
    pub fn find_intersect_min(&self, range: impl Into<TextRange>) -> Option<&Node<T>> {
        self.root.as_ref().and_then(|r| r.find_intersect_min(range.into()))
    }

    /// Like `find_intersect_min`, but finds the maximum key.
    pub fn find_intersect_max(&self, range: impl Into<TextRange>) -> Option<&Node<T>> {
        self.root.as_ref().and_then(|r| r.find_intersect_max(range.into()))
    }

    /// Return the minimum node in the tree, or `None` if the tree is empty.
    pub fn min(&self) -> Option<&Node<T>> {
        self.root.as_ref().map(|n| n.min())
    }

    /// Cleans the interval tree by:
    /// 1. Removing empty intervals or intervals with empty values
    /// 2. Merging adjacent intervals with equal values
    ///
    /// This function iterates through the tree in order and:
    /// - Removes any node where the interval is empty or the value is considered empty
    /// - Merges adjacent nodes when their values are considered equal
    ///
    /// # Arguments
    ///
    /// * `eq` - A closure that determines if two values should be considered equal
    /// * `empty` - A closure that determines if a value should be considered empty
    ///
    /// # Examples
    ///
    /// ```
    /// use interval_tree::{IntervalTree, TextRange};
    ///
    /// let mut tree = IntervalTree::new();
    /// tree.insert(TextRange::new(0, 5), 1, |a, _| a);
    /// tree.insert(TextRange::new(5, 10), 1, |a, _| a);
    /// tree.insert(TextRange::new(10, 15), 2, |a, _| a);
    /// tree.insert(TextRange::new(15, 20), 0, |a, _| a); // Empty value
    ///
    /// // Clean the tree, merging equal values and removing empty ones
    /// tree.clean(|a, b| a == b, |v| *v == 0);
    ///
    /// assert_eq!(tree.find_intersects(TextRange::new(0, 20)).len(), 2);
    /// ```
    pub fn clean<F: Fn(&T, &T) -> bool, G: Fn(&T) -> bool>(&mut self, eq: F, empty: G) {
        let min = self.min_mut();
        self.clean_from_node(min, eq, empty);
    }

    pub fn clean_from<F: Fn(&T, &T) -> bool, G: Fn(&T) -> bool>(
        &mut self,
        start: impl Into<TextRange>,
        eq: F,
        empty: G,
    ) {
        let node = self.get_node_mut(start).map(ptr::from_mut);
        self.clean_from_node(node, eq, empty);
    }

    fn clean_from_node<F: Fn(&T, &T) -> bool, G: Fn(&T) -> bool>(
        &mut self,
        start: Option<*mut Node<T>>,
        eq: F,
        empty: G,
    ) {
        let mut node = start;

        while let Some(n_raw) = node {
            let n = safe_mut(n_raw);
            let mut next_raw = n.next_raw();
            // delete empty props at first
            if n.key.empty() || empty(&n.val) {
                self.delete_exact(n.key);
                node = next_raw;
                continue;
            }
            // delete empty props after n, which is already guranteed to be non-empty
            while let Some(next_r) = next_raw {
                let next = safe_mut(next_r);
                if next.key.empty() || empty(&next.val) {
                    let key = next.key;
                    next_raw = next.next_raw();
                    self.delete_exact(key);
                } else {
                    break;
                }
            }

            // try merge n and next. If succeed, find next.next(); if not, advance n to next. Then repeat
            let Some(next_r) = next_raw else { return };
            let next = safe_mut(next_r);

            if n.key.end == next.key.start && eq(&n.val, &next.val) {
                let new_end = next.key.end;
                self.delete_exact(next.key);
                n.key.end = new_end;
                continue;
            }
            node = Some(next_r);
        }
    }

    fn min_mut(&mut self) -> Option<*mut Node<T>> {
        self.root.as_mut().map(|n| ptr::from_mut(n.min_mut()))
    }

    /// Merges adjacent intervals in the tree that have equal properties.
    ///
    /// This function iterates over the nodes in the interval tree, starting from
    /// the minimum node. It checks if the current node's end equals the next node's
    /// start and if their values are considered equal by the provided `equal`
    /// function. If both conditions are met, it merges the intervals by extending
    /// the current node's end to the next node's end and deletes the next node.
    ///
    /// # Arguments
    ///
    /// * `equal` - A closure that takes references to two values and returns `true`
    ///   if they are considered equal, `false` otherwise.
    pub fn merge<F: Fn(&T, &T) -> bool>(&mut self, eq: F) {
        self.clean(eq, |_| false);
    }

    pub fn apply<F: FnMut(&T)>(&self, f: &mut F) {
        if let Some(r) = self.root.as_ref() {
            r.apply(&mut |n: &Node<T>| f(&n.val));
        }
    }

    pub fn apply_mut<F: FnMut(&mut Node<T>)>(&mut self, f: &mut F) {
        if let Some(r) = self.root.as_mut() {
            r.apply_mut(&mut |n| f(n));
        }
    }

    /// Applies a transformation function to intervals that intersect with the given range,
    /// splitting intervals as needed to apply the function only to the intersecting portions.
    ///
    /// The function `f` takes the current value of an interval and returns:
    /// - `Some(new_value)` to update the interval's value
    /// - `None` to remove the interval entirely
    ///
    /// If an interval only partially intersects with the range, it will be split into
    /// non-intersecting and intersecting parts, with `f` only applied to the intersecting part.
    ///
    /// # Arguments
    ///
    /// * `f` - Transformation function to apply to intersecting intervals
    /// * `range` - The range to check for intersections (can be any type that converts to `TextRange`)
    ///
    /// # Examples
    ///
    /// ```
    /// use interval_tree::{IntervalTree, TextRange};
    ///
    /// let mut tree = IntervalTree::new();
    /// tree.insert(TextRange::new(0, 10), 1, |a, _| a);
    ///
    /// // Double values in range 5-15
    /// tree.apply_with_split(|val| Some(val * 2), TextRange::new(5, 15));
    ///
    /// // Remove intervals in range 7-8
    /// tree.apply_with_split(|_| None, TextRange::new(7, 8));
    /// ```
    pub fn apply_with_split<F: Fn(T) -> Option<T>>(&mut self, f: F, range: impl Into<TextRange>) {
        let merge = |_, _| unreachable!();

        let range = range.into();
        let Some(start) = self.find_intersect_min(range) else {
            return;
        };
        // re-gain mutable reference, since we don't have interior mutability for now
        let mut node_ptr = self.get_node_mut(start.key).map(ptr::from_mut);
        while let Some(n_ptr) = node_ptr {
            let n = safe_mut(n_ptr);
            node_ptr = n.next_raw();
            if let Some(overlap) = n.key.intersection(range) {
                if let Some(left) = TextRange::new_valid(n.key.start, overlap.start) {
                    n.key.start = left.end;
                    self.insert(left, n.val.clone(), merge);
                }

                if let Some(right) = TextRange::new_valid(overlap.end, n.key.end) {
                    n.key.end = right.start;
                    self.insert(right, n.val.clone(), merge);
                }

                let new_val = f(n.val.clone());
                match new_val {
                    Some(val) => n.val = val,
                    None => {
                        self.delete_exact(n.key);
                    }
                }
            }
        }
    }
}

// impl debug

impl<T: Clone + Debug> Node<T> {
    fn print_inner(&self, f: &mut std::fmt::Formatter, level: usize) -> std::fmt::Result {
        write_fmt_with_level(
            f,
            level,
            format_args!("[key: {:?}, val: {:?}, color: {:?}]\n", self.key, self.val, self.color),
        )?;
        if let Some(parent) = unsafe { self.parent.as_ref() } {
            let direction = if self.is_right_child { "right" } else { "left" };
            write_fmt_with_level(
                f,
                level,
                format_args!("parent({} child): {:?}", direction, parent.key),
            )?;
        } else {
            write_fmt_with_level(f, level, format_args!("parent: not found"))?;
        }
        f.write_char('\n')?;
        if let Some(ref l) = self.left {
            write_fmt_with_level(f, level, format_args!("left: \n"))?;
            l.print_inner(f, level + 1)?;
            write_fmt_with_level(f, level, format_args!("left end for {:?}\n", self.key))?;
        } else {
            write_fmt_with_level(f, level, format_args!("no left child \n"))?;
        }
        if let Some(ref r) = self.right {
            write_fmt_with_level(f, level, format_args!("right: \n"))?;
            r.print_inner(f, level + 1)?;
            write_fmt_with_level(f, level, format_args!("right end for {:?}\n", self.key))?;
        } else {
            write_fmt_with_level(f, level, format_args!("no right child \n"))?;
        }
        Ok(())
    }
}

impl<T: Clone + Debug> IntervalTree<T> {
    /// Recursively print out the tree, for debugging purposes. The output format
    /// is not guaranteed to be stable.
    pub fn print(&self) {
        println!("{self:?}");
    }
}

fn write_fmt_with_level(
    f: &mut std::fmt::Formatter,
    level: usize,
    fmt: Arguments<'_>,
) -> std::fmt::Result {
    for _ in 0..level {
        f.write_char('\t')?;
    }
    f.write_fmt(fmt)
}

impl<T: Clone + Debug> Debug for Node<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Node:\n")?;
        self.print_inner(f, 0)
    }
}

impl<T: Clone + Debug> Debug for IntervalTree<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Interval Tree:\n")?;
        if let Some(root) = self.root.as_ref() {
            root.print_inner(f, 0)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn merge<T: Clone + Debug>(a: T, _b: T) -> T {
        a
    }

    fn build_tree<T: Clone + Debug>(val: T) -> IntervalTree<T> {
        let mut tree = IntervalTree::new();
        tree.insert(TextRange::new(0, 1), val.clone(), merge);
        tree.insert(TextRange::new(1, 2), val.clone(), merge);
        tree.insert(TextRange::new(2, 3), val.clone(), merge);
        tree.insert(TextRange::new(3, 4), val.clone(), merge);
        tree.insert(TextRange::new(4, 5), val.clone(), merge);
        tree.insert(TextRange::new(5, 6), val.clone(), merge);
        tree.insert(TextRange::new(6, 7), val.clone(), merge);
        tree.insert(TextRange::new(7, 8), val.clone(), merge);
        tree.insert(TextRange::new(8, 9), val.clone(), merge);
        tree.insert(TextRange::new(9, 10), val.clone(), merge);
        tree.print();
        tree
    }

    #[test]
    fn rotate_left() {
        let val = 1;
        let mut node1 = Node::new_boxed(TextRange::new(0, 3), val);
        node1.color = Color::Black;
        let mut node2 = Node::new_boxed(TextRange::new(3, 6), val);
        let mut node3 = Node::new_boxed(TextRange::new(6, 9), val);
        node3.color = Color::Black;
        let mut node4 = Node::new_boxed(TextRange::new(9, 12), val);
        node4.color = Color::Black;
        let mut node5 = Node::new_boxed(TextRange::new(12, 15), val);
        node5.color = Color::Black;

        node2.left = Some(node3);
        node2.right = Some(node4);
        node2.color = Color::Red;
        node1.right = Some(node2);
        node1.left = Some(node5);
        Node::rotate_left(&mut node1);
        assert_eq!(node1.key.start, 3);
        let n2 = node1.left.unwrap();
        assert_eq!(n2.key.start, 0);
        let n3 = n2.right.unwrap();
        assert_eq!(n3.key.start, 6);
        let n4 = node1.right.unwrap();
        assert_eq!(n4.key.start, 9);
        let n5 = n2.left.unwrap();
        assert_eq!(n5.key.start, 12);

        assert_eq!(node1.color, Color::Black);
        assert_eq!(n2.color, Color::Red);
        assert_eq!(n2.n, 3);
    }

    #[test]
    fn insert() {
        let val = 1;
        let tree = build_tree(val);
        let root = tree.root.as_ref().unwrap();
        assert_eq!(root.key.start, 3);
        let n1 = root.left.as_ref().unwrap();
        assert_eq!(n1.key.start, 1);
        let n2 = root.right.as_ref().unwrap();
        assert_eq!(n2.key.start, 7);
        let n3 = n2.right.as_ref().unwrap();
        assert_eq!(n3.key.start, 9);
        let n4 = n3.left.as_ref().unwrap();
        assert_eq!(n4.key.start, 8);
        assert!(n3.right.is_none());
    }

    #[test]
    fn delete() {
        let val = 1;
        let mut tree = build_tree(val);
        // let mut tree = dbg!(tree);
        for k in [8, 4, 5, 7, 3, 6] {
            let i = TextRange::new(k, k + 1);
            let a = tree.delete_exact(i).unwrap();
            assert_eq!(a.key, i);
        }
    }

    #[test]
    fn delete_ptr() {
        let val = 1;
        let mut tree = build_tree(val);
        let a = 3;
        let key = TextRange::new(a, a + 1);
        let n: *mut Node<i32> = tree.get_node_mut(key).unwrap();
        let mut a = tree.delete_exact(key).unwrap();
        assert_eq!(n, ptr::from_mut(&mut *a));
    }

    #[test]
    fn delete_min() {
        let val = 1;
        let mut tree = build_tree(val);
        // let mut tree = dbg!(tree);
        let a = tree.delete_min().unwrap();
        assert_eq!(a.key.start, 0);
    }

    #[test]
    fn test_find_intersects() {
        let mut tree = IntervalTree::new();
        tree.insert(TextRange::new(0, 5), 1, merge);
        tree.insert(TextRange::new(5, 10), 2, merge);
        tree.insert(TextRange::new(10, 15), 3, merge);
        tree.insert(TextRange::new(15, 20), 4, merge);

        // Test exact match
        let results = tree.find_intersects(TextRange::new(5, 10));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, TextRange::new(5, 10));
        assert_eq!(results[0].val, 2);

        // Test partial overlap at start
        let results = tree.find_intersects(TextRange::new(3, 7));
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].key, TextRange::new(0, 5));
        assert_eq!(results[1].key, TextRange::new(5, 10));

        // Test partial overlap at end
        let results = tree.find_intersects(TextRange::new(12, 18));
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].key, TextRange::new(10, 15));
        assert_eq!(results[1].key, TextRange::new(15, 20));

        // Test range that spans multiple intervals
        let results = tree.find_intersects(TextRange::new(8, 17));
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].key, TextRange::new(5, 10));
        assert_eq!(results[1].key, TextRange::new(10, 15));
        assert_eq!(results[2].key, TextRange::new(15, 20));

        // Test range that is completely contained within an interval
        let results = tree.find_intersects(TextRange::new(6, 8));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, TextRange::new(5, 10));

        // Test range that doesn't intersect any intervals
        let results = tree.find_intersects(TextRange::new(25, 30));
        assert!(results.is_empty());

        // Test empty range
        let results = tree.find_intersects(TextRange::new(3, 3));
        assert!(results.is_empty());

        // Test range that starts before first interval and ends after last
        let results = tree.find_intersects(TextRange::new(0, 25));
        assert_eq!(results.len(), 4);

        // Test single point intersection
        let results = tree.find_intersects(TextRange::new(10, 11));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, TextRange::new(10, 15));
    }

    #[test]
    fn advance() {
        let val = 1;
        let mut tree = build_tree(val);
        tree.advance(7, 5);
        // let mut tree = dbg!(tree);
        tree.get(TextRange::new(6, 7)).unwrap();
        tree.get(TextRange::new(7, 13)).unwrap();
        tree.get(TextRange::new(13, 14)).unwrap();
        tree.get(TextRange::new(14, 15)).unwrap();
    }

    #[test]
    fn find_next() {
        let val = 1;
        let mut tree = build_tree(val);
        tree.delete_exact(TextRange::new(5, 6));
        let mut n = tree.min().unwrap();

        while let Some(ne) = n.next() {
            n = ne;
        }
        assert_eq!(n.key.start, 9);
    }

    #[test]
    fn test_merge_adjacent_intervals_with_equal_properties() {
        let mut tree = IntervalTree::new();
        tree.insert(TextRange::new(1, 5), 1, merge);
        tree.insert(TextRange::new(5, 10), 1, merge);
        tree.merge(|a, b| *a == *b);
        assert_eq!(tree.find_intersects(TextRange::new(1, 10)).len(), 1);
    }

    #[test]
    fn test_not_merge_adjacent_intervals_with_different_properties() {
        let mut tree = IntervalTree::new();
        tree.insert(TextRange::new(1, 5), 1, merge);
        tree.insert(TextRange::new(5, 10), 2, merge);
        tree.merge(|a, b| *a == *b);
        assert_eq!(tree.find_intersects(TextRange::new(1, 10)).len(), 2);
    }

    #[test]
    fn test_not_merge_non_adjacent_intervals() {
        let mut tree = IntervalTree::new();
        tree.insert(TextRange::new(1, 5), 1, merge);
        tree.insert(TextRange::new(10, 15), 1, merge);
        tree.merge(|a, b| *a == *b);
        assert_eq!(tree.find_intersects(TextRange::new(1, 15)).len(), 2);
    }

    #[test]
    fn test_merge_multiple_adjacent_intervals_with_equal_properties() {
        let mut tree = IntervalTree::new();
        tree.insert(TextRange::new(5, 10), 1, merge);
        tree.insert(TextRange::new(1, 5), 1, merge);
        tree.insert(TextRange::new(10, 15), 1, merge);
        tree.merge(|a, b| *a == *b);
        assert_eq!(tree.find_intersects(TextRange::new(1, 15)).len(), 1);
    }

    #[test]
    fn test_handle_empty_tree() {
        let mut tree: IntervalTree<i32> = IntervalTree::new();
        tree.merge(|a, b| *a == *b);
        assert_eq!(tree.find_intersects(TextRange::new(1, 10)).len(), 0);
    }

    #[test]
    fn test_handle_tree_with_single_node() {
        let mut tree = IntervalTree::new();
        tree.insert(TextRange::new(1, 5), 1, merge);
        tree.merge(|a, b| *a == *b);
        assert_eq!(tree.find_intersects(TextRange::new(1, 5)).len(), 1);
    }

    #[test]
    fn test_apply_with_split() {
        let mut tree = IntervalTree::new();
        tree.insert(TextRange::new(0, 10), 1, merge);

        // Apply function to partial overlap
        tree.apply_with_split(|val| Some(val * 2), TextRange::new(5, 15));
        let nodes = tree.find_intersects(TextRange::new(0, 15));
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].val, 1);
        assert_eq!(nodes[1].val, 2);

        // Remove an interval
        tree.apply_with_split(|_| None, TextRange::new(7, 8));
        let nodes = tree.find_intersects(TextRange::new(0, 15));
        assert_eq!(nodes.len(), 3);
        assert_eq!(nodes[1].key, TextRange::new(5, 7));
        assert_eq!(nodes[1].val, 2); // The removed interval should be back to original value

        // Apply to exact match
        tree.apply_with_split(|val| Some(val + 1), TextRange::new(2, 4));
        let node = tree.find_intersects(TextRange::new(2, 4))[0];
        assert_eq!(node.val, 2);
    }

    #[test]
    fn test_clean() {
        let mut tree = IntervalTree::new();
        tree.insert(TextRange::new(0, 5), 1, merge);
        tree.insert(TextRange::new(5, 10), 1, merge);
        tree.insert(TextRange::new(10, 15), 2, merge);
        tree.insert(TextRange::new(15, 20), 0, merge); // Empty value
        tree.insert(TextRange::new(20, 25), 2, merge);
        tree.insert(TextRange::new(25, 30), 2, merge);

        // Clean the tree, merging equal values and removing empty ones
        tree.clean(|a, b| a == b, |v| *v == 0);

        let nodes = tree.find_intersects(TextRange::new(0, 30));
        assert_eq!(nodes.len(), 3);
        assert_eq!(nodes[0].key, TextRange::new(0, 10));
        assert_eq!(nodes[2].key, TextRange::new(20, 30));
    }

    #[test]
    fn test_clean_empty_tree() {
        let mut tree: IntervalTree<i32> = IntervalTree::new();
        tree.clean(|a, b| a == b, |v| *v == 0);
        assert!(tree.find_intersects(TextRange::new(0, 1)).is_empty());
    }

    #[test]
    fn test_clean_with_empty_intervals() {
        let mut tree = IntervalTree::new();
        tree.insert(TextRange::new(0, 0), 1, merge); // Empty interval
        tree.insert(TextRange::new(0, 5), 1, merge);
        tree.insert(TextRange::new(5, 5), 2, merge); // Empty interval

        tree.clean(|a, b| a == b, |v| *v == 0);

        assert_eq!(tree.find_intersects(TextRange::new(0, 5)).len(), 1);
    }

    #[test]
    fn test_find_intersect_min() {
        let mut tree = IntervalTree::new();
        tree.insert(TextRange::new(0, 5), 1, merge);
        tree.insert(TextRange::new(5, 10), 2, merge);
        tree.insert(TextRange::new(14, 18), 3, merge);

        // Test exact match
        assert_eq!(
            tree.find_intersect_min(TextRange::new(5, 10)).unwrap().key,
            TextRange::new(5, 10)
        );

        // Test partial overlap
        assert_eq!(
            tree.find_intersect_min(TextRange::new(3, 7)).unwrap().key,
            TextRange::new(0, 5)
        );

        assert_eq!(
            tree.find_intersect_min(TextRange::new(6, 15)).unwrap().key,
            TextRange::new(5, 10)
        );

        // Test no overlap
        assert!(tree.find_intersect_min(TextRange::new(19, 23)).is_none());

        // Test empty tree
        let empty_tree: IntervalTree<i32> = IntervalTree::new();
        assert!(empty_tree.find_intersect_min(TextRange::new(0, 1)).is_none());
    }
}
