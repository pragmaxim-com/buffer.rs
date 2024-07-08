## buffer

![Build status](https://github.com/pragmaxim-com/buffer.rs/workflows/Rust/badge.svg)
[![Documentation](https://docs.rs/buffer/badge.svg)](https://docs.rs/buffer)

A stream adapter that buffers elements using `tokio::sync::mpsc` to improve performance
when downstream is busy (ie. db compactions). In comparison to `buffered` it works not only
with Futures but any element type.

## Usage

```rust
use futures::stream;
use buffer::StreamBufferExt;
use futures::StreamExt;

async fn slow_cpu_heavy_operation(x: i32) -> i32 {
    // Simulate a CPU-heavy operation with a delay
    sleep(Duration::from_millis(500)).await;
    x
}

async fn mock_db_insert(x: i32) {
    // Simulate a database insert operation that can become slow during IO bounded compactions
    sleep(Duration::from_millis(100)).await;
}


let collected: Vec<i32> = 
    stream::iter(0..100_000)
        .map(|x| slow_cpu_heavy_operation(x))
        .buffer(1000)
        .map(|x| mock_db_insert(x))
        .collect().await;
```
