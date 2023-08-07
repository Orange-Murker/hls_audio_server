use m3u8::{get_playlist, PlaybackState};
use std::{str, fs::File, collections::VecDeque};

mod server;
mod m3u8;
mod encode_audio;

fn main() {
    let playback_state = PlaybackState {
        media_sequence: 0,
        segments: VecDeque::new(),
    };

    let play = get_playlist(&playback_state);

    let play_string = str::from_utf8(play.as_slice()).unwrap();
    println!("{}", play_string);

    let mut file = File::open("sine.wav").unwrap();

    let (header, data) = wav::read(&mut file).unwrap();

    println!("{:?}", header);

    let encoded = encode_audio::aac_encode(data.as_sixteen().unwrap());
}
