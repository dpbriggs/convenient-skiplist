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
