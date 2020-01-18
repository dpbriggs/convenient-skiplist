use rand;
use rand::prelude::*;
use std::cmp::{Ordering, PartialOrd};
use std::fmt;
use std::ptr::NonNull;
#[derive(PartialEq, Debug)]
enum NodeValue<T> {
    NegInf,
    Value(T),
    PosInf,
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

impl<T: PartialEq + PartialOrd + std::fmt::Debug> PartialOrd<NodeValue<T>> for NodeValue<T> {
    #[inline]
    fn partial_cmp(&self, other: &NodeValue<T>) -> Option<Ordering> {
        // dbg!((self, other));
        match (self, other) {
            (NodeValue::NegInf, _) => Some(Ordering::Less),
            (_, NodeValue::PosInf) => Some(Ordering::Less),
            (NodeValue::Value(l), NodeValue::Value(r)) => l.partial_cmp(r),
            _ => unreachable!(),
        }
    }
}

impl<T: PartialEq + PartialOrd> PartialOrd<T> for NodeValue<T> {
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
        // unsafe { drop(Box::from_raw(self.top_left.as_ptr())) }
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
        write!(f, ")")
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

pub struct SkipList<T> {
    top_left: NonNull<Node<T>>,
    height: u32,
}

fn get_level() -> u32 {
    let mut height = 1;
    let mut rng = rand::thread_rng();
    while rng.gen::<f32>() >= 0.5 {
        height += 1;
    }
    height
}

struct SkipListIter<'a, T> {
    curr_node: *mut Node<T>,
    item: &'a T,
    finished: bool,
}

impl<'a, T: PartialEq + PartialOrd + std::fmt::Debug> Iterator for SkipListIter<'a, T> {
    type Item = *mut Node<T>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        unsafe {
            loop {
                match ((*self.curr_node).right, (*self.curr_node).down) {
                    // We're somewhere in the middle of the skiplist, so if `item` is larger than our right,
                    (Some(right), Some(down)) => {
                        match right.as_ref().value.partial_cmp(self.item).unwrap() {
                            // `right` is larger OR equal than `self.item`, so let's go down.
                            Ordering::Greater | Ordering::Equal => {
                                return Some(std::mem::replace(&mut self.curr_node, down.as_ptr()));
                            }
                            // `right` is smaller to `self.item`, so let's go right.
                            Ordering::Less => {
                                self.curr_node = right.as_ptr();
                                // Some(std::mem::replace(&mut self.curr_node, right.as_ptr()))
                            }
                        }
                    }
                    // We're at the bottom of the skiplist
                    (Some(right), None) => {
                        match right.as_ref().value.partial_cmp(self.item).unwrap() {
                            // `right` >= `self.item`, and we're at the bottom, so stop.
                            Ordering::Greater | Ordering::Equal => {
                                self.finished = true;
                                return Some(self.curr_node);
                            }
                            // `right` is smaller than `self.item`, so let's advance right.
                            Ordering::Less => {
                                self.curr_node = right.as_ptr();
                            }
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

impl<T: std::fmt::Debug + PartialEq + PartialOrd + Clone> Default for SkipList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: std::fmt::Debug + PartialEq + PartialOrd + Clone> SkipList<T> {
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
    fn iter<'a>(&'a self, item: &'a T) -> SkipListIter<'a, T> {
        SkipListIter {
            curr_node: self.top_left.as_ptr(),
            item,
            finished: false,
        }
    }

    #[inline]
    pub fn contains(&mut self, item: &T) -> bool {
        unsafe {
            let last_ptr = self.iter(item).last().unwrap();
            &(*last_ptr).value == item
        }
        // unsafe {
        //     let mut curr_node = self.top_left.as_ref();
        //     loop {
        //         if curr_node.value.item_eq(&item) {
        //             return true;
        //         }
        //         // dbg!(curr_node);
        //         match (&(*curr_node).right, &(*curr_node).down) {
        //             (None, None) => return false,
        //             // We can see right, and if it's equal, we're done.
        //             (Some(right), _) if &right.as_ref().value == item => {
        //                 return true;
        //             }
        //             // We can see right, and cannot go down, and item to the right is greater than us.
        //             (Some(right), None) if &right.as_ref().value > item => {
        //                 return false;
        //             }
        //             // We can see right and down, and item to the right is less than us.
        //             (Some(right), Some(down)) if &right.as_ref().value > item => {
        //                 curr_node = down.as_ref();
        //             }
        //             _ => return false,
        //         }
        //     }
        // }
    }

    #[inline]
    fn path_to(&mut self, item: &T) -> Vec<*mut Node<T>> {
        self.iter(item).collect()
        // let mut path: Vec<*mut Node<T>> = Vec::new();
        // unsafe {
        //     let mut curr_node = self.top_left.as_mut() as *mut Node<T>;
        //     loop {
        //         path.push(curr_node);
        //         if &(*curr_node).value == item {
        //             break;
        //         }
        //         match ((*curr_node).right, (*curr_node).down) {
        //             (Some(right), _) if &right.as_ref().value <= item => curr_node = right.as_ptr(),
        //             (Some(right), Some(down)) if &right.as_ref().value > item => {
        //                 curr_node = down.as_ptr();
        //             }
        //             _ => break,
        //         }
        //     }
        // }
        // let mut last_node = *path.last().unwrap();
        // unsafe {
        //     while let Some(down) = (*last_node).down {
        //         path.push(down.as_ptr());
        //         last_node = down.as_ptr();
        //     }
        // }
        // return path;
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
    fn ensure_rows_ordered(&self) {
        let mut left_row = self.top_left;
        let mut curr_node = self.top_left;
        unsafe {
            loop {
                while let Some(right) = curr_node.as_ref().right {
                    // dbg!(&curr_node.as_ref().value);
                    // dbg!(&right.as_ref().value);
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
    }

    pub fn insert(&mut self, item: T) {
        #[cfg(debug_assertions)]
        {
            self.ensure_invariants()
        }

        if self.contains(&item) {
            return;
        }
        let height = get_level();
        // dbg!(height);
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
        // dbg!("--------------------");
        // dbg!(self.height);
        // dbg!(height);
        // dbg!(&item);
        for node in self.path_to(&item).into_iter().rev().take(height as usize) {
            // unsafe {
            //     // dbg!(&*node);
            // }
            let mut new_node = SkipList::make_node(item.clone());
            let node: *mut Node<T> = node;
            unsafe {
                new_node.as_mut().down = node_below_me;
                new_node.as_mut().right = (*node).right;
                (*node).right = Some(new_node);
                node_below_me = Some(new_node);
            }
        }
        // dbg!("--------------------");
        #[cfg(debug_assertions)]
        {
            self.ensure_invariants()
        }
    }
}

// TODO: Not leak memory

#[cfg(test)]
mod tests {
    use crate::SkipList;

    #[test]
    fn insert_no_panic() {
        let mut sl = SkipList::new();
        sl.insert(10);
        // dbg!(&sl);
        sl.insert(30);
        // dbg!(&sl);
        sl.insert(50);
        // dbg!(&sl);
        sl.insert(5);
        // dbg!(&sl);
        sl.insert(0);
        // dbg!(&sl);
        sl.insert(3);
        // dbg!(&sl);
        sl.ensure_invariants();
    }
}
