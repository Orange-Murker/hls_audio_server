use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::Result;
use hyper::{server::conn::Http, service::service_fn, Body, Method, Request, Response, StatusCode};
use tokio::{net::TcpListener, time::interval};

use crate::m3u8::HLSPlayback;

struct HLSState {
    hls_playback: HLSPlayback,
    media_playlist: Vec<u8>,
    segment_data: HashMap<String, Vec<u8>>,
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
            },
        })
    }

    /// Makes data from the callback available over HLS
    /// The amount of data needs to be equal to the segment length specified in HLSConfig
    pub async fn serve_data<F>(self, mut data_callback: F) -> Result<()>
    where
        F: FnMut() -> Vec<u8> + Send + 'static,
    {
        let state = Arc::new(Mutex::new(self.state));

        let state_1 = state.clone();

        tokio::spawn(async move {
            // Hardcoded for now
            let mut interval = interval(Duration::from_secs_f32(5.0));
            loop {
                tokio::spawn(Self::update_playlist(state_1.clone(), data_callback()));
                interval.tick().await;
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
    async fn update_playlist(state: Arc<Mutex<HLSState>>, new_segment_data: Vec<u8>) {
        let mut segment_available_interval = 0.0;
        let mut removed_segment_name = None;
        if let Ok(state) = &mut state.lock() {
            let (added_segment_name, removed_segment_name_a) = state.hls_playback.add_segment();

            state
                .segment_data
                .insert(added_segment_name, new_segment_data);

            state.media_playlist = state.hls_playback.generate_playlist();

            segment_available_interval = state.hls_playback.config.segment_duration;
            removed_segment_name = removed_segment_name_a;
        };

        if let Some(removed_segment_name) = removed_segment_name {
            // RFC: 8126 section 6.2.2
            // When the server removes a Media Segment URI from the Playlist, the
            // corresponding Media Segment MUST remain available to clients for a
            // period of time equal to the duration of the segment plus the duration
            // of the longest Playlist file distributed by the server containing
            // that segment.
            let mut interval = interval(Duration::from_secs_f32(segment_available_interval));

            interval.tick().await;

            // Shouldn't fail because if add_segment removed this from the playlist then it must be
            // in the HashMap

            if let Ok(state) = &mut state.lock() {
                state.segment_data.remove(&removed_segment_name).unwrap();
            }
        }
    }
}

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

fn not_found() -> hyper::Result<Response<Body>> {
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::empty())
        .unwrap())
}
