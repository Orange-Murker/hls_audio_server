//! Handles updating the segment data and removing old segments.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use byteorder::{BigEndian, WriteBytesExt};
use id3::{frame::Private, Tag, TagLike, Version};
use tokio::time::interval;

use crate::m3u8::Playlist;

/// The main state that holds all playback related data.
pub struct HLSState {
    pub playlist: Playlist,
    pub media_playlist: Vec<u8>,
    pub segment_data: HashMap<String, Vec<u8>>,
    /// MPEG-2 Program Elementary Timestamp.
    pub timestamp: u64,
}

/// Adds a new segment to the playlist and updates the playlist and segment data accordingly.
pub async fn update_playlist(state: Arc<Mutex<HLSState>>, mut new_segment_data: Vec<u8>) {
    let mut segment_available_interval = 0.0;
    let mut removed_segment_name = None;

    if let Ok(state) = &mut state.lock() {
        let (added_segment_name, removed_segment_name_a) = state.playlist.add_segment();

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

        state.media_playlist = state.playlist.generate_playlist();

        // RFC: 8126 section 6.2.2
        // When the server removes a Media Segment URI from the Playlist, the
        // corresponding Media Segment MUST remain available to clients for a
        // period of time equal to the duration of the segment plus the duration
        // of the longest Playlist file distributed by the server containing
        // that segment.
        segment_available_interval =
            (1.0 + state.playlist.segments.len() as f32) * state.playlist.config.segment_duration;

        // Timestamps have a resolution of 90kHz
        state.timestamp += (segment_available_interval * 90000.0) as u64;

        removed_segment_name = removed_segment_name_a;
    };

    if let Some(removed_segment_name) = removed_segment_name {
        let mut interval = interval(Duration::from_secs_f32(segment_available_interval));

        interval.tick().await;

        if let Ok(state) = &mut state.lock() {
            state.segment_data.remove(&removed_segment_name)
                    .expect("Shouldn't fail because if add_segment removed this from the playlist then it must be in the HashMap");
        }
    }
}
