use m3u8::{HLSConfig, HLSPlayback};
use server::HLSServer;
use std::{fs::File, net::SocketAddr};

mod encode_audio;
mod m3u8;
mod server;

#[tokio::main]
async fn main() {
    let mut file = File::open("test48.wav").unwrap();
    let (_header, data) = wav::read(&mut file).unwrap();

    // The encoder takes frames of 2048 samples in case of 2 channels
    // We can make sure that each chunk is a multiple of the frame and the sample rate
    // This is to get an integer segment duration (doesn't have to be an integer but it is nice)
    // lcm(2048, 96000) = 768000
    // 768000 / 96000 = 8 seconds of audio
    let data: Vec<Vec<i16>> = data
        .as_sixteen()
        .unwrap()
        .chunks(768000)
        .map(|chunk| Vec::from(chunk))
        .collect();
    let mut current_chunk = 0;

    let hls_config = HLSConfig {
        segments_to_keep: 10,
        segment_duration: 8.0,
        uri: "http://192.168.8.6:3000/".into(),
        file_extension: ".aac".into(),
    };

    let hls_playback = HLSPlayback::new(hls_config);

    let addr = SocketAddr::from(([0, 0, 0, 0], 4000));
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
