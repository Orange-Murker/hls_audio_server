//! A hassle free way to serve audio over HLS.
//!
//! Example:
//! ```
//! use hls_audio_server::m3u8::{HLSConfig, Playlist};
//! use hls_audio_server::server::HLSServer;
//! use std::net::SocketAddr;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let hls_config = HLSConfig {
//!         segments_to_keep: 10,
//!         segment_duration: 8.0,
//!         uri: "http://localhost:3000/".into(),
//!         file_extension: ".aac".into(),
//!     };
//!
//!     let hls_playback = Playlist::new(hls_config);
//!
//!     let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
//!     let hls_server = HLSServer::new(addr, hls_playback).await?;
//!
//!     hls_server
//!         .serve_data(move || {
//!             // Serve your encoded audio here
//!             Vec::new()
//!         })
//!         .await?;
//!     Ok(())
//! }
//! ```
//!
//! The playlist will be available at `http://localhost:3000/stream.m3u8` in this case.
//!
//! It is the user's responsibility to encode audio in the format compatible with RFC: 8216 section
//! 3.4. The appropriate ID3 tag is automatically added, so the encoded audio must have no ID3 tags.
//!
//! But do not worry! You can find a complete example that uses AAC LC coding to serve a .wav file
//! in the project repository in case you do not want to mess with different codecs.

pub mod m3u8;
pub mod playback;
pub mod server;
