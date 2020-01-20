# Convenient SkipList

A performant and convenient skiplist, with advanced range queries and serde support.

To add this to your project, simply add the below to your Cargo.toml:

```
convenient_skiplist = "0.1.0"
```

Or if you want `serde` support:

```
convenient_skiplist = { "version" = "0.1.0", features = ["serde_support"] } 
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

## What this library provides

This library provides tools to efficiently construct and iterate over skiplists. Under the hood it's all pointers
and inlined comparison functions. In debug mode there's several invariant checks, which are disabled for performance reasons in release.

### Basic Features

You can construct a skiplist, and insert elements and check if they exist in the skiplist (`contains`);

```rust
let sk = SkipList::new()
sk.insert(0u32);
assert!(sk.contains(&0))
```

### Iterators

There's currently three main methods to iterate over a skiplist:

```rust
use convenient_skiplist::SkipList;

// First make a skiplist with plenty of elements:

let sk = SkipList::new()
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

## Time / Space Complexities:

- Skiplists have an expected space complexity of ~`2n`.
- `SkipList::insert` - O(logn) time | ~O(1) space
- `Skiplist::contains` - O(logn) time
- `Skiplist::remove` - O(logn) time
- `Skiplist::iter_all` - O(n) time | O(1) space (iterator yields a single element at a time)
- `Skiplist::range` - O(logn + k), where k is width of range | O(1) space (iterator yields a single element at a time)
- `Skiplist::range_with` - O(logn + k), where k is width of range | O(1) space (iterator yields a single element at a time)
- `PartialEq<SkipList>` - O(n) time; compare if two skiplists have the same elements
- `From<Vec<T>>` - O(nlogn) time; generating a skiplist from a vec of items

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


## Soundness

This library uses a _lot_ of unsafe. It's closer to a cpp library that happens to be in rust.
In particular, I wouldn't stress the iterator stuff. I do some interesting things with iterators to make
the lifetimes work properly.

But `miri` seems to like it, so 🤷
