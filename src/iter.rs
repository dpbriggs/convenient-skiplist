use crate::{Node, NodeValue, RangeHint, SkipList};
use core::ops::{Bound, RangeBounds};
use std::hint::unreachable_unchecked;

pub(crate) struct VerticalIter<T> {
    curr_node: Option<*mut Node<T>>,
}

impl<T> VerticalIter<T> {
    pub(crate) fn new(curr_node: *mut Node<T>) -> Self {
        Self {
            curr_node: Some(curr_node),
        }
    }
}

impl<T> Iterator for VerticalIter<T> {
    type Item = *mut Node<T>;
    fn next(&mut self) -> Option<Self::Item> {
        let next = self
            .curr_node
            .and_then(|n| unsafe { (*n).down })
            .map(|p| p.as_ptr());

        std::mem::replace(&mut self.curr_node, next)
    }
}

/// Iterator to grab all values from the right of `curr_node`
pub(crate) struct NodeRightIter<T> {
    curr_node: *mut Node<T>,
}

impl<T> NodeRightIter<T> {
    pub(crate) fn new(curr_node: *mut Node<T>) -> Self {
        Self { curr_node }
    }
}

impl<T: Clone> Iterator for NodeRightIter<T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let next = (*self.curr_node).right?.as_ptr();
            let ret = std::mem::replace(&mut self.curr_node, next);
            Some((*ret).value.get_value().clone())
        }
    }
}

/// Struct to keep track of things for IntoIterator
/// *Warning*: As all nodes are heap allocated, we have
/// to clone them to produce type T.
pub struct IntoIter<T> {
    _skiplist: SkipList<T>,
    curr_node: *mut Node<T>,
    finished: bool,
    total_len: usize,
}

impl<T: Clone> Iterator for IntoIter<T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        unsafe {
            match (*self.curr_node).right {
                Some(right) => {
                    self.curr_node = right.as_ptr();
                    Some((*self.curr_node).value.get_value().clone())
                }
                None => {
                    self.finished = true;
                    Some((*self.curr_node).value.get_value().clone())
                }
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.total_len, Some(self.total_len))
    }
}

impl<T: PartialOrd + Clone> IntoIterator for SkipList<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            total_len: self.len,
            curr_node: self.top_left.as_ptr(),
            _skiplist: self,
            finished: false,
        }
    }
}

// TODO: Drain
// pub struct Drain<T> {
//     curr_node: *mut Node<T>,
//     finished: bool,
// }

// impl<T: Clone> Iterator for Drain<T> {
//     type Item = T;
//     fn next(&mut self) -> Option<Self::Item> {
//         if self.finished {
//             return None;
//         }
//         unsafe {
//             match (*self.curr_node).right {
//                 Some(right) => {
//                     let ret = std::mem::replace(&mut self.curr_node, right.as_ptr());
//                     let ret = Box::from_raw(ret);
//                     return Some(ret.value.get_value().clone());
//                 }
//                 None => {
//                     self.finished = true;
//                     return Some(Box::from_raw(self.curr_node).value.get_value().clone());
//                 }
//             };
//         };
//     }
// }

/// IterAll is a iterator struct to iterate over the entire
/// linked list.
///
/// You should use the method `iter_all` on [SkipList](convenient-skiplist::SkipList)
pub struct IterAll<'a, T> {
    curr_node: &'a Node<T>,
    at_bottom: bool,
    finished: bool,
    total_len: usize,
}

impl<'a, T> IterAll<'a, T> {
    #[inline]
    pub(crate) fn new(curr_node: &'a Node<T>, total_len: usize) -> Self {
        Self {
            curr_node,
            at_bottom: false,
            finished: false,
            total_len,
        }
    }
}

impl<'a, T: PartialOrd> Iterator for IterAll<'a, T> {
    type Item = &'a T;
    #[inline]
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
                NodeValue::Value(..) => {}
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

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.total_len, Some(self.total_len))
    }
}

pub struct SkipListIndexRange<'a, R: RangeBounds<usize>, T> {
    range: R,
    curr_node: *const Node<T>,
    curr_index: usize,
    phantom: std::marker::PhantomData<&'a T>,
}

impl<'a, R: RangeBounds<usize>, T> SkipListIndexRange<'a, R, T> {
    pub(crate) fn new(curr_node: *const Node<T>, range: R) -> Self {
        let mut curr_node = curr_node;
        // Find closest starting node
        let mut curr_index = 0;
        let mut curr_node = match range.start_bound() {
            Bound::Unbounded => {
                while let Some(down) = unsafe { (*curr_node).down } {
                    curr_node = down.as_ptr();
                }
                // Advance once to the right from neginf
                unsafe { (*curr_node).right.unwrap().as_ptr() }
            }
            bound => loop {
                unsafe {
                    match ((*curr_node).right, (*curr_node).down) {
                        (Some(right), Some(down)) => {
                            let idx = match bound {
                                Bound::Included(&idx) => {
                                    let idx = idx + 1;
                                    if curr_index == idx {
                                        break curr_node;
                                    }
                                    idx
                                }
                                Bound::Excluded(&idx) => {
                                    let idx = idx + 1;
                                    if curr_index == idx {
                                        break right.as_ptr();
                                    }
                                    idx
                                }
                                _ => unreachable_unchecked(),
                            };
                            let width = (*curr_node).width;
                            if curr_index + width <= idx {
                                curr_node = right.as_ptr() as *const _;
                                curr_index += width;
                            } else {
                                curr_node = down.as_ptr();
                            }
                        }
                        (Some(right), None) => {
                            match bound {
                                Bound::Included(&idx) => {
                                    if curr_index == idx + 1 {
                                        break curr_node;
                                    }
                                }
                                Bound::Excluded(&idx) => {
                                    if curr_index == idx + 1 {
                                        break right.as_ptr();
                                    }
                                }
                                _ => unreachable_unchecked(),
                            };
                            curr_node = right.as_ptr();
                            curr_index += (*curr_node).width;
                        }
                        (None, None) => {
                            break curr_node;
                        }
                        _ => unreachable!("264"),
                    }
                }
            },
        };
        // Make sure we reach the bottom
        while let Some(down) = unsafe { (*curr_node).down } {
            curr_node = down.as_ptr();
        }
        Self {
            range,
            curr_node,
            curr_index: curr_index.saturating_sub(1),
            phantom: std::marker::PhantomData::default(),
        }
    }
}

macro_rules! get_value_and_advance {
    ($curr:expr, $right:expr) => {{
        Some(
            (*std::mem::replace($curr, $right.as_ptr()))
                .value
                .get_value(),
        )
    }};
}

impl<'a, T, R: RangeBounds<usize>> Iterator for SkipListIndexRange<'a, R, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            debug_assert!((*self.curr_node).down.is_none());
            let right = (*self.curr_node).right?;
            match self.range.end_bound() {
                Bound::Unbounded => get_value_and_advance!(&mut self.curr_node, right),
                Bound::Included(&idx) => {
                    if self.curr_index > idx {
                        return None;
                    }
                    self.curr_index += 1;
                    get_value_and_advance!(&mut self.curr_node, right)
                }
                Bound::Excluded(&idx) => {
                    if self.curr_index == idx {
                        return None;
                    }
                    self.curr_index += 1;
                    get_value_and_advance!(&mut self.curr_node, right)
                }
            }
        }
    }
}

pub struct SkipListRange<'a, T> {
    curr_node: &'a Node<T>,
    start: &'a T,
    end: &'a T,
    at_bottom: bool,
}

impl<'a, T> SkipListRange<'a, T> {
    pub(crate) fn new(curr_node: &'a Node<T>, start: &'a T, end: &'a T) -> Self {
        Self {
            curr_node,
            start,
            end,
            at_bottom: false,
        }
    }
}

impl<'a, T: PartialOrd> Iterator for SkipListRange<'a, T> {
    type Item = &'a T;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // Step 1: Find the first node >= self.start
        while !self.at_bottom {
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
                    } else {
                        self.at_bottom = true;
                        self.curr_node = self.curr_node.right.unwrap().as_ptr().as_ref().unwrap();
                        break;
                    }
                },
                _ => unreachable!(),
            }
        }
        // Verify that we are, indeed, at the bottom
        debug_assert!(self.curr_node.down.is_none());
        if &self.curr_node.value <= self.end {
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

#[derive(Clone)]
pub(crate) struct NodeWidth<T> {
    pub curr_node: *mut Node<T>,
    /// The total width traveled so _far_ in the iterator.
    /// The last iteration is guaranteed to be the true width from
    /// negative infinity to the element.
    pub curr_width: usize,
}

impl<T> NodeWidth<T> {
    pub(crate) fn new(curr_node: *mut Node<T>, curr_width: usize) -> Self {
        Self {
            curr_node,
            curr_width,
        }
    }
}

pub(crate) struct LeftBiasIterWidth<'a, T> {
    curr_node: *mut Node<T>,
    total_width: usize,
    item: &'a T,
    finished: bool,
}

impl<'a, T> LeftBiasIterWidth<'a, T> {
    pub(crate) fn new(curr_node: *mut Node<T>, item: &'a T) -> Self {
        Self {
            curr_node,
            item,
            finished: false,
            total_width: 0,
        }
    }
}

impl<'a, T: PartialOrd> Iterator for LeftBiasIterWidth<'a, T> {
    type Item = NodeWidth<T>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        unsafe {
            loop {
                match ((*self.curr_node).right, (*self.curr_node).down) {
                    // We're somewhere in the middle of the skiplist
                    (Some(right), Some(down)) => {
                        // The node our right is smaller than `item`, so let's advance forward.
                        if &right.as_ref().value < self.item {
                            self.total_width += (*self.curr_node).width;
                            self.curr_node = right.as_ptr();
                        } else {
                            // The node to our right is the first seen that's larger than `item`,
                            // So we yield it and head down.
                            let ret_node = std::mem::replace(&mut self.curr_node, down.as_ptr());
                            return Some(NodeWidth::new(ret_node, self.total_width));
                        }
                    }
                    // We're at the bottom of the skiplist
                    (Some(right), None) => {
                        // We're at the bottom row, and the item to our right >= `self.item`.
                        // This is exactly the same as a linked list -- we don't want to continue further.
                        if &right.as_ref().value >= self.item {
                            self.finished = true;
                            return Some(NodeWidth::new(self.curr_node, self.total_width));
                        } else {
                            // The node to our right is _smaller_ than us, so continue forward.
                            self.curr_node = right.as_ptr();
                            self.total_width += 1;
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
    #[inline]
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

    // Is `item` in our range?
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
    #[inline]
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
        let expected: Vec<usize> = (0..10).collect();
        for e in &expected {
            sk.insert(*e);
        }
        let foo: Vec<_> = sk.iter_all().cloned().collect();
        for i in 0..expected.len() {
            assert_eq!(expected[i], foo[i]);
        }
        let mut second = foo.clone();
        second.sort();
        assert_eq!(foo, second)
    }

    #[test]
    fn test_empty() {
        let sk = SkipList::<usize>::new();
        let foo: Vec<_> = sk.iter_all().cloned().collect();
        assert!(foo.is_empty());
    }

    // MIRI: This test takes forever.
    #[test]
    fn test_range() {
        let mut sk = SkipList::new();
        for i in 0..500 {
            sk.insert(i);
        }
        let expected: Vec<usize> = (50..=100).collect();
        let got: Vec<usize> = sk.range(&50, &100).cloned().collect();
        assert_eq!(expected, got);
    }

    #[test]
    fn test_range_empty() {
        let sk = SkipList::new();
        let expected: Vec<usize> = Vec::new();
        let got: Vec<usize> = sk.range(&50, &100).cloned().collect();
        assert_eq!(expected, got);
    }

    #[test]
    fn test_range_outside() {
        let mut sk = SkipList::new();
        for i in 20..30 {
            sk.insert(i);
        }
        let expected: Vec<usize> = Vec::new();
        let less: Vec<usize> = sk.range(&0, &19).cloned().collect();
        let more: Vec<usize> = sk.range(&30, &32).cloned().collect();
        assert_eq!(expected, less);
        assert_eq!(expected, more);
    }

    #[test]
    fn test_inclusion_fn_range_with() {
        use crate::iter::IterRangeWith;
        use crate::{Node, NodeValue};
        let n = Node {
            right: None,
            down: None,
            value: NodeValue::Value(3),
            width: 1,
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
    fn test_index_range() {
        use std::ops::RangeBounds;
        fn sk_range<R: RangeBounds<usize>>(range: R) -> Vec<usize> {
            let sk = SkipList::from(0..20);
            sk.index_range(range).cloned().collect()
        }

        fn vec_range<R: RangeBounds<usize>>(range: R) -> Vec<usize> {
            let mut vec: Vec<_> = (0..20).collect();
            vec.drain(range).collect()
        }

        fn test_against<R: RangeBounds<usize> + Clone + std::fmt::Debug>(range: R) {
            assert_eq!(
                sk_range(range.clone()),
                vec_range(range.clone()),
                "\nRange that caused the failure: {:?}",
                range
            );
        }

        test_against(..);
        test_against(4..10);
        test_against(0..20);
        test_against(20..20);
        test_against(..20);
        test_against(10..);
        test_against(20..);
        test_against(1..1);
        test_against(1..=1);
        test_against(3..=8);
        test_against(..=8);

        // assert_eq!(sk_range(..), vec_range(..));
        // assert_eq!(sk_range(4..10), vec_range(4..10));
        // assert_eq!(sk_range(0..20), vec_range(0..20));
        // assert_eq!(sk_range(20..20), vec_range(20..20));
        // assert_eq!(sk_range(..20), vec_range(..20));
        // assert_eq!(sk_range(10..), vec_range(10..));
        // assert_eq!(sk_range(20..), vec_range(20..));
        // assert_eq!(sk_range(1..1), vec_range(1..1));
        // assert_eq!(sk_range(1..=1), vec_range(1..=1));
        // assert_eq!(sk_range(3..=8), vec_range(3..=8));
        // assert_eq!(sk_range(..=8), vec_range(..=8));
    }

    #[test]
    fn test_range_with() {
        use crate::iter::RangeHint;
        let mut sk = SkipList::<usize>::new();
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
        let sk = SkipList::<usize>::new();
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
        let expected: Vec<usize> = vec![];
        assert_eq!(f, expected);
    }

    #[test]
    fn test_range_with_all() {
        use crate::iter::RangeHint;
        let mut sk = SkipList::<usize>::new();
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
        let mut sk = SkipList::<usize>::new();
        let expected = &[0, 1, 2, 3, 4, 5];
        for e in expected {
            sk.insert(*e);
        }
        let f: Vec<_> = sk
            .range_with(|&_i| RangeHint::SmallerThanRange)
            .cloned()
            .collect();
        // compiler bug? Should not need to specify type
        let expected: Vec<usize> = Vec::new();
        assert_eq!(f, expected);
        let f: Vec<_> = sk
            .range_with(|&_i| RangeHint::LargerThanRange)
            .cloned()
            .collect();
        assert_eq!(f, expected);
    }

    // You should run this test with miri
    #[test]
    fn test_range_pathological_no_panic() {
        use crate::RangeHint;
        use rand;
        use rand::prelude::*;
        let mut sk = SkipList::<usize>::new();
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
