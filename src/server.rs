//! HTTP server for the playlist and the corresponding segments.

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::Result;
use hyper::{server::conn::Http, service::service_fn, body::Body, Method, Request, Response, StatusCode};
use tokio::{net::TcpListener, time::interval};

use crate::{m3u8::Playlist, playback::{update_playlist, HLSState}};


/// HLS Server implementation based on hyper.
pub struct HLSServer {
    tcp_listener: TcpListener,
    state: HLSState,
}

impl HLSServer {
    /// Creates a new server with the given address and Playlist.
    pub async fn new(addr: SocketAddr, playlist: Playlist) -> std::io::Result<Self> {
        let tcp_listener = TcpListener::bind(addr).await?;
        let capacity = playlist.config.segments_to_keep;
        Ok(Self {
            tcp_listener,
            state: HLSState {
                playlist,
                media_playlist: Vec::new(),
                segment_data: HashMap::with_capacity(capacity),
                timestamp: 0,
            },
        })
    }

    /// Makes the audio from the callback available over HLS.
    ///
    /// To guarantee smooth playback the length of the audio clip needs to be equal
    /// to the segment length specified in HLSConfig.
    ///
    /// The callback is called at the same rate as the length of the clip.
    pub async fn serve_data<F>(self, mut data_callback: F) -> Result<()>
    where
        F: FnMut() -> Vec<u8> + Send + 'static,
    {
        let segment_duration = self.state.playlist.config.segment_duration;
        let state = Arc::new(Mutex::new(self.state));

        let state_1 = state.clone();

        // Periodically update the playlist
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs_f32(segment_duration));
            loop {
                interval.tick().await;
                tokio::spawn(update_playlist(state_1.clone(), data_callback()));
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

}

/// Service function that serves the playlist and audio segments.
async fn serve_playlist(
    request: Request<Body>,
    state: Arc<Mutex<HLSState>>,
) -> hyper::Result<Response<Body>> {
    let uri = request.uri().to_string();

    let state = state.lock().expect("The server could not acquire a mutex on the state");
    if request.method() == Method::GET {
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

/// Generic 404 page.
fn not_found() -> hyper::Result<Response<Body>> {
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::empty())
        .unwrap())
}
