use buffer::StreamBufferExt;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use futures::stream;
use futures::StreamExt;
use tokio::runtime::Runtime;

async fn batch(size: usize) {
    let upstream = stream::iter(0..size);
    let buffered_stream = upstream.buffer(size / 10);

    let _: Vec<_> = buffered_stream.collect().await;
}

fn criterion_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("buffer");
    for &size in &[10, 100, 1000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &size,
            |bencher, &size| {
                bencher.to_async(&rt).iter(|| batch(size));
            },
        );
    }
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
