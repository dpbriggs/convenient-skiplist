# Convenient SkipList

A performant and convenient skiplist, with advanced range queries and serde support.

To add this to your project, simply add the below to your Cargo.toml:

```
convenient-skiplist = "1.0.1"
```

Or if you want `serde` support:

```
convenient-skiplist = { "version" = "1.0.1", features = ["serde_support"] }
```

## Simple Example

```rust
use convenient_skiplist::SkipList;

fn main() {
    // Make a new skiplist
    let mut sk = SkipList::new();
    for i in 0..5u32 {
        // Inserts are O(log(n)) on average
        sk.insert(i);
    }
    // You can print the skiplist!
    dbg!(&sk);
    // You can check if the skiplist contains an element, O(log(n))
    assert!(sk.contains(&0));
    assert!(!sk.contains(&10));
    assert!(sk.remove(&0)); // remove is also O(log(n))
}
```

This outputs:

```
[src/main.rs:11] &sk = SkipList(wall_height: 4), and table:
NegInf -> PosInf
NegInf -> Value(2) -> PosInf
NegInf -> Value(1) -> Value(2) -> PosInf
NegInf -> Value(0) -> Value(1) -> Value(2) -> Value(3) -> Value(4) -> PosInf
```

Scroll down or see the `examples` folder for more information.

## What this library provides

This library provides tools to efficiently construct and iterate over skiplists. Under the hood it's all pointers
and inlined comparison functions. In debug mode there's several invariant checks, which are disabled for performance reasons in release.

### Basic Features

You can construct a skiplist, and insert elements and check if they exist in the skiplist (`contains`);

```rust
// Create a new skiplist
let mut sk = SkipList::new();

// Insert an element
sk.insert(0u32);

// Verify that the element exists in the SkipList
assert!(sk.contains(&0))

// Remove an element from the skiplist:
assert!(sk.remove(&0))

// Check the length
assert_eq!(sk.len(), 0)
assert_eq!(sk.is_empty(), true)

// Find the index of an element
sk.insert(1u32);
sk.insert(2u32);
sk.insert(3u32);

assert_eq!(sk.index_of(&1), Some(0))
assert_eq!(sk.index_of(&2), Some(1))
assert_eq!(sk.index_of(&99), None)
```

### Indexing

Convenient SkipList has several index-based features:

```rust

use convenient_skiplist::SkipList;

let mut sk = SkipList::from((b'a'..=b'z').map(|c| c as char));

// Find the index (rank) of an item
assert_eq!(sk.index_of(&'a'), Some(0));
assert_eq!(sk.index_of(&'b'), Some(1));
assert_eq!(sk.index_of(&'z'), Some(25));
assert_eq!(sk.index_of(&'💩'), None);

// Get the element at index (rank -> value)
assert_eq!(sk.at_index(0), Some(&'a'));
assert_eq!(sk.at_index(25), Some(&'z'));
assert_eq!(sk.at_index(100), None);

// We can also efficiently pop maximum and minimum values:
assert_eq!(vec!['z'], sk.pop_max(1));
assert_eq!(vec!['a', 'b', 'c'], sk.pop_min(3));
```

### Iterators

There's currently three main methods to iterate over a skiplist:

```rust
use convenient_skiplist::SkipList;

// First make a skiplist with plenty of elements:

let sk = SkipList::new();
for i in 0..500u32 {
    sk.insert(i);
}

// SkipList::iter_all -- Iterator over all elements (slow!)

for i in sk.iter_all() {
     println!("{}", i);
}

// SkipList::range -- Fast, typically bounded by range width.

for i in sk.range(&200, &400) {
     println!("{}", i);
}

// SkipList::range_with -- Fast, typically bounded by range width.
// You need to provide a comparison function to guide the
// iterator towards the desired range.

use convenient_skiplist::iter::RangeHint;

let my_range_fn = |&ele| {
    if ele < 111 {
        RangeHint::SmallerThanRange
    } else if ele > 333 {
        RangeHint::LargerThanRange
    } else {
        RangeHint::InRange
    }
};

for i in sk.range_with(my_range_fn) {
     println!("{}", i);
}

```

## Performance

General rule of thumb: Mutate operations are microseconds, immutable nanoseconds.
The main mutation bottleneck is heap allocations and frees.

You can test how `convenient-skiplist` performs for you by using cargo bench:

```bash
$ cargo bench
```

## Time / Space Complexities:

- Skiplists have an expected space complexity of ~`2n`.
- `SkipList::insert` - O(logn) time | ~O(1) space
- `Skiplist::contains` - O(logn) time
- `Skiplist::remove` - O(logn) time
- `Skiplist::iter_all` - O(n) time | O(1) space (iterator yields a single element at a time)
- `Skiplist::range` - O(logn + k), where k is width of range | O(1) space (iterator yields a single element at a time)
- `Skiplist::range_with` - O(logn + k + flogn), where k is width of range, f is cost of function passed | O(1) space (iterator yields a single element at a time)
- `Skiplist::index_of` - O(logn) time
- `Skiplist::at_index` - O(logn) time
- `Skiplist::pop_min` - O(logn * k) time | O(k) space, where k is the number of elements to pop
- `Skiplist::at_index` - O(logn * k) time | O(logn + k) space, where k is the number of elements to pop
- `PartialEq<SkipList>` - O(n) time; compare if two skiplists have the same elements
- `From<FromIterator<T>>` - O(nlogn) time; generating a skiplist from a iterator of `n` items
- `Skiplist::pop_back` - O(log n) time
- `Skiplist::pop_front` - O(1) time

## Data Structure Description

A Skiplist is probabilistic data-structure of ordered elements. It resembles a 2D linked list,
where the bottom most row is a just a normal linked list.

Each row is structured as `"Negative Infinity" -> ... ordered linked list ... -> "Positive Infinity"`. Each element lives in a tower of random height (geometric dist), and you can only traverse down the tower. You can also traverse right as in a normal linked list.

The main idea behind the data-structure is that you start in the top left, and if the element
you're currently seaching with is larger than the element to your right, you skip. This lets you
avoid a lot of work by literally jumping elements you otherwise would have searched.
Otherwise, you head one level down, and try to skip again. Repeat this until you hit the bottom, where
you can advance right like a normal linked list.

An example of a skiplist:

<p align="center">
  <img src="https://upload.wikimedia.org/wikipedia/commons/8/86/Skip_list.svg">
</p>

## More Advanced Example

```rust
use convenient_skiplist::{RangeHint, SkipList};
use std::cmp::Ordering;

#[derive(PartialEq, Debug, Clone)]
struct MoreComplex {
    pub score: f64,
    pub data: String,
}

// We're going to sort the skiplist by the "score" field
impl PartialOrd for MoreComplex {
    fn partial_cmp(&self, other: &MoreComplex) -> Option<Ordering> {
        self.score.partial_cmp(&other.score)
    }
}

fn main() {
    let mut sk = SkipList::new();
    for i in 0..100 {
        sk.insert(MoreComplex {
            score: i as f64 / 100.0,
            data: i.to_string(),
        });
    }
    let range = sk.range_with(|ele| {
        if ele.score <= 0.05 {
            RangeHint::SmallerThanRange
        } else if ele.score <= 0.55 {
            RangeHint::InRange
        } else {
            RangeHint::LargerThanRange
        }
    });
    for item in range {
        println!("{:?}", item);
    }
}

```


## Soundness

This library uses a _lot_ of unsafe. It's closer to a cpp library that happens to be in rust.
In particular, I wouldn't stress the iterator stuff. I do some interesting things with iterators to make
the lifetimes work properly.

But `miri` seems to like it, so 🤷
