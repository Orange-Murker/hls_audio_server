use std::collections::VecDeque;

use chrono::Utc;
use m3u8_rs::{MediaPlaylist, MediaSegment};

const SEGMENTS_TO_KEEP: usize = 10;

pub struct PlaybackState {
    pub media_sequence: u64,
    pub segments: VecDeque<MediaSegment>,
}

/// Add a new segment to the playlist and remove an old one if necessary
/// Returns the name of just added segment
pub fn add_segment(playback_state: &mut PlaybackState) -> String {
    if playback_state.segments.len() >= SEGMENTS_TO_KEEP {
        playback_state.segments.pop_front().unwrap();
    }

    let timestamp = Utc::now().timestamp_millis();

    let name = String::from("segment-") + &timestamp.to_string();

    let segment = MediaSegment {
        uri: name.clone(),
        duration: 0.0,
        title: None,
        byte_range: None,
        discontinuity: false,
        key: None,
        map: None,
        program_date_time: None,
        daterange: None,
        unknown_tags: Vec::new(),
    };
    playback_state.segments.push_back(segment);

    playback_state.media_sequence += 1;

    name
}

/// Generate a playlist based on the playback state
pub fn get_playlist(playback_state: &PlaybackState) -> Vec<u8> {
    let playlist = MediaPlaylist {
        version: Some(6),
        target_duration: 6.0,
        media_sequence: playback_state.media_sequence,
        segments: playback_state.segments.clone().into(),
        discontinuity_sequence: 0,
        end_list: false,
        playlist_type: None,
        i_frames_only: false,
        start: None,
        independent_segments: false,
        unknown_tags: Vec::new(),
    };

    let mut play_vec: Vec<u8> = Vec::new();
    playlist.write_to(&mut play_vec).unwrap();

    play_vec
}
