#[cfg(test)]
#[macro_use]
extern crate doc_comment;

#[cfg(test)]
doctest!("../README.md");

use futures::task::{Context, Poll};
use futures::Stream;
use futures::StreamExt;
use pin_project_lite::pin_project;
use std::pin::Pin;
use tokio::sync::mpsc;

pin_project! {
    pub struct BufferedStream<S>
    where
        S: Stream,
    {
        #[pin]
        receiver: mpsc::Receiver<S::Item>,
    }
}

impl<S> BufferedStream<S>
where
    S: Stream + Send + 'static,
    S::Item: Send + 'static,
{
    fn new(stream: S, buffer_size: usize) -> Self {
        let (sender, receiver) = mpsc::channel(buffer_size);
        let mut stream = Box::pin(stream);

        tokio::spawn(async move {
            while let Some(item) = stream.as_mut().next().await {
                if sender.send(item).await.is_err() {
                    break; // Receiver dropped
                }
            }
        });

        BufferedStream { receiver }
    }
}

impl<S> Stream for BufferedStream<S>
where
    S: Stream + Send + 'static,
    S::Item: Send + 'static,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        Pin::new(&mut *this.receiver).poll_recv(cx)
    }
}

pub trait StreamBufferExt: Stream {
    fn buffer(self, buffer_size: usize) -> BufferedStream<Self>
    where
        Self: Sized + Send + 'static,
        Self::Item: Send + 'static,
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
