use crate::iter::{
    IterAll, IterRangeWith, LeftBiasIter, LeftBiasIterWidth, NodeRightIter, NodeWidth,
    SkipListRange,
};
use rand;
use rand::prelude::*;
use std::cmp::{Ordering, PartialOrd};
use std::fmt;
use std::iter::FromIterator;
use std::ptr::NonNull;
pub mod iter;

#[cfg(feature = "serde_support")]
mod serde;

#[derive(PartialEq, Debug)]
enum NodeValue<T> {
    NegInf,
    Value(T),
    PosInf,
}

impl<T> NodeValue<T> {
    #[inline]
    fn get_value(&self) -> &T {
        match &self {
            NodeValue::Value(v) => v,
            _ => unreachable!(),
        }
    }
    #[inline]
    fn is_pos_inf(&self) -> bool {
        match &self {
            NodeValue::PosInf => true,
            _ => false,
        }
    }
}

impl<T: PartialEq> PartialEq<T> for NodeValue<T> {
    #[inline]
    fn eq(&self, other: &T) -> bool {
        match self {
            NodeValue::Value(v) => v == other,
            _ => false,
        }
    }
}

impl<T: PartialOrd> PartialOrd<NodeValue<T>> for NodeValue<T> {
    #[inline]
    fn partial_cmp(&self, other: &NodeValue<T>) -> Option<Ordering> {
        match (self, other) {
            (NodeValue::NegInf, _) => Some(Ordering::Less),
            (_, NodeValue::PosInf) => Some(Ordering::Less),
            (NodeValue::Value(l), NodeValue::Value(r)) => l.partial_cmp(r),
            _ => unreachable!(),
        }
    }
}

impl<T: PartialOrd> PartialOrd<T> for NodeValue<T> {
    #[inline]
    fn partial_cmp(&self, other: &T) -> Option<Ordering> {
        match self {
            NodeValue::NegInf => Some(Ordering::Less),
            NodeValue::PosInf => Some(Ordering::Greater),
            NodeValue::Value(v) => v.partial_cmp(other),
        }
    }
}

struct Node<T> {
    right: Option<NonNull<Node<T>>>,
    down: Option<NonNull<Node<T>>>,
    value: NodeValue<T>,
    width: usize,
}

impl<T> Node<T> {
    #[inline]
    fn nodes_skipped_over(&self) -> usize {
        self.width - 1
    }

    #[inline]
    fn clear_right(&mut self) {
        self.width = 1;
        unsafe {
            while let Some(right) = self.right {
                if right.as_ref().value.is_pos_inf() {
                    break;
                }
                let garbage = std::mem::replace(&mut self.right, (*right.as_ptr()).right);
                drop(Box::from_raw(garbage.unwrap().as_ptr()));
            }
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Node<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Node(")?;
        writeln!(
            f,
            "  right: {:?},",
            self.right
                .map(|some| format!("{:?}", unsafe { &some.as_ref().value }))
        )?;
        writeln!(
            f,
            "  down: {:?},",
            self.down
                .map(|some| format!("{:?}", unsafe { &some.as_ref().value }))
        )?;
        writeln!(f, "  value: {:?}", self.value)?;
        writeln!(f, "  width: {:?}", self.width)?;
        write!(f, ")")
    }
}

/// Hint that the current value `item` is:
///
/// - SmallerThanRange: `item` is strictly smaller than the range.
/// - InRange: `item` is in the range.
/// - LargerThanRange: `item` is strictly larger than the range.
///
/// Used with IterRangeWith, or `range_with`
#[derive(Debug)]
pub enum RangeHint {
    SmallerThanRange,
    InRange,
    LargerThanRange,
}

/// `SkipLists` are fast probabilistic data-structures that feature logarithmic time complexity for inserting elements,
/// testing element association, removing elements, and finding ranges of elements.
///
/// ```rust
/// use convenient_skiplist::SkipList;
///
/// // Make a new skiplist
/// let mut sk = SkipList::new();
/// for i in 0..5usize {
///     // Inserts are O(log(n)) on average
///     sk.insert(i);
/// }
/// // You can print the skiplist!
/// dbg!(&sk);
/// // You can check if the skiplist contains an element, O(log(n))
/// assert!(sk.contains(&0));
/// assert!(!sk.contains(&10));
/// assert!(sk.remove(&0)); // remove is also O(log(n))
/// assert!(sk == sk); // equality checking is O(n)
/// let from_vec = SkipList::from(vec![1usize, 2, 3].into_iter()); // From<Vec<T>> is O(nlogn)
/// assert_eq!(vec![1, 2, 3], from_vec.iter_all().cloned().collect::<Vec<usize>>());
/// ```
pub struct SkipList<T> {
    top_left: NonNull<Node<T>>,
    height: usize,
    len: usize,
    _prevent_sync_send: std::marker::PhantomData<*const ()>,
}

impl<T> Drop for SkipList<T> {
    fn drop(&mut self) {
        // Main idea: Start in top left and iterate row by row.
        let mut curr_left_node = self.top_left.as_ptr();
        let mut next_down;
        let mut curr_node = self.top_left.as_ptr();
        unsafe {
            loop {
                if let Some(down) = (*curr_left_node).down {
                    next_down = Some(down.as_ptr());
                } else {
                    next_down = None;
                }
                while let Some(right) = (*curr_node).right {
                    let garbage = std::mem::replace(&mut curr_node, right.as_ptr());
                    drop(Box::from_raw(garbage));
                }
                drop(Box::from_raw(curr_node));
                if let Some(next_down) = next_down {
                    curr_left_node = next_down;
                    curr_node = curr_left_node;
                } else {
                    break;
                }
            }
        }
    }
}

impl<T: PartialOrd + Clone> FromIterator<T> for SkipList<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> SkipList<T> {
        let mut sk = SkipList::new();
        for item in iter {
            sk.insert(item);
        }
        sk
    }
}

impl<T: PartialOrd + Clone, I: Iterator<Item = T>> From<I> for SkipList<T> {
    fn from(iter: I) -> Self {
        iter.collect()
    }
}

impl<T: PartialOrd + Clone> PartialEq for SkipList<T> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter_all().zip(other.iter_all()).all(|(l, r)| l == r)
    }
}

macro_rules! fmt_node {
    ($f:expr, $node:expr) => {
        write!(
            $f,
            "{:?}(skipped: {})",
            $node.as_ref().value,
            $node.as_ref().nodes_skipped_over()
        )
    };
}

impl<T: fmt::Debug> fmt::Debug for SkipList<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "SkipList(wall_height: {}), and table:", self.height)?;
        unsafe {
            fmt_node!(f, self.top_left)?;
            write!(f, " -> ")?;
            fmt_node!(f, self.top_left.as_ref().right.unwrap())?;
            writeln!(f)?;
            let mut curr_down = self.top_left.as_ref().down;
            while let Some(down) = curr_down {
                fmt_node!(f, down)?;
                let mut curr_right = down.as_ref().right;
                while let Some(right) = curr_right {
                    write!(f, " -> ")?;
                    fmt_node!(f, right)?;
                    curr_right = right.as_ref().right;
                }
                curr_down = down.as_ref().down;
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

impl<T: PartialOrd + Clone> Default for SkipList<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Get the level of an item in the skiplist
#[inline]
fn get_level() -> usize {
    let mut height = 1;
    let mut rng = rand::thread_rng();
    while rng.gen::<f32>() >= 0.5 {
        height += 1;
    }
    height
}

impl<T: PartialOrd + Clone> SkipList<T> {
    /// Make a new, empty SkipList. By default there is three levels.
    ///
    /// # Example
    ///
    /// ```rust
    /// use convenient_skiplist::SkipList;
    /// let mut sk = SkipList::new();
    /// sk.insert(0usize);
    ///
    /// assert!(sk.contains(&0));
    /// ```
    #[inline]
    pub fn new() -> SkipList<T> {
        let mut sk = SkipList {
            top_left: SkipList::pos_neg_pair(1),
            height: 1,
            len: 0,
            _prevent_sync_send: std::marker::PhantomData,
        };
        sk.add_levels(2);
        sk
    }

    /// add `additional_levels` to the _top_ of the SkipList
    #[inline]
    fn add_levels(&mut self, additional_levels: usize) {
        let mut curr_level = self.top_left;
        for _ in 0..additional_levels {
            let mut new_level = SkipList::pos_neg_pair(self.len() + 1);
            // We're going to insert this `new_level` between curr_level and the row below it.
            // So it will look like:
            // | top_left -> top_right
            // | *new row here*
            // | *existing row*
            unsafe {
                new_level.as_mut().down = curr_level.as_ref().down;
                curr_level.as_mut().down = Some(new_level);
                curr_level = new_level;
            }
        }
        self.height += additional_levels as usize;
    }
    /// Insert `item` into the `SkipList`.
    ///
    /// Returns `true` if the item was actually inserted (i.e. wasn't already in the skiplist)
    /// and `false` otherwise. Runs in `O(logn)` time.
    ///
    /// # Arguments
    ///
    /// * `item` - the item to insert.
    ///
    /// # Example
    ///
    /// ```rust
    /// use convenient_skiplist::SkipList;
    /// let mut sk = SkipList::new();
    /// sk.insert(0usize);
    ///
    /// assert!(sk.contains(&0));
    /// ```
    #[inline]
    pub fn insert(&mut self, item: T) -> bool {
        #[cfg(debug_assertions)]
        {
            self.ensure_invariants()
        }

        if self.contains(&item) {
            return false;
        }
        let height = get_level();
        let additional_height_req: i32 = (height as i32 - self.height as i32) + 1;
        if additional_height_req > 0 {
            self.add_levels(additional_height_req as usize);
            debug_assert!(self.height > height);
        }
        #[cfg(debug_assertions)]
        {
            self.ensure_invariants()
        }

        // Now the skiplist has enough height to actually insert this element.
        // We'll need to reverse iterate to stitch the required items between.
        // As self.path_to returns all nodes immediately *left* of where we've inserted,
        // we just need to insert the nodes after.
        let mut node_below_me = None;
        let mut added = 0;
        let mut total_width = None;
        for node in self.insert_path(&item).into_iter().rev() {
            unsafe {
                (*node.curr_node).width += 1;
            }
            // Set total_width from the bottom node.
            if total_width.is_none() {
                total_width = Some(node.curr_width);
            }
            let total_width = total_width.unwrap();
            if added < height {
                unsafe {
                    // IDEA: We are iterating every node immediately *left* of where we're inserting
                    // an element. This means we can use `total_width`, or the maximum distance
                    // traveled to the right to reach the node to determine node widths relatively.
                    //
                    // eg. We insert 4 into the skiplist below:
                    // -inf ->                ...
                    // -inf -> 1 ->           ...
                    // -inf -> 1 -> 2 ->      ...
                    // -inf -> 1 -> 2 -> 3 -> ...
                    //
                    // Imagine a placeholder where 4 goes.
                    //
                    // eg. We insert 4 into the skiplist below:
                    // -inf ->                _ -> ...
                    // -inf -> 1 ->           _ -> ...
                    // -inf -> 1 -> 2 ->      _ -> ...
                    // -inf -> 1 -> 2 -> 3 -> _ -> ...
                    //
                    // This placeholder has then increased the width of all nodes by 1.
                    // Once we determine height, for every element on the left,
                    // we need to distribute the widths. We can do this
                    // relative to `total_width`:
                    //
                    // 1. -inf ->                _ -> ...
                    // 2. -inf -> 1 ->           _ -> ...
                    // 3. -inf -> 1 -> 2 ->      _ -> ...
                    // 4. -inf -> 1 -> 2 -> 3 -> _ -> ...
                    //          ~    ~    ~    ~
                    // We know how far _right_ we've been, and know that
                    // all areas a '4' goes is going to truncate widths
                    // of the elements to the left. For example,
                    // row element '2' in row 3 is going to report a `node.curr_width`
                    // of 3, so it's new width is (4 - 3) + 1 (i.e. the number of links between it and 4)
                    //
                    // Lastly, we distribute the remaining width after the
                    // truncation above to the new element.

                    let left_node_width = total_width - node.curr_width + 1;
                    let new_node_width = (*node.curr_node).width - left_node_width;

                    (*node.curr_node).width = left_node_width;

                    debug_assert!(total_width + 1 == node.curr_width + left_node_width);

                    let mut new_node = SkipList::make_node(item.clone(), new_node_width);

                    let node: *mut Node<T> = node.curr_node;
                    new_node.as_mut().down = node_below_me;
                    new_node.as_mut().right = (*node).right;
                    (*node).right = Some(new_node);
                    node_below_me = Some(new_node);
                }
                added += 1;
            }
        }
        self.len += 1;
        #[cfg(debug_assertions)]
        {
            self.ensure_invariants()
        }
        true
    }
    /// Test if `item` is in the skiplist. Returns `true` if it's in the skiplist,
    /// `false` otherwise.
    ///
    /// Runs in `O(logn)` time
    ///
    /// # Arguments
    ///
    /// * `item` - the item we're testing.
    ///
    /// # Example
    ///
    /// ```rust
    /// use convenient_skiplist::SkipList;
    /// let mut sk = SkipList::new();
    /// sk.insert(0usize);
    ///
    /// assert!(sk.contains(&0));
    /// ```
    #[inline]
    pub fn contains(&self, item: &T) -> bool {
        self.iter_left(item).any(|node| unsafe {
            if let Some(right) = &(*node).right {
                &right.as_ref().value == item
            } else {
                false
            }
        })
    }

    /// Remove `item` from the SkipList.
    ///
    /// Returns `true` if the item was in the collection to be removed,
    /// and `false` otherwise. Runs in `O(logn)` time.
    ///
    /// # Arguments
    ///
    /// * `item` - the item to remove.
    ///
    /// # Example
    ///
    /// ```rust
    /// use convenient_skiplist::SkipList;
    /// let mut sk = SkipList::new();
    /// sk.insert(0usize);
    ///
    /// let removed = sk.remove(&0);
    /// assert!(removed);
    /// ```
    pub fn remove(&mut self, item: &T) -> bool {
        if !self.contains(item) {
            return false;
        }
        for node in self.iter_left(item) {
            unsafe {
                (*node).width -= 1;
                // Invariant: `node` can never be PosInf
                let right = (*node).right.unwrap();
                if &right.as_ref().value != item {
                    continue;
                }
                // So the node right of us needs to be removed.
                (*node).width += right.as_ref().width;
                let garbage = std::mem::replace(&mut (*node).right, right.as_ref().right);
                drop(Box::from_raw(garbage.unwrap().as_ptr()));
            }
        }
        self.len -= 1;
        true
    }

    /// Return the number of elements in the skiplist.
    ///
    /// # Example
    /// ```rust
    /// use convenient_skiplist::SkipList;
    /// let mut sk = SkipList::new();
    ///
    /// sk.insert(0);
    /// assert_eq!(sk.len(), 1);
    ///
    /// sk.insert(1);
    /// assert_eq!(sk.len(), 2);
    /// ```

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns true if the skiplist is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    // TODO
    // fn remove_range<'a>(&'a mut self, _start: &'a T, _end: &'a T) -> usize {
    //     // Idea: Use iter_left twice to determine the chunk in the middle to remove.
    //     // Hardest part will be cleaning up garbage. :thinking:
    //     todo!()
    // }

    /// Find the index of `item` in the `SkipList`. Runs in `O(logn)` time.
    ///
    /// # Arguments
    ///
    /// * `item`: the item to find the position of.
    ///
    /// # Example
    /// ```rust
    /// use convenient_skiplist::SkipList;
    /// let mut sk = SkipList::new();
    /// sk.insert(1);
    /// sk.insert(2);
    /// sk.insert(3);
    ///
    /// assert_eq!(sk.index_of(&1), Some(0));
    /// assert_eq!(sk.index_of(&2), Some(1));
    /// assert_eq!(sk.index_of(&3), Some(2));
    /// assert_eq!(sk.index_of(&999), None);
    /// ```
    #[inline]
    pub fn index_of(&self, item: &T) -> Option<usize> {
        // INVARIANT: path_to is a LeftBiasIterWidth, so there's always a
        // node right of us.
        self.path_to(item).last().and_then(|node| {
            if unsafe { &(*node.curr_node).right.unwrap().as_ref().value } == item {
                Some(node.curr_width)
            } else {
                None
            }
        })
    }

    /// Get the item at the index `index `in the `SkipList`. Runs in `O(n)` time.
    ///
    /// # Arguments
    ///
    /// * `index`: the index to get the item at
    ///
    /// # Example
    /// ```rust
    /// use convenient_skiplist::SkipList;
    /// let sk = SkipList::from(0..10);
    /// for i in 0..10 {
    ///     assert_eq!(Some(&i), sk.at_index(i));
    /// }
    /// assert_eq!(None, sk.at_index(11));
    ///
    /// let mut sk = SkipList::new();
    /// sk.insert('a');
    /// sk.insert('b');
    /// sk.insert('c');
    /// assert_eq!(Some(&'a'), sk.at_index(0));
    /// assert_eq!(Some(&'b'), sk.at_index(1));
    /// assert_eq!(Some(&'c'), sk.at_index(2));
    /// assert_eq!(None, sk.at_index(3));
    /// ```
    pub fn at_index(&self, index: usize) -> Option<&T> {
        if index >= self.len() {
            return None;
        }
        unsafe {
            let mut curr_node = self.top_left.as_ref();
            let mut distance_left = index + 1;
            loop {
                if distance_left == 0 {
                    return Some(curr_node.value.get_value());
                }
                if curr_node.width <= distance_left {
                    distance_left -= curr_node.width;
                    // INVARIANT: We've checked if `index` < self.len(),
                    // so there's always a `right`
                    curr_node = curr_node.right.unwrap().as_ptr().as_ref().unwrap();
                    continue;
                } else if let Some(down) = curr_node.down {
                    curr_node = down.as_ptr().as_ref().unwrap();
                } else {
                    unreachable!()
                }
            }
        }
    }

    /// Pop `count` elements off of the end of the Skiplist.
    /// Runs in O(logn + count) time, O(logn + count) space.
    ///
    /// Returns an empty `vec` if count == 0.
    /// Will dealloc the whole skiplist if count > usize and start fresh.
    ///
    /// # Example
    ///
    /// ```rust
    /// use convenient_skiplist::SkipList;
    /// let mut sk = SkipList::from(0..10);
    ///
    /// assert_eq!(Some(&7), sk.at_index(7));
    /// assert_eq!(vec![7, 8, 9], sk.pop_max(3));
    /// assert_eq!(vec![6], sk.pop_max(1));
    /// assert_eq!(vec![4, 5], sk.pop_max(2));
    /// assert_eq!(vec![0, 1, 2, 3], sk.pop_max(5));
    ///
    /// let v: Vec<u32> = Vec::new();
    /// assert_eq!(v, sk.pop_max(1000)); // empty
    /// ```
    #[inline]
    pub fn pop_max(&mut self, count: usize) -> Vec<T> {
        if self.is_empty() || count == 0 {
            return vec![];
        }
        if count >= self.len() {
            // let new = SkipList::new();
            // let garbage = std::mem::replace(&mut self, &mut new);
            // drop(garbage);
            let ret = self.iter_all().cloned().collect();
            *self = SkipList::new(); // TODO: Does this drop me?
            return ret;
        }
        let ele_at = self.at_index(self.len() - count).unwrap().clone();
        self.len = self.len - count;
        // IDEA: Calculate widths by adding _backwards_ through the
        // insert path.
        let mut frontier = self.insert_path(&ele_at);
        let last_value = frontier.last_mut().cloned().unwrap();
        let mut last_width = last_value.curr_width;
        let mut ret: Vec<_> = Vec::with_capacity(count);
        let mut jumped_left = 1;
        unsafe {
            ret.extend(NodeRightIter::new(
                (*last_value.curr_node).right.unwrap().as_ptr(),
            ));
            (*last_value.curr_node).clear_right();
        }
        for mut nw in frontier.into_iter().rev().skip(1) {
            unsafe {
                // We've jumped right, and now need to update our width field.
                // Do we need this if-gate?
                if (*nw.curr_node).value != (*last_value.curr_node).value {
                    jumped_left += last_width - nw.curr_width;
                    last_width = nw.curr_width;
                }
                (*nw.curr_node).clear_right();
                (*nw.curr_node).width = jumped_left;
            }
        }
        ret
    }

    /// Left-Biased iterator towards `item`.
    ///
    /// Returns all possible positions *left* where `item`
    /// is or should be in the skiplist.
    #[inline]
    fn iter_left<'a>(&'a self, item: &'a T) -> impl Iterator<Item = *mut Node<T>> + 'a {
        LeftBiasIter::new(self.top_left.as_ptr(), item)
    }

    /// Iterator over all elements in the Skiplist.
    ///
    /// This runs in `O(logn)` time.
    ///
    /// # Example
    ///
    /// ```rust
    /// use convenient_skiplist::SkipList;
    /// let mut sk = SkipList::new();
    /// sk.insert(0usize);
    /// sk.insert(1usize);
    /// sk.insert(2usize);
    /// for item in sk.iter_all() {
    ///     println!("{:?}", item);
    /// }
    /// ```
    #[inline]
    pub fn iter_all(&self) -> IterAll<T> {
        unsafe { IterAll::new(self.top_left.as_ref(), self.len) }
    }

    /// Iterator over an inclusive range of elements in the SkipList.
    ///
    /// This runs in `O(logn + k)`, where k is the width of range.
    ///
    /// # Example
    ///
    /// ```rust
    /// use convenient_skiplist::SkipList;
    /// let mut sk = SkipList::new();
    /// for item in 0..100 {
    ///     sk.insert(item);
    /// }
    ///
    /// for item in sk.range(&20, &40) {
    ///     println!("{}", item); // First prints 20, then 21, ... and finally 40.
    /// }
    /// ```
    #[inline]
    pub fn range<'a>(&'a self, start: &'a T, end: &'a T) -> SkipListRange<'a, T> {
        SkipListRange::new(unsafe { self.top_left.as_ref() }, start, end)
    }

    /// Iterator over an inclusive range of elements in the SkipList,
    /// as defined by the `inclusive_fn`.
    /// This runs in `O(logn + k)`, where k is the width of range.
    ///
    /// As the skiplist is ordered in an ascending way, `inclusive_fn` should be
    /// structured with the idea in mind that you're going to see the smallest elements
    /// first. `inclusive_fn` should be designed to extract a *single contiguous
    /// stretch of elements*.
    ///
    /// This iterator will find the smallest element in the range,
    /// and then return elements until it finds the first element
    /// larger than the range.
    ///
    /// If multiple ranges are desired, you can use `range_with` multiple times,
    /// and simply use the last element of the previous run as the start of
    /// the next run.
    ///
    /// # Example
    ///
    /// ```rust
    /// use convenient_skiplist::{RangeHint, SkipList};
    /// let mut sk = SkipList::new();
    /// for item in 0..100 {
    ///     sk.insert(item);
    /// }
    ///
    /// let desired_range = sk.range_with(|&ele| {
    ///     if ele <= 5 {
    ///         RangeHint::SmallerThanRange
    ///     } else if ele <= 30 {
    ///         RangeHint::InRange
    ///     } else {
    ///         RangeHint::LargerThanRange
    ///     }
    /// });
    /// for item in desired_range {
    ///     println!("{}", item); // First prints 6, then 7, ... and finally 30.
    /// }
    /// ```
    #[inline]
    pub fn range_with<F>(&self, inclusive_fn: F) -> IterRangeWith<T, F>
    where
        F: Fn(&T) -> RangeHint,
    {
        IterRangeWith::new(unsafe { self.top_left.as_ref() }, inclusive_fn)
    }

    #[inline]
    fn path_to<'a>(&self, item: &'a T) -> LeftBiasIterWidth<'a, T> {
        LeftBiasIterWidth::new(self.top_left.as_ptr(), item)
    }

    #[inline]
    fn insert_path(&mut self, item: &T) -> Vec<NodeWidth<T>> {
        self.path_to(item).collect()
    }

    fn pos_neg_pair(width: usize) -> NonNull<Node<T>> {
        let right = Box::new(Node {
            right: None,
            down: None,
            value: NodeValue::PosInf,
            width: 1,
        });
        unsafe {
            let left = Box::new(Node {
                right: Some(NonNull::new_unchecked(Box::into_raw(right))),
                down: None,
                value: NodeValue::NegInf,
                width,
            });
            NonNull::new_unchecked(Box::into_raw(left))
        }
    }

    fn make_node(value: T, width: usize) -> NonNull<Node<T>> {
        unsafe {
            let node = Box::new(Node {
                right: None,
                down: None,
                value: NodeValue::Value(value),
                width,
            });
            NonNull::new_unchecked(Box::into_raw(node))
        }
    }

    #[cfg(debug_assertions)]
    fn ensure_columns_same_value(&self) {
        let mut left_row = self.top_left;
        let mut curr_node = self.top_left;
        unsafe {
            loop {
                while let Some(right) = curr_node.as_ref().right {
                    let curr_value = &curr_node.as_ref().value;
                    let mut curr_down = curr_node;
                    while let Some(down) = curr_down.as_ref().down {
                        assert!(&down.as_ref().value == curr_value);
                        curr_down = down;
                    }
                    curr_node = right;
                }
                // Now, move a an entire row down.
                if let Some(down) = left_row.as_ref().down {
                    left_row = down;
                    curr_node = left_row;
                } else {
                    break;
                }
            }
        }
    }

    #[cfg(debug_assertions)]
    fn ensure_rows_ordered(&self) {
        let mut left_row = self.top_left;
        let mut curr_node = self.top_left;
        unsafe {
            loop {
                while let Some(right) = curr_node.as_ref().right {
                    assert!(curr_node.as_ref().value < right.as_ref().value);
                    curr_node = right;
                }
                if let Some(down) = left_row.as_ref().down {
                    left_row = down;
                    curr_node = left_row;
                } else {
                    break;
                }
            }
        }
    }

    #[cfg(debug_assertions)]
    fn ensure_rows_sum_len(&self) {
        let mut left_row = self.top_left;
        let mut curr_node = self.top_left;
        unsafe {
            loop {
                let mut curr_sum = 0;
                while let Some(right) = curr_node.as_ref().right {
                    curr_sum += curr_node.as_ref().width;
                    curr_node = right;
                }
                if let Some(down) = left_row.as_ref().down {
                    assert_eq!(self.len(), curr_sum - 1);
                    left_row = down;
                    curr_node = left_row;
                } else {
                    break;
                }
            }
        }
    }

    #[cfg(debug_assertions)]
    fn ensure_invariants(&self) {
        unsafe {
            assert!(self.top_left.as_ref().right.unwrap().as_ref().value == NodeValue::PosInf)
        }
        self.ensure_rows_ordered();
        self.ensure_columns_same_value();
        self.ensure_rows_sum_len();
    }
}

#[cfg(test)]
mod tests {
    use crate::SkipList;
    use std::collections::HashSet;

    #[test]
    fn insert_no_panic() {
        let mut sl = SkipList::new();
        for i in &[10, 30, 50, 5, 0, 3] {
            sl.insert(*i);
            assert!(sl.contains(&i));
        }
        #[cfg(debug_assertions)]
        sl.ensure_invariants();
    }

    #[test]
    fn test_remove() {
        let mut sl = SkipList::new();
        sl.insert(0usize);
        assert!(sl.remove(&0));
        assert!(!sl.remove(&0));
        assert!(!sl.contains(&0));
        sl.insert(0);
        sl.insert(1);
        sl.insert(2);
        assert!(sl.remove(&1));
        assert!(!sl.contains(&1));
        sl.remove(&2);
        assert!(!sl.contains(&2));
    }

    #[test]
    fn test_inclusive_range() {
        let mut sl = SkipList::new();
        let values: &[i32] = &[10, 30, 50, 5, 0, 3];
        for i in &[10, 30, 50, 5, 0, 3] {
            sl.insert(*i);
            assert!(sl.contains(&i));
        }
        let lower = 3;
        let upper = 30;
        let v: HashSet<i32> = sl.range(&lower, &upper).cloned().collect();
        for expected_value in values.iter().filter(|&&i| lower <= i && i <= upper) {
            assert!(v.contains(expected_value));
        }
        let right_empty: HashSet<i32> = sl.range(&100, &1000).cloned().collect();
        assert!(right_empty.is_empty());

        let left_empty: HashSet<i32> = sl.range(&-2, &-1).cloned().collect();
        assert!(left_empty.is_empty());

        // Excessive range
        let lower = -10;
        let upper = 1000;
        let v: HashSet<i32> = sl.range(&lower, &upper).cloned().collect();
        for expected_value in values.iter().filter(|&&i| lower <= i && i <= upper) {
            assert!(v.contains(expected_value));
        }
    }

    #[test]
    fn test_len() {
        let mut sl = SkipList::new();
        assert_eq!(sl.len(), 0);
        assert!(sl.is_empty());
        sl.insert(0);
        assert_eq!(sl.len(), 1);
        assert!(!sl.is_empty());
        sl.insert(0);
        assert_eq!(sl.len(), 1);
        sl.insert(1);
        assert_eq!(sl.len(), 2);
        sl.remove(&1);
        assert_eq!(sl.len(), 1);
        sl.remove(&1);
        assert_eq!(sl.len(), 1);
        sl.remove(&0);
        assert_eq!(sl.len(), 0);
        sl.remove(&0);
        assert_eq!(sl.len(), 0);
    }

    #[test]
    fn test_eq() {
        let mut s0 = SkipList::new();
        let mut s1 = SkipList::new();
        assert!(s0 == s1);
        s0.insert(0);
        assert!(s0 != s1);
        s1.insert(1);
        assert!(s0 != s1);
        s0.insert(1);
        s1.insert(0);
        assert!(s0 == s1);
        s0.insert(2);
        s0.insert(3);
        assert!(s0 != s1);
    }

    #[test]
    fn test_from() {
        let values = vec![1usize, 2, 3];
        let sk = SkipList::from(values.clone().into_iter());
        assert_eq!(sk.iter_all().cloned().collect::<Vec<_>>(), values);
        let values: Vec<usize> = (0..10).collect();
        let sk = SkipList::from(0..10);
        assert_eq!(sk.iter_all().cloned().collect::<Vec<_>>(), values);
    }

    #[test]
    fn test_index_of() {
        let mut sk = SkipList::new();
        sk.insert(1);
        sk.insert(2);
        sk.insert(3);

        assert_eq!(sk.index_of(&1), Some(0));
        assert_eq!(sk.index_of(&2), Some(1));
        assert_eq!(sk.index_of(&3), Some(2));
        assert_eq!(sk.index_of(&999), None);
        let sk = SkipList::new();
        assert_eq!(sk.index_of(&0), None);
        assert_eq!(sk.index_of(&999), None);
    }

    #[test]
    fn test_at_index() {
        let sk = SkipList::from(0..10);
        for i in 0..10 {
            assert_eq!(Some(&i), sk.at_index(i));
        }
        assert_eq!(None, sk.at_index(11));

        let mut sk = SkipList::new();
        sk.insert('a');
        sk.insert('b');
        sk.insert('c');
        assert_eq!(Some(&'a'), sk.at_index(0));
        assert_eq!(Some(&'b'), sk.at_index(1));
        assert_eq!(Some(&'c'), sk.at_index(2));
        assert_eq!(None, sk.at_index(3));
    }

    #[test]
    fn test_pop_max() {
        let mut sk = SkipList::from(0..10);
        assert_eq!(Some(&7), sk.at_index(7));
        assert_eq!(vec![7, 8, 9], sk.pop_max(3));
        assert_eq!(vec![6], sk.pop_max(1));
        assert_eq!(vec![4, 5], sk.pop_max(2));
        assert_eq!(vec![0, 1, 2, 3], sk.pop_max(5));
        let v: Vec<u32> = Vec::new();
        assert_eq!(v, sk.pop_max(1));
    }
}
