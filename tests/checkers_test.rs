use convenient_skiplist::{iter::RangeHint, SkipList};

#[global_allocator]
static ALLOCATOR: checkers::Allocator = checkers::Allocator::system();

#[checkers::test]
fn test_allocations() {
    let mut sk = SkipList::new();
    let _: Vec<u32> = sk.iter_all().cloned().collect();
    let _: Vec<u32> = sk.range(&10, &20).cloned().collect();
    let _: Vec<u32> = sk.range(&10, &20).cloned().collect();
    let _: Vec<u32> = sk
        .range_with(|&i| {
            if i < 2 {
                RangeHint::SmallerThanRange
            } else if i > 10 {
                RangeHint::LargerThanRange
            } else {
                RangeHint::InRange
            }
        })
        .cloned()
        .collect();

    for i in 0..50u32 {
        sk.insert(i);
    }
    sk.contains(&13);
    let _: Vec<u32> = sk.iter_all().cloned().collect();
    let _: Vec<u32> = sk.range(&10, &20).cloned().collect();
    let _: Vec<u32> = sk
        .range_with(|&i| {
            if i < 2 {
                RangeHint::SmallerThanRange
            } else if i > 10 {
                RangeHint::LargerThanRange
            } else {
                RangeHint::InRange
            }
        })
        .cloned()
        .collect();
}
