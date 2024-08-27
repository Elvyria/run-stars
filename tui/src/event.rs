use std::{task::Poll, time::Duration};

use futures_lite::Stream;
use ratatui::crossterm;

pub struct EventStream;

impl Stream for EventStream {
    type Item = crossterm::event::Event;

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        match crossterm::event::poll(Duration::ZERO) {
            Ok(true)  => Poll::Ready(crossterm::event::read().ok()),
            Ok(false) => Poll::Pending,
            Err(_)    => Poll::Ready(None),
        }
    }
}
