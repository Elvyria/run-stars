use std::{ffi::OsString, io};

use futures_lite::{stream::Map, StreamExt};
use inotify::{EventMask, EventStream, Inotify, WatchDescriptor, WatchMask};

use crate::path::{self, Kind};

pub fn monitor() -> io::Result<Map<EventStream<[u8; 512]>, impl FnMut(io::Result<inotify::Event<OsString>>) -> StateEvent>>  {
    let inotify = Inotify::init()?;

    let runtime_dir = crate::path::runtime_dir();
    let persistent_dir = crate::path::persistent_dir();

    const MASK: WatchMask = WatchMask::CREATE
    .union(WatchMask::DELETE)
    // .union(WatchMask::CLOSE_WRITE)
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

    // let full_path = move |event: Event<OsString>| {
    //     let name = event.name.expect("watch mask doesn't include flags for directories");

    //     match event.wd {
    //         wd if wd == runtime_wd => runtime_dir.join(name),
    //         wd if wd == persistent_wd => persistent_dir.join(name),
    //         _ => unreachable!("inotify has recieved an unknown watch descriptor"),
    //     }
    // };

    let stream = inotify.into_event_stream([0; 512])?.map(move |event| {
        let event = event.unwrap();

        return match event.mask {
            EventMask::MODIFY => StateEvent {
                event: Event::Modified,
                kind: which(event.wd),
                file_name: event.name.expect("watch mask doesn't include flags for directories"),
            },
            EventMask::CREATE | EventMask::MOVED_TO => StateEvent {
                event: Event::New,
                kind: which(event.wd),
                file_name: event.name.expect("watch mask doesn't include flags for directories"),
            },
            EventMask::DELETE | EventMask::MOVED_FROM => StateEvent {
                event: Event::Removed,
                kind: which(event.wd),
                file_name: event.name.expect("watch mask doesn't include flags for directories"),
            },
            EventMask::Q_OVERFLOW => {
                // TODO: Logging
                panic!("buffer for inotify events is not big enough.")
            }
            _ => unreachable!("inotify got an unknown event ({event:?})")
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
}
