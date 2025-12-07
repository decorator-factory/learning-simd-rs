use criterion::{
    BenchmarkId,
    Criterion,
    Throughput,
    criterion_group,
    criterion_main,
};
use std::hint::black_box;

use learning_simd::count_spaces::{
    count_spaces_simd_novec,
    count_spaces_simd_novec_x8,
    count_spaces_simd_portable_256,
    count_spaces_simd_portable_512,
    count_spaces_std,
    pad_bytes,
};

fn size_for_trhuput(s: &[u8]) -> usize {
    s.iter().position(|c| *c == b'\n').unwrap()
}

fn criterion_benchmark(c: &mut Criterion) {
    let inputs: [&[u8]; _] = [
        b"1234 567 8123456 78 1234 567 8123456 789 12\n",
        include_bytes!("../test_data/count_spaces_long_input.txt"),
    ];
    let inputs = inputs.map(|b| (size_for_trhuput(b), pad_bytes(b)));

    type Fn = unsafe fn(&[u8]) -> u16;

    for (fname, sut) in [
        ("count_spaces_std", count_spaces_std as Fn),
        ("count_spaces_simd_novec", count_spaces_simd_novec as Fn),
        ("count_spaces_simd_novec_x8", count_spaces_simd_novec_x8 as Fn),
        ("count_spaces_simd_portable_256", count_spaces_simd_portable_256 as Fn),
        ("count_spaces_simd_portable_512", count_spaces_simd_portable_512 as Fn),
    ] {
        let mut group = c.benchmark_group(fname);
        for (idx, &(size, input)) in inputs.iter().enumerate() {
            let label = format!("{idx}_size={size}");
            group.throughput(Throughput::Bytes(size as u64));
            group.bench_with_input(BenchmarkId::from_parameter(label), input, |b, input| {
                b.iter(|| unsafe { sut(black_box(input)) });
            });
        }
        group.finish();
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
