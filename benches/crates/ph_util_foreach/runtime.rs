#![allow(missing_docs, clippy::must_use_candidate)]

use ph_util_foreach::impl_foreach;

impl_foreach!(range = 0..64, body = {
    pub fn step + #() -> u64 {
        #
    }
});

fn total() -> u64 {
    let mut sum = 0u64;
    impl_foreach!(range = 0..64, body = {
        sum = sum.wrapping_add(step + #());
    });
    sum
}

fn main() {
    let start = std::time::Instant::now();
    let mut acc = 0u64;
    for _ in 0..1_000_000 {
        acc = acc.wrapping_add(std::hint::black_box(total()));
    }
    std::hint::black_box(acc);
    println!("runtime_seconds {:.6}", start.elapsed().as_secs_f64());
}
