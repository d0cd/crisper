use crate::cmd::{Parse, ParseError};
use crate::{Connection, Db, Frame};

use bytes::Bytes;
use std::time::Duration;
use tracing::{debug, instrument};

/// Set `key` to hold the string `value`.
///
/// If `key` already holds a value, it is overwritten, regardless of its type.
/// Any previous time to live associated with the key is discarded on successful
/// SET operation.
#[derive(Debug)]
pub struct Set {
    /// the lookup key
    key: String,

    /// the value to be stored
    value: Bytes,
}

impl Set {
    /// Create a new `Set` command which sets `key` to `value`.
    pub fn new(key: impl ToString, value: Bytes) -> Set {
        Set {
            key: key.to_string(),
            value,
        }
    }

    /// Get the key
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Get the value
    pub fn value(&self) -> &Bytes {
        &self.value
    }


    /// Parse a `Set` instance from a received frame.
    ///
    /// The `Parse` argument provides a cursor-like API to read fields from the
    /// `Frame`. At this point, the entire frame has already been received from
    /// the socket.
    ///
    /// The `SET` string has already been consumed.
    ///
    /// # Returns
    ///
    /// Returns the `Set` value on success. If the frame is malformed, `Err` is
    /// returned.
    ///
    /// # Format
    ///
    /// Expects an array frame containing at least 3 entries.
    ///
    /// ```text
    /// SET key value [EX seconds|PX milliseconds]
    /// ```
    pub(crate) fn parse_frames(parse: &mut Parse) -> crate::Result<Set> {
        use ParseError::EndOfStream;

        // Read the key to set. This is a required field
        let key = parse.next_string()?;

        // Read the value to set. This is a required field.
        let value = parse.next_bytes()?;

        // Attempt to parse another string.
        match parse.next_string() {
            // Currently, mini-redis does not support any of the other SET
            // options. An error here results in the connection being
            // terminated. Other connections will continue to operate normally.
            Ok(_) => return Err("currently `SET` supports no options".into()),
            // The `EndOfStream` error indicates there is no further data to
            // parse. In this case, it is a normal run time situation and
            // indicates there are no specified `SET` options.
            Err(EndOfStream) => {}
            // All other errors are bubbled up, resulting in the connection
            // being terminated.
            Err(err) => return Err(err.into()),
        }

        Ok(Set { key, value })
    }

    /// Converts the command into an equivalent `Frame`.
    ///
    /// This is called by the client when encoding a `Set` command to send to
    /// the server.
    pub(crate) fn into_frame(self) -> Frame {
        let mut frame = Frame::array();
        frame.push_bulk(Bytes::from("set".as_bytes()));
        frame.push_bulk(Bytes::from(self.key.into_bytes()));
        frame.push_bulk(self.value);
        frame
    }
}
