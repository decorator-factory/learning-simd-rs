#![feature(stmt_expr_attributes)]

use criterion::{
    BenchmarkId,
    Criterion,
    Throughput,
    criterion_group,
    criterion_main,
};
use std::hint::black_box;

use learning_simd::base64::{
    B64Error,
    decoded_len,
    unb64_nonsimd,
    unb64_nonsimd_cachelines,
    unb64_simd_elegant,
};

struct Input<'a> {
    b64_encoded: &'a [u8],
    _decoded: &'a [u8],
}

fn benchmark_decoding(c: &mut Criterion) {
    let inputs = [Input {
        b64_encoded: include_bytes!("../test_data/b64_expected_output.txt"),
        _decoded: include_bytes!("../test_data/b64_input.bin"),
    }];

    type Fun = unsafe fn(&[u8], &mut Vec<u8>) -> Result<(), B64Error>;

    #[rustfmt::skip]
    for (fname, sut) in [
        ("unb64_nonsimd", unb64_nonsimd as Fun),
        ("unb64_nonsimd_cachelines", unb64_nonsimd_cachelines as Fun),
        ("unb64_simd_elegant", unb64_simd_elegant as Fun),
    ] {
        let mut group = c.benchmark_group(fname);
        for (idx, input) in inputs.iter().enumerate() {
            let size = input.b64_encoded.len();
            let label = format!("{}_size={}", idx, size);

            let mut outvec = Vec::with_capacity(decoded_len(size));

            group.throughput(Throughput::Bytes(size as u64));
            group.bench_with_input(BenchmarkId::from_parameter(label), input, |b, inp| {
                b.iter(|| unsafe {
                    outvec.clear();  // should add negligible amount of time
                    sut(black_box(inp.b64_encoded), black_box(&mut outvec)).unwrap()
                });
            });
        }
        group.finish();
    }
}

criterion_group!(benches, benchmark_decoding);
criterion_main!(benches);
