#[cfg(test)]
#[macro_use]
extern crate doc_comment;

#[cfg(test)]
doctest!("../README.md");

use futures::task::{Context, Poll};
use futures::Stream;
use futures::StreamExt;
use std::pin::Pin;
use tokio::sync::mpsc;

pub struct BufferedStream<S>
where
    S: Stream,
{
    receiver: mpsc::Receiver<S::Item>,
    _task: tokio::task::JoinHandle<()>,
}

impl<S> BufferedStream<S>
where
    S: Stream + Send + 'static,
    S::Item: Send + 'static,
{
    fn new(upstream: S, buffer_size: usize) -> Self {
        let (tx, rx) = mpsc::channel(buffer_size);

        let task = tokio::spawn(async move {
            tokio::pin!(upstream);

            while let Some(item) = upstream.next().await {
                if tx.send(item).await.is_err() {
                    break; // If the receiver is dropped, stop filling the buffer
                }
            }
        });

        BufferedStream {
            receiver: rx,
            _task: task,
        }
    }
}

impl<S> Stream for BufferedStream<S>
where
    S: Stream + Unpin,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut receiver = Pin::new(&mut self.get_mut().receiver);
        receiver.poll_recv(cx)
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
    use tokio_stream::iter;

    #[tokio::test]
    async fn test_empty_stream() {
        let upstream = iter(Vec::<i32>::new());
        let buffered_stream = upstream.buffer(10);

        let collected: Vec<i32> = buffered_stream.collect().await;
        assert_eq!(collected.len(), 0);
    }

    #[tokio::test]
    async fn test_single_element_stream() {
        let upstream = iter(vec![42]);
        let buffered_stream = upstream.buffer(10);

        let collected: Vec<i32> = buffered_stream.collect().await;
        assert_eq!(collected, vec![42]);
    }

    #[tokio::test]
    async fn test_multiple_elements_stream() {
        let upstream = iter(vec![1, 2, 3, 4, 5]);
        let buffered_stream = upstream.buffer(10);

        let collected: Vec<i32> = buffered_stream.collect().await;
        assert_eq!(collected, vec![1, 2, 3, 4, 5]);
    }
}
