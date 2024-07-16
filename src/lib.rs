#[cfg(test)]
#[macro_use]
extern crate doc_comment;

#[cfg(test)]
doctest!("../README.md");

use futures::task::{Context, Poll};
use futures::Stream;
use pin_project_lite::pin_project;
use std::collections::VecDeque;
use std::pin::Pin;

pin_project! {
    pub struct BufferedStream<S>
    where
        S: Stream,
    {
        #[pin]
        stream: S,
        buffer: VecDeque<S::Item>,
        buffer_size: usize,
    }
}

impl<S> BufferedStream<S>
where
    S: Stream,
{
    fn new(stream: S, buffer_size: usize) -> Self {
        BufferedStream {
            stream,
            buffer: VecDeque::with_capacity(buffer_size),
            buffer_size,
        }
    }
}

impl<S> Stream for BufferedStream<S>
where
    S: Stream,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        // Try to fill the buffer if it's not full
        while this.buffer.len() < *this.buffer_size {
            match this.stream.as_mut().poll_next(cx) {
                Poll::Ready(Some(item)) => this.buffer.push_back(item),
                Poll::Ready(None) => break,
                Poll::Pending => break,
            }
        }

        // Return the next item from the buffer if available
        if let Some(item) = this.buffer.pop_front() {
            Poll::Ready(Some(item))
        } else {
            match this.stream.as_mut().poll_next(cx) {
                Poll::Ready(Some(item)) => Poll::Ready(Some(item)),
                Poll::Ready(None) => Poll::Ready(None),
                Poll::Pending => Poll::Pending,
            }
        }
    }
}

pub trait StreamBufferExt: Stream {
    fn buffer(self, buffer_size: usize) -> BufferedStream<Self>
    where
        Self: Sized,
    {
        BufferedStream::new(self, buffer_size)
    }
}

impl<T: ?Sized> StreamBufferExt for T where T: Stream {}

// Tests
#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_empty_stream() {
        let upstream = tokio_stream::iter(Vec::<i32>::new());
        let buffered_stream = upstream.buffer(10);

        let collected: Vec<i32> = buffered_stream.collect().await;
        assert_eq!(collected.len(), 0);
    }

    #[tokio::test]
    async fn test_single_element_stream() {
        let upstream = tokio_stream::iter(vec![42]);
        let buffered_stream = upstream.buffer(10);

        let collected: Vec<i32> = buffered_stream.collect().await;
        assert_eq!(collected, vec![42]);
    }

    #[tokio::test]
    async fn test_multiple_elements_stream() {
        let upstream = tokio_stream::iter(vec![1, 2, 3, 4, 5]);
        let buffered_stream = upstream.buffer(10);

        let collected: Vec<i32> = buffered_stream.collect().await;
        assert_eq!(collected, vec![1, 2, 3, 4, 5]);
    }

    #[tokio::test]
    async fn test_buffer_overflow() {
        let upstream = tokio_stream::iter(1..=100); // 100 elements
        let buffer_capacity = 10;
        let buffered_stream = upstream.buffer(buffer_capacity);

        let collected: Vec<i32> = buffered_stream.collect().await;
        assert_eq!(collected, (1..=100).collect::<Vec<i32>>());
    }
}
