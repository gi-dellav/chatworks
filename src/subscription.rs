use std::pin::Pin;
use std::task::{Context, Poll};

use crate::error::Result;
use crate::event::EventEnvelope;

pub trait EventStream: Send {
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>)
    -> Poll<Option<Result<EventEnvelope>>>;

    fn close(self: Pin<&mut Self>) {}
}

pub struct Subscription {
    inner: Pin<Box<dyn EventStream>>,
}

impl Subscription {
    pub fn new<S>(inner: S) -> Self
    where
        S: EventStream + 'static,
    {
        Self {
            inner: Box::pin(inner),
        }
    }

    pub fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<EventEnvelope>>> {
        self.inner.as_mut().poll_next(cx)
    }

    pub async fn recv(&mut self) -> Option<Result<EventEnvelope>> {
        std::future::poll_fn(|cx| self.poll_next(cx)).await
    }

    pub fn close(&mut self) {
        self.inner.as_mut().close();
    }
}
