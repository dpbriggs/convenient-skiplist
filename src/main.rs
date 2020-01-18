use convenient_skiplist::SkipList;
fn main() {
    let mut sk = SkipList::new();
    dbg!(&sk);
    sk.insert(0u32);
    dbg!(&sk);
    sk.insert(1u32);
    dbg!(&sk);

    // dbg!(sk);
    // handle.write_all(format!("{:?}", sk).as_bytes());
    // dbg!(&sk);
    // sk.insert(1u32);
    // dbg!(&sk);
    // sk.insert(2u32);
    // dbg!(&sk);
    // sk.insert(3u32);
    // dbg!(&sk);
    // sk.insert(4u32);
}
