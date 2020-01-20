use crate::iter::{IterAll, IterRangeWith, LeftBiasIter, SkipListRange};
use rand;
use rand::prelude::*;
use std::cmp::{Ordering, PartialOrd};
use std::fmt;
use std::ptr::NonNull;
pub mod iter;

#[derive(PartialEq, Debug)]
enum NodeValue<T> {
    NegInf,
    Value(T),
    PosInf,
}

impl<T> NodeValue<T> {
    fn get_value(&self) -> &T {
        match &self {
            NodeValue::Value(v) => v,
            _ => unreachable!(),
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

pub struct SkipList<T> {
    top_left: NonNull<Node<T>>,
    height: u32,
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

impl<T: fmt::Debug> fmt::Debug for SkipList<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "SkipList(wall_height: {}), and table:", self.height)?;
        unsafe {
            writeln!(
                f,
                "{:?} -> {:?}",
                self.top_left.as_ref().value,
                self.top_left.as_ref().right.unwrap().as_ref().value
            )?;
            let mut curr_down = self.top_left.as_ref().down;
            while let Some(down) = curr_down {
                write!(f, "{:?}", down.as_ref().value)?;
                let mut curr_right = down.as_ref().right;
                while let Some(right) = curr_right {
                    write!(f, " -> {:?}", right.as_ref().value)?;
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
    fn default() -> Self {
        Self::new()
    }
}

/// Get the level of an item in the skiplist
fn get_level() -> u32 {
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
    /// sk.insert(0u32);
    ///
    /// assert!(sk.contains(&0));
    /// ```
    pub fn new() -> SkipList<T> {
        let mut sk = SkipList {
            top_left: SkipList::pos_neg_pair(),
            height: 1,
        };
        sk.add_levels(2);
        sk
    }

    #[inline]
    fn add_levels(&mut self, additional_levels: usize) {
        let mut curr_level = self.top_left;
        for _ in 0..additional_levels {
            let mut new_level = SkipList::pos_neg_pair();
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
        self.height += additional_levels as u32;
    }

    #[inline]
    fn iter_left<'a>(&'a self, item: &'a T) -> LeftBiasIter<'a, T> {
        LeftBiasIter::new(self.top_left.as_ptr(), item)
    }

    /// Iterator over all elements in the Skiplist.
    ///
    /// This runs in O(n) time.
    ///
    /// # Example
    ///
    /// ```rust
    /// use convenient_skiplist::SkipList;
    /// let mut sk = SkipList::new();
    /// sk.insert(0u32);
    /// sk.insert(1u32);
    /// sk.insert(2u32);
    /// for item in sk.iter_all() {
    ///     println!("{:?}", item);
    /// }
    /// ```
    #[inline]
    pub fn iter_all(&self) -> IterAll<T> {
        unsafe { IterAll::new(self.top_left.as_ref()) }
    }

    /// Iterator over an inclusive range of elements in the SkipList.
    ///
    /// This runs in O(logn + k), where k is the width of range.
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
    /// This runs in O(logn + k), where k is the width of range.
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
    /// sk.insert(0u32);
    ///
    /// assert!(sk.contains(&0));
    /// ```
    #[inline]
    pub fn contains(&self, item: &T) -> bool {
        unsafe {
            let last_ptr = self.iter_left(item).last().unwrap();
            if let Some(right) = &(*last_ptr).right {
                &right.as_ref().value == item
            } else {
                false
            }
        }
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
    /// sk.insert(0u32);
    ///
    /// let removed = sk.remove(&0);
    /// assert!(removed);
    /// ```
    pub fn remove(&mut self, item: &T) -> bool {
        let mut actually_removed_node = false;
        for node in self.iter_left(item) {
            unsafe {
                // Invariant: `node` can never be PosInf
                let right = (*node).right.unwrap();
                if &right.as_ref().value != item {
                    continue;
                }
                // So the node right of us needs to be removed.
                actually_removed_node = true;
                let garbage = std::mem::replace(&mut (*node).right, right.as_ref().right);
                drop(garbage);
            }
        }
        actually_removed_node
    }

    #[inline]
    fn path_to(&mut self, item: &T) -> Vec<*mut Node<T>> {
        self.iter_left(item).collect()
    }

    fn pos_neg_pair() -> NonNull<Node<T>> {
        let right = Box::new(Node {
            right: None,
            down: None,
            value: NodeValue::PosInf,
        });
        unsafe {
            let left = Box::new(Node {
                right: Some(NonNull::new_unchecked(Box::into_raw(right))),
                down: None,
                value: NodeValue::NegInf,
            });
            NonNull::new_unchecked(Box::into_raw(left))
        }
    }

    fn make_node(value: T) -> NonNull<Node<T>> {
        unsafe {
            let node = Box::new(Node {
                right: None,
                down: None,
                value: NodeValue::Value(value),
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
    fn ensure_invariants(&self) {
        unsafe {
            assert!(self.top_left.as_ref().right.unwrap().as_ref().value == NodeValue::PosInf)
        }
        self.ensure_rows_ordered();
        self.ensure_columns_same_value();
    }

    /// Insert `item` into the SkipList.
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
    /// sk.insert(0u32);
    ///
    /// assert!(sk.contains(&0));
    /// ```
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
        for node in self.path_to(&item).into_iter().rev().take(height as usize) {
            let mut new_node = SkipList::make_node(item.clone());
            let node: *mut Node<T> = node;
            unsafe {
                new_node.as_mut().down = node_below_me;
                new_node.as_mut().right = (*node).right;
                (*node).right = Some(new_node);
                node_below_me = Some(new_node);
            }
        }
        #[cfg(debug_assertions)]
        {
            self.ensure_invariants()
        }
        true
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
        sl.insert(0u32);
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
        dbg!(&v);
        for expected_value in values.iter().filter(|&&i| lower <= i && i <= upper) {
            dbg!(&expected_value);
            assert!(v.contains(expected_value));
        }
        let right_empty: HashSet<i32> = sl.range(&100, &1000).cloned().collect();
        dbg!(&right_empty);
        assert!(right_empty.is_empty());

        let left_empty: HashSet<i32> = sl.range(&-2, &-1).cloned().collect();
        dbg!(&left_empty);
        assert!(left_empty.is_empty());

        // Excessive range
        let lower = -10;
        let upper = 1000;
        let v: HashSet<i32> = sl.range(&lower, &upper).cloned().collect();
        dbg!(&v);
        for expected_value in values.iter().filter(|&&i| lower <= i && i <= upper) {
            dbg!(&expected_value);
            assert!(v.contains(expected_value));
        }
    }
}
