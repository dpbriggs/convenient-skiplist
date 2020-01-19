use crate::{Node, NodeValue};
use std::cmp::Ordering;

/// IterAll is an
pub struct IterAll<'a, T> {
    curr_node: &'a Node<T>,
    at_bottom: bool,
    finished: bool,
}

impl<'a, T> IterAll<'a, T> {
    pub(crate) fn new(curr_node: &'a Node<T>) -> Self {
        Self {
            curr_node,
            at_bottom: false,
            finished: false,
        }
    }
}

impl<'a, T: PartialEq + PartialOrd> Iterator for IterAll<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        // step 1: Hit the bottom
        if !self.at_bottom {
            unsafe {
                while let Some(down) = self.curr_node.down.as_ref() {
                    self.curr_node = down.as_ref();
                }
            }
            // step 2: Go one to the right
            unsafe {
                self.curr_node = self.curr_node.right.unwrap().as_ptr().as_ref().unwrap();
            }
            self.at_bottom = true;
        }
        unsafe {
            match self.curr_node.value {
                NodeValue::NegInf => {
                    self.curr_node = self.curr_node.right.unwrap().as_ptr().as_ref().unwrap();
                }
                NodeValue::PosInf => return None,
                _ => {}
            };
            if self.curr_node.right.unwrap().as_ref().value == NodeValue::PosInf {
                self.finished = true;
                return Some(self.curr_node.value.get_value());
            } else {
                let next = self.curr_node.right.unwrap().as_ptr().as_ref().unwrap();
                let to_ret = std::mem::replace(&mut self.curr_node, next);
                return Some(to_ret.value.get_value());
            }
        }
    }
}

pub struct SkipListRange<'a, T> {
    curr_node: &'a Node<T>,
    start: &'a T,
    end: &'a T,
}

impl<'a, T> SkipListRange<'a, T> {
    pub(crate) fn new(curr_node: &'a Node<T>, start: &'a T, end: &'a T) -> Self {
        Self {
            curr_node,
            start,
            end,
        }
    }
}

impl<'a, T: PartialOrd + PartialEq> Iterator for SkipListRange<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        // Step 1: Find the first node >= self.start
        while &self.curr_node.value < self.start {
            match (self.curr_node.right, self.curr_node.down) {
                (Some(right), Some(down)) => unsafe {
                    if &right.as_ref().value < self.start {
                        self.curr_node = right.as_ptr().as_ref().unwrap();
                    } else {
                        self.curr_node = down.as_ptr().as_ref().unwrap();
                    }
                },
                (Some(right), None) => unsafe {
                    if &right.as_ref().value < self.start {
                        self.curr_node = right.as_ptr().as_ref().unwrap();
                    } else if &right.as_ref().value > self.end {
                        return None;
                    } else {
                        break; // ?
                    }
                },
                _ => unreachable!(),
            }
        }
        // Now, head to the bottom.
        while let Some(down) = self.curr_node.down {
            unsafe {
                self.curr_node = down.as_ptr().as_ref().unwrap();
            }
        }
        // curr_node is now >= self.start
        while &self.curr_node.value <= self.end {
            unsafe {
                let next = self.curr_node.right.unwrap().as_ptr().as_ref().unwrap();
                let curr_value = std::mem::replace(&mut self.curr_node, next);
                match &curr_value.value {
                    NodeValue::Value(v) => return Some(v),
                    _ => continue,
                }
            }
        }
        None
    }
}

/// Left-biased iteration towards `item`.
///
/// Guaranteed to return an iterator of items directly left of `item`,
/// or where `item` should be in the skiplist.
pub(crate) struct LeftBiasIter<'a, T> {
    curr_node: *mut Node<T>,
    item: &'a T,
    finished: bool,
}

impl<'a, T> LeftBiasIter<'a, T> {
    pub(crate) fn new(curr_node: *mut Node<T>, item: &'a T) -> Self {
        Self {
            curr_node,
            item,
            finished: false,
        }
    }
}

impl<'a, T: PartialEq + PartialOrd> Iterator for LeftBiasIter<'a, T> {
    type Item = *mut Node<T>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        unsafe {
            loop {
                match ((*self.curr_node).right, (*self.curr_node).down) {
                    // We're somewhere in the middle of the skiplist, so if `self.item` is larger than our right,
                    (Some(right), Some(down)) => {
                        // The node our right is smaller than `item`, so let's advance forward.
                        if &right.as_ref().value < self.item {
                            self.curr_node = right.as_ptr();
                        } else {
                            // The node to our right is the first seen that's larger than `item`,
                            // So we yield it and head down.
                            return Some(std::mem::replace(&mut self.curr_node, down.as_ptr()));
                        }
                    }
                    // We're at the bottom of the skiplist
                    (Some(right), None) => {
                        // We're at the bottom row, and the item to our right >= `self.item`.
                        // This is exactly the same as a linked list -- we don't want to continue further.
                        if &right.as_ref().value >= self.item {
                            self.finished = true;
                            return Some(self.curr_node);
                        } else {
                            // The node to our right is _smaller_ than us, so continue forward.
                            self.curr_node = right.as_ptr();
                        }
                    }
                    // If we've upheld invariants correctly, there's always a right when iterating
                    // Otherwise, some element was larger than NodeValue::PosInf.
                    _ => unreachable!(),
                }
            }
        }
    }
}

/// Hint that the current value `item` is:
/// - Smaller (outside) than the desired
/// - Inside the desired range
/// - Larger (outside) the desired range
///
/// Used with IterRangeWith, or `range_with`
pub enum RangeHint {
    SmallerThanRange,
    InRange,
    LargerThanRange,
}

pub struct IterRangeWith<'a, T, F>
where
    T: PartialEq + PartialOrd,
    F: Fn(&'a T) -> RangeHint,
{
    inclusive_fn: F,
    curr_node: &'a Node<T>,
}

impl<'a, T, F> IterRangeWith<'a, T, F>
where
    T: PartialEq + PartialOrd,
    F: Fn(&T) -> RangeHint,
{
    pub(crate) fn new(curr_node: &'a Node<T>, inclusive_fn: F) -> Self {
        Self {
            inclusive_fn,
            curr_node,
        }
    }

    #[inline]
    fn item_lt(&self, item: &NodeValue<T>) -> bool {
        match item {
            NodeValue::NegInf => false,
            NodeValue::PosInf => true,
            NodeValue::Value(v) => {
                if let RangeHint::SmallerThanRange = (self.inclusive_fn)(v) {
                    true
                } else {
                    false
                }
            }
        }
    }

    fn item_in_range(&self, item: &NodeValue<T>) -> bool {
        match item {
            NodeValue::NegInf => false,
            NodeValue::PosInf => true,
            NodeValue::Value(v) => {
                if let RangeHint::InRange = (self.inclusive_fn)(v) {
                    true
                } else {
                    false
                }
            }
        }
    }

    fn item_gt(&self, item: &NodeValue<T>) -> bool {
        match item {
            NodeValue::NegInf => false,
            NodeValue::PosInf => true,
            NodeValue::Value(v) => {
                if let RangeHint::LargerThanRange = (self.inclusive_fn)(v) {
                    true
                } else {
                    false
                }
            }
        }
    }
}

impl<'a, T, F> Iterator for IterRangeWith<'a, T, F>
where
    T: PartialEq + PartialOrd,
    F: Fn(&T) -> RangeHint,
{
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        while self.item_lt(&self.curr_node.value) {
            match (self.curr_node.right, self.curr_node.down) {
                (Some(right), Some(down)) => unsafe {
                    if self.item_lt(&right.as_ref().value) {
                        self.curr_node = right.as_ptr().as_ref().unwrap();
                    } else {
                        self.curr_node = down.as_ptr().as_ref().unwrap();
                    }
                },
                (Some(right), None) => unsafe {
                    if self.item_lt(&right.as_ref().value) {
                        self.curr_node = right.as_ptr().as_ref().unwrap();
                    } else if self.item_gt(&right.as_ref().value) {
                        return None;
                    } else {
                        break; // ?
                    }
                },
                _ => unreachable!(),
            }
        }
        // Now, head to the bottom.
        while let Some(down) = self.curr_node.down {
            unsafe {
                self.curr_node = down.as_ptr().as_ref().unwrap();
            }
        }
        // curr_node is now >= self.start
        while !self.item_gt(&self.curr_node.value) {
            unsafe {
                let next = self.curr_node.right.unwrap().as_ptr().as_ref().unwrap();
                let curr_value = std::mem::replace(&mut self.curr_node, next);
                match &curr_value.value {
                    NodeValue::Value(v) => return Some(v),
                    _ => continue,
                }
            }
        }
        None
    }
}

mod tests {
    use crate::RangeHint;
    use crate::SkipList;

    #[test]
    fn test_iterall() {
        let mut sk = SkipList::new();
        let expected = &[0, 1, 2];
        for e in expected {
            sk.insert(*e);
        }
        let foo: Vec<_> = sk.iter_all().cloned().collect();
        for i in 0..3 {
            assert_eq!(expected[i], foo[i]);
        }
    }

    #[test]
    fn test_empty() {
        let sk = SkipList::<u32>::new();
        dbg!(&sk);
        let foo: Vec<_> = sk.iter_all().cloned().collect();
        assert!(foo.is_empty());
    }

    #[test]
    fn test_range_with() {
        use crate::iter::RangeHint;
        let mut sk = SkipList::<u32>::new();
        let expected = &[0, 1, 2, 3, 4, 5];
        for e in expected {
            sk.insert(*e);
        }
        let f: Vec<_> = sk
            .range_with(|&i| {
                if i < 2 {
                    RangeHint::SmallerThanRange
                } else if i > 4 {
                    RangeHint::LargerThanRange
                } else {
                    RangeHint::InRange
                }
            })
            .cloned()
            .collect();
        assert_eq!(f, vec![2, 3, 4]);
    }
}
