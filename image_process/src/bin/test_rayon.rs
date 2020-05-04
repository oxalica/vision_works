use rayon::prelude::*;

fn main() {
    (0..4).into_par_iter().for_each(|i| println!("Hello {}", i));
}
