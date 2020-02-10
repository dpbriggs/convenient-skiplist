/// Run this example with cargo run --example simple
use convenient_skiplist::SkipList;

fn main() {
    let mut sk = SkipList::from(0..3);

    // print the skiplist
    // warning: this can print a lot of nodes (~ 2 * sk.len())
    println!("{:?}", sk);

    // Test association
    if sk.contains(&0) {
        println!("It contains 0!");
    }
    if !sk.contains(&99) {
        println!("It doesn't contain 99 :C");
    }
    // Insert and remove elements
    if sk.insert(99) {
        println!("... it now contains 99 🎉");
    }
    // Elements are unique
    if !sk.insert(99) {
        println!("... can't insert 99 twice :c");
    }

    if sk.remove(&99) {
        println!("... I removed 99");
    }

    // Pop items
    sk.insert(100);
    sk.insert(200);
    dbg!(sk.pop_max(2));
    dbg!(sk.pop_min(2));

    // We can check how many elements are in the skiplist

    dbg!(sk.len(), sk.is_empty());

    // Let's make a big skiplist
    let sk = SkipList::from(0..1000);

    // Lets iterate over all of them
    let all_eles: Vec<_> = sk.iter_all().collect();
    dbg!((all_eles.len(), sk.len()));

    // Lets iterate over a range
    dbg!(sk.range(&700, &705).collect::<Vec<_>>());
}
