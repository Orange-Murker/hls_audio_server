use m3u8::{HLSConfig, HLSPlayback};
use server::HLSServer;
use std::{fs::File, net::SocketAddr};

mod encode_audio;
mod m3u8;
mod server;

#[tokio::main]
async fn main() {
    let mut file = File::open("test.wav").unwrap();
    let (_header, data) = wav::read(&mut file).unwrap();
    let data: Vec<Vec<i16>> = data
        .as_sixteen()
        .unwrap()
        .chunks(441000)
        .map(|chunk| Vec::from(chunk))
        .collect();
    let mut current_chunk = 0;

    let hls_config = HLSConfig {
        segments_to_keep: 10,
        segment_duration: 5.0,
        uri: "http://192.168.8.6:3000/".into(),
    };

    let hls_playback = HLSPlayback::new(hls_config);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let hls_server = HLSServer::new(addr, hls_playback).await.unwrap();

    hls_server
        .serve_data(move || {
            let current_chunk = &mut current_chunk;
            if *current_chunk == data.len() {
                *current_chunk = 0;
            }

            let encoded = encode_audio::aac_encode(data[*current_chunk].as_slice());

            *current_chunk += 1;

            encoded
        })
        .await
        .unwrap();
}
