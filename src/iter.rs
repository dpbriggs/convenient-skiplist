use crate::{Node, NodeValue};

/// IterAll is a iterator struct to iterate over the entire
/// linked list.
///
/// You should use the method `iter_all` on [SkipList](convenient-skiplist::SkipList)
pub struct IterAll<'a, T> {
    curr_node: &'a Node<T>,
    at_bottom: bool,
    finished: bool,
}

impl<'a, T> IterAll<'a, T> {
    #[inline]
    pub(crate) fn new(curr_node: &'a Node<T>) -> Self {
        Self {
            curr_node,
            at_bottom: false,
            finished: false,
        }
    }
}

impl<'a, T: PartialOrd> Iterator for IterAll<'a, T> {
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
                Some(self.curr_node.value.get_value())
            } else {
                let next = self.curr_node.right.unwrap().as_ptr().as_ref().unwrap();
                let to_ret = std::mem::replace(&mut self.curr_node, next);
                Some(to_ret.value.get_value())
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

impl<'a, T: PartialOrd> Iterator for SkipListRange<'a, T> {
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

impl<'a, T: PartialOrd> Iterator for LeftBiasIter<'a, T> {
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
#[derive(Debug)]
pub enum RangeHint {
    SmallerThanRange,
    InRange,
    LargerThanRange,
}

pub struct IterRangeWith<'a, T, F>
where
    T: PartialOrd,
    F: Fn(&'a T) -> RangeHint,
{
    inclusive_fn: F,
    curr_node: &'a Node<T>,
    at_bottom: bool,
}

impl<'a, T, F> IterRangeWith<'a, T, F>
where
    T: PartialOrd,
    F: Fn(&T) -> RangeHint,
{
    #[inline]
    pub(crate) fn new(curr_node: &'a Node<T>, inclusive_fn: F) -> Self {
        Self {
            inclusive_fn,
            curr_node,
            at_bottom: false,
        }
    }

    // Is `item` smaller than our range?
    #[inline]
    fn item_smaller_than_range(&self, item: &NodeValue<T>) -> bool {
        match item {
            NodeValue::NegInf => true,
            NodeValue::PosInf => false,
            NodeValue::Value(v) => {
                if let RangeHint::SmallerThanRange = (self.inclusive_fn)(v) {
                    true
                } else {
                    false
                }
            }
        }
    }

    #[inline]
    fn item_in_range(&self, item: &NodeValue<T>) -> bool {
        match item {
            NodeValue::NegInf => false,
            NodeValue::PosInf => false,
            NodeValue::Value(v) => {
                if let RangeHint::InRange = (self.inclusive_fn)(v) {
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
    T: PartialOrd,
    F: Fn(&T) -> RangeHint,
{
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        // Step 1: Find the *largest* element smaller than our range.
        // This process is _very_ similar to LeftBiasIter, where
        // we search for the element immediately left of the desired one.
        while !self.at_bottom {
            match (self.curr_node.right, self.curr_node.down) {
                // We're in the middle of the skiplist somewhere
                (Some(right), Some(down)) => unsafe {
                    // The item to our right is _smaller_ than our range,
                    // so we get to skip right.
                    if self.item_smaller_than_range(&right.as_ref().value) {
                        self.curr_node = right.as_ptr().as_ref().unwrap();
                    } else {
                        // The item is in our range, or larger, so we need to go down.
                        self.curr_node = down.as_ptr().as_ref().unwrap();
                    }
                },
                // We're at the bottom of the skiplist
                (Some(right), None) => unsafe {
                    // The item immediately to our right is _smaller_ than the range,
                    // so advance right.
                    if self.item_smaller_than_range(&right.as_ref().value) {
                        self.curr_node = right.as_ptr().as_ref().unwrap();
                    } else {
                        // The element to our right is in the range, or larger!
                        self.at_bottom = true;
                        // We're exactly ONE step away from the first item in the range,
                        // so advance one to the right
                        self.curr_node = self.curr_node.right.unwrap().as_ptr().as_ref().unwrap();
                        break;
                    }
                },
                _ => unreachable!(),
            }
        }
        // Verify that we are, indeed, at the bottom
        debug_assert!(self.curr_node.down.is_none());
        if self.item_in_range(&self.curr_node.value) {
            unsafe {
                let ret_val = &self.curr_node.value;
                let next = self.curr_node.right.unwrap().as_ptr().as_ref().unwrap();
                self.curr_node = next;
                return Some(ret_val.get_value());
            }
        }
        None
    }
}

#[cfg(test)]
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
        let foo: Vec<_> = sk.iter_all().cloned().collect();
        assert!(foo.is_empty());
    }

    #[test]
    fn test_inclusion_fn_range_with() {
        use crate::iter::IterRangeWith;
        use crate::{Node, NodeValue};
        let n = Node {
            right: None,
            down: None,
            value: NodeValue::Value(3),
        };
        let srw = IterRangeWith::new(&n, |&i| {
            if i < 2 {
                RangeHint::SmallerThanRange
            } else if i > 4 {
                RangeHint::LargerThanRange
            } else {
                RangeHint::InRange
            }
        });
        assert!(srw.item_smaller_than_range(&NodeValue::Value(1)) == true);
        assert!(srw.item_smaller_than_range(&NodeValue::Value(2)) == false);
        assert!(srw.item_smaller_than_range(&NodeValue::Value(4)) == false);
        assert!(srw.item_smaller_than_range(&NodeValue::Value(5)) == false);
        assert!(srw.item_smaller_than_range(&NodeValue::NegInf) == true);
        assert!(srw.item_smaller_than_range(&NodeValue::PosInf) == false);

        assert!(srw.item_in_range(&NodeValue::Value(1)) == false);
        assert!(srw.item_in_range(&NodeValue::Value(2)) == true);
        assert!(srw.item_in_range(&NodeValue::Value(3)) == true);
        assert!(srw.item_in_range(&NodeValue::Value(4)) == true);
        assert!(srw.item_in_range(&NodeValue::Value(5)) == false);
        assert!(srw.item_in_range(&NodeValue::PosInf) == false);
        assert!(srw.item_in_range(&NodeValue::NegInf) == false);
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

    #[test]
    fn test_range_with_empty() {
        use crate::iter::RangeHint;
        let sk = SkipList::<u32>::new();
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
        assert_eq!(f, vec![]);
    }

    #[test]
    fn test_range_with_all() {
        use crate::iter::RangeHint;
        let mut sk = SkipList::<u32>::new();
        let expected = &[0, 1, 2, 3, 4, 5];
        for e in expected {
            sk.insert(*e);
        }
        let f: Vec<_> = sk.range_with(|&_i| RangeHint::InRange).cloned().collect();
        assert_eq!(f, expected.to_vec());
    }

    #[test]
    fn test_range_with_none() {
        use crate::iter::RangeHint;
        let mut sk = SkipList::<u32>::new();
        let expected = &[0, 1, 2, 3, 4, 5];
        for e in expected {
            sk.insert(*e);
        }
        let f: Vec<_> = sk
            .range_with(|&_i| RangeHint::SmallerThanRange)
            .cloned()
            .collect();
        assert_eq!(f, vec![]);
        let f: Vec<_> = sk
            .range_with(|&_i| RangeHint::LargerThanRange)
            .cloned()
            .collect();
        assert_eq!(f, vec![]);
    }

    // You should run this test with miri
    #[test]
    fn test_range_pathological_no_panic() {
        use crate::iter::RangeHint;
        use rand;
        use rand::prelude::*;
        let mut sk = SkipList::<u32>::new();
        let expected = &[0, 1, 2, 3, 4, 5];
        for e in expected {
            sk.insert(*e);
        }
        let _f: Vec<_> = sk
            .range_with(|&_i| {
                let mut thrng = rand::thread_rng();
                let r: f32 = thrng.gen();
                if 0.0 < r && r < 0.33 {
                    RangeHint::SmallerThanRange
                } else if r < 0.66 {
                    RangeHint::InRange
                } else {
                    RangeHint::LargerThanRange
                }
            })
            .cloned()
            .collect();
    }
}

// #[cfg(test)]
// mod bench {
//     extern crate test;
//     use crate::iter::RangeHint;
//     use crate::SkipList;
//     use test::Bencher;
//     #[bench]
//     fn bench_iter_all(b: &mut Bencher) {
//         let mut sk = SkipList::<u32>::new();
//         let upper = 500;
//         for i in 0..upper {
//             sk.insert(i);
//         }
//         b.iter(|| for _i in sk.iter_all() {});
//     }

//     #[bench]
//     fn bench_iter_range(b: &mut Bencher) {
//         let mut sk = SkipList::<u32>::new();
//         let upper = 500;
//         for i in 0..upper {
//             sk.insert(i);
//         }
//         b.iter(|| for _i in sk.range(&(upper / 2), &(upper / 2 + upper / 5)) {});
//     }

//     #[bench]
//     fn bench_iter_range_with(b: &mut Bencher) {
//         let mut sk = SkipList::<u32>::new();
//         let upper = 500;
//         for i in 0..upper {
//             sk.insert(i);
//         }
//         b.iter(|| {
//             let f = sk.range_with(|&i| {
//                 if i < (upper / 2) {
//                     RangeHint::SmallerThanRange
//                 } else if i > (upper / 2 + upper / 5) {
//                     RangeHint::LargerThanRange
//                 } else {
//                     RangeHint::InRange
//                 }
//             });
//             for _i in f {}
//         });
//     }
// }