use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::Result;
use byteorder::{BigEndian, WriteBytesExt};
use hyper::{server::conn::Http, service::service_fn, body::Body, Method, Request, Response, StatusCode};
use id3::{frame::Private, Tag, TagLike, Version};
use tokio::{net::TcpListener, time::interval};

use crate::m3u8::HLSPlayback;

struct HLSState {
    hls_playback: HLSPlayback,
    media_playlist: Vec<u8>,
    segment_data: HashMap<String, Vec<u8>>,
    /// MPEG-2 Program Elementary Timestamp
    timestamp: u64,
}

pub struct HLSServer {
    tcp_listener: TcpListener,
    state: HLSState,
}

impl HLSServer {
    pub async fn new(addr: SocketAddr, hls_playback: HLSPlayback) -> std::io::Result<Self> {
        // let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
        let tcp_listener = TcpListener::bind(addr).await?;
        let capacity = hls_playback.config.segments_to_keep;
        Ok(Self {
            tcp_listener,
            state: HLSState {
                hls_playback,
                media_playlist: Vec::new(),
                segment_data: HashMap::with_capacity(capacity),
                timestamp: 0,
            },
        })
    }

    /// Makes data from the callback available over HLS
    /// The amount of data needs to be equal to the segment length specified in HLSConfig
    pub async fn serve_data<F>(self, mut data_callback: F) -> Result<()>
    where
        F: FnMut() -> Vec<u8> + Send + 'static,
    {
        let segment_duration = self.state.hls_playback.config.segment_duration;
        let state = Arc::new(Mutex::new(self.state));

        let state_1 = state.clone();

        // Periodically update the playlist
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs_f32(segment_duration));
            loop {
                interval.tick().await;
                tokio::spawn(Self::update_playlist(state_1.clone(), data_callback()));
            }
        });

        loop {
            let (tcp_stream, _) = self.tcp_listener.accept().await?;

            let state_2 = state.clone();
            let service = service_fn(move |request| serve_playlist(request, state_2.clone()));

            tokio::spawn(async move {
                if let Err(error) = Http::new().serve_connection(tcp_stream, service).await {
                    eprintln!("Error while serving HTTP connection: {}", error);
                }
            });
        }
    }

    /// Adds a new segment to the playlist and updates the playlist and segment data accordingly
    async fn update_playlist(state: Arc<Mutex<HLSState>>, mut new_segment_data: Vec<u8>) {
        let mut segment_available_interval = 0.0;
        let mut removed_segment_name = None;

        if let Ok(state) = &mut state.lock() {
            let (added_segment_name, removed_segment_name_a) = state.hls_playback.add_segment();

            println!("Added {}", &added_segment_name);

            // RFC: 8126 section 3.4
            // The ID3 payload MUST
            // be a 33-bit MPEG-2 Program Elementary Stream timestamp expressed as a
            // big-endian eight-octet number, with the upper 31 bits set to zero.
            let mut timestamp_vec = Vec::with_capacity(8);
            timestamp_vec
                .write_u64::<BigEndian>(state.timestamp)
                .expect("Should not fail");

            // RFC: 8126 section 3.4
            // Each Packed Audio Segment MUST signal the timestamp of its first
            // sample with an ID3 Private frame (PRIV) tag [ID3] at the beginning of
            // the segment.
            let mut id3 = Tag::new();
            id3.add_frame(Private {
                owner_identifier: String::from("com.apple.streaming.transportStreamTimestamp"),
                private_data: timestamp_vec,
            });

            let mut id3_segment_data = Vec::new();
            id3.write_to(&mut id3_segment_data, Version::Id3v23)
                .unwrap();

            id3_segment_data.append(&mut new_segment_data);

            state
                .segment_data
                .insert(added_segment_name, id3_segment_data);

            state.media_playlist = state.hls_playback.generate_playlist();

            // println!(
            //     "{}",
            //     String::from_utf8(state.media_playlist.clone()).unwrap()
            // );

            // RFC: 8126 section 6.2.2
            // When the server removes a Media Segment URI from the Playlist, the
            // corresponding Media Segment MUST remain available to clients for a
            // period of time equal to the duration of the segment plus the duration
            // of the longest Playlist file distributed by the server containing
            // that segment.
            segment_available_interval = (1.0 + state.hls_playback.segments.len() as f32)
                * state.hls_playback.config.segment_duration;

            // Timestamps have a resolution of 90kHz
            state.timestamp += (segment_available_interval * 90000.0) as u64;

            removed_segment_name = removed_segment_name_a;
        };

        if let Some(removed_segment_name) = removed_segment_name {
            let mut interval = interval(Duration::from_secs_f32(segment_available_interval));

            interval.tick().await;

            println!("Removed: {}", &removed_segment_name);

            if let Ok(state) = &mut state.lock() {
                state.segment_data.remove(&removed_segment_name)
                    .expect("Shouldn't fail because if add_segment removed this from the playlist then it must be in the HashMap");
            }
        }
    }
}

/// Service function that serves the playlist and audio segments
async fn serve_playlist(
    request: Request<Body>,
    state: Arc<Mutex<HLSState>>,
) -> hyper::Result<Response<Body>> {
    let uri = request.uri().to_string();

    let state = state.lock().unwrap();
    if request.method() == &Method::GET {
        println!("{}", uri);
        if uri == "/stream.m3u8" {
            return Ok(Response::new(state.media_playlist.clone().into()));
        } else if uri.starts_with("/segment-") {
            return if let Some(segment) = state.segment_data.get(&uri[1..]) {
                Ok(Response::new(segment.clone().into()))
            } else {
                not_found()
            };
        }
    }

    not_found()
}

/// Generic 404 page
fn not_found() -> hyper::Result<Response<Body>> {
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::empty())
        .unwrap())
}
