use std::path::Path;

use crate::{error::Error, init_persistent_dir, init_runtime_dir, parse, State};

/// Read runtime dir
/// Read persistent dir
/// Check mod times
/// return new one
pub fn read(p: impl AsRef<Path> + Copy) -> Result<State, Error> {
    let absolute = std::path::absolute(p).map_err(|io| Error::Absolute { path: p.as_ref().to_owned(), io })?;
    let encoded = crate::fmt::encode_path(absolute);

    let mut runtime = init_runtime_dir()?;
    runtime.push(encoded);

    let r_meta = match std::fs::metadata(runtime) {
        Ok(meta) => {
            if meta.is_file() {
                panic!("not a file");
            }
        },
        Err(_) => {
            panic!("AAA");
        },
    };

    Ok(State { file_name: ":C".into() })

    // match read_runtime(&encoded) {
    //     Ok(runtime) => return Ok(runtime),
    //     Err(Error::NotFound(_)) => {},
    //     Err(e) => {
    //         // TODO: LOG
    //         eprintln!("{e}");
    //     }
    // }

    // read_persistent(encoded)
    // match read_persistent(encoded)? {
    //     Ok(state) => return Ok(state),
    //     Err(Error::NotFound())
    // }
}

// fn read_persistent(encoded: impl AsRef<Path>) -> Result<State, Error> {
//     let mut persistent = init_persistent_dir()?;
//     persistent.push(&encoded);

//     let tasks = parse(&persistent)?;

//     Ok(State {
//         tasks,
//         name: persistent,
//         live: false,
//     })
// }

// fn read_runtime(encoded: impl AsRef<Path>) -> Result<State, Error> {
//     let mut runtime = init_runtime_dir()?;
//     runtime.push(encoded);

//     let tasks = parse(&runtime)?;

//     Ok(State { 
//         tasks,
//         name: runtime,
//         live: true,
//     })
// }
