// All benchmarks measure the full pipeline: the `.wb` format bytes are built
// in-memory as a `Vec<u8>`, after which the loading
// (transpilation of bytes into instructions) + program execution is measured.
mod vm;

use criterion::{Criterion, criterion_group, criterion_main};

criterion_group! {
    name = vm_benches;
    config = Criterion::default();
    targets =
        vm::ackermann::ackermann,
        vm::binary_trees::binary_trees,
        vm::dense_arithmetic_10k::dense_arithmetic_10k,
        vm::fib_loop_10k::fib_loop_10k,
        vm::fib_loop_100k::fib_loop_100k,
        vm::fib_recursive::fib_recursive,
        vm::mandelbrot::mandelbrot,
        vm::nbody::nbody,
        vm::sieve::sieve,
        vm::spectral_norm::spectral_norm,
        vm::tak::tak,
}
criterion_main!(vm_benches);
