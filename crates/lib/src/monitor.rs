use std::{ffi::OsString, io};

use futures_lite::{stream::Map, StreamExt};
use inotify::{EventMask, EventStream, Inotify, WatchDescriptor, WatchMask};

use crate::path::{self, Kind};

pub fn monitor() -> io::Result<Map<EventStream<[u8; 512]>, impl FnMut(io::Result<inotify::Event<OsString>>) -> StateEvent>>  {
    let inotify = Inotify::init()?;

    let runtime_dir = crate::path::runtime_dir();
    let persistent_dir = crate::path::persistent_dir();

    const MASK: WatchMask = WatchMask::CREATE
    .union(WatchMask::OPEN)
    .union(WatchMask::DELETE)
    .union(WatchMask::CLOSE_WRITE)
    .union(WatchMask::MOVED_FROM)
    .union(WatchMask::MOVED_TO)
    .union(WatchMask::MODIFY);

    let mut watches = inotify.watches();

    let runtime_wd = watches.add(&runtime_dir, MASK)?;
    let persistent_wd = watches.add(&persistent_dir, MASK)?;

    let which = move |wd: WatchDescriptor| {
        match wd {
            wd if wd == runtime_wd => Kind::Runtime,
            wd if wd == persistent_wd => Kind::Persistent,
            _ => unreachable!("inotify has recieved an unknown watch descriptor"),
        }
    };

    let stream = inotify.into_event_stream([0; 512])?.map(move |event| {
        let event = event.unwrap();
        let kind = which(event.wd);
        let file_name = event.name.unwrap_or_else(|| match kind {
            Kind::Runtime => runtime_dir.clone().into(),
            Kind::Persistent => persistent_dir.clone().into(),
        });

        let event = match event.mask {
            EventMask::MODIFY => Event::Modified,
            EventMask::CREATE | EventMask::MOVED_TO => Event::New,
            EventMask::DELETE | EventMask::MOVED_FROM => Event::Removed,
            EventMask::CLOSE_WRITE => Event::Closed,
            EventMask::Q_OVERFLOW => {
                panic!("buffer for inotify events is not big enough to handle all events")
            },
            _ => Event::Unknown,
        };

        StateEvent {
            event,
            kind,
            file_name
        }
    });

    Ok(stream)
}

#[derive(Debug)]
pub struct StateEvent {
    pub event: Event,
    pub kind:  path::Kind,
    pub file_name:  OsString
}

#[derive(Debug)]
pub enum Event {
    New,
    Modified,
    Removed,
    Closed,
    Unknown,
}
