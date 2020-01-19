# Convenient SkipList

A performant and convenient skiplist, with advanced range queries and serde support.

Still under construction.

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

## Data Structure Description

A Skiplist is probabilistic data-structure of ordered elements. It resembles a 2D linked list,
where the bottom most row is a just a normal linked list.

Each row is structured as `"Negative Infinity" -> ... ordered linked list ... -> "Positive Infinity"`. Each element lives in a tower of random height (binomial dist), and you can only traverse down the tower.

The main idea behind the data-structure is that you start in the top left, and if the element
you're currently seaching with is larger than the element to your right, you skip. Otherwise,
you head one level down, and try to skip again. Repeat this until you hit the bottom, where
you can advance right like a normal linked list.

An example of a skiplist:

![Skiplist Picture](https://upload.wikimedia.org/wikipedia/commons/8/86/Skip_list.svg)
