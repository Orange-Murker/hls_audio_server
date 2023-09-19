use std::collections::VecDeque;

use chrono::Utc;
use m3u8_rs::{MediaPlaylist, MediaSegment};

pub struct HLSConfig {
    pub segments_to_keep: usize,
    pub segment_duration: f32,
    /// URI of such form: `https://123.123.123.123:6969/`
    pub uri: String,
    /// The file extension for files served over HLS. For example: `aac`
    pub file_extension: String,
}

pub struct HLSPlayback {
    pub media_sequence: u64,
    pub segments: VecDeque<MediaSegment>,
    pub config: HLSConfig,
}

impl HLSPlayback {
    pub fn new(config: HLSConfig) -> Self {
        Self {
            media_sequence: 0,
            segments: VecDeque::new(),
            config,
        }
    }

    /// Add a new segment to the playlist and remove an old one if necessary
    /// Returns the name of just added segment and the removed segment
    pub fn add_segment(&mut self) -> (String, Option<String>) {
        let removed_segment_name = if self.segments.len() >= self.config.segments_to_keep {
            self.media_sequence += 1;
            Some(
                self.segments
                    .pop_front()
                    .unwrap()
                    .uri
                    .split("/")
                    .nth(3)
                    .unwrap()
                    .to_string(),
            )
        } else {
            None
        };

        let timestamp = Utc::now().timestamp_millis();

        let added_segment_name =
            String::from("segment-") + &timestamp.to_string() + &self.config.file_extension;

        let uri = self.config.uri.to_owned() + &added_segment_name;

        let segment = MediaSegment {
            uri,
            duration: self.config.segment_duration,
            title: None,
            byte_range: None,
            discontinuity: false,
            key: None,
            map: None,
            program_date_time: None,
            daterange: None,
            unknown_tags: Vec::new(),
        };
        self.segments.push_back(segment);

        (added_segment_name, removed_segment_name)
    }

    // Generate the playlist based on the current state
    pub fn generate_playlist(&mut self) -> Vec<u8> {
        let playlist = MediaPlaylist {
            version: Some(3),
            target_duration: self.config.segment_duration.ceil(),
            media_sequence: self.media_sequence,
            segments: self.segments.clone().into(),
            discontinuity_sequence: 0,
            end_list: false,
            playlist_type: None,
            i_frames_only: false,
            start: None,
            independent_segments: false,
            unknown_tags: Vec::new(),
        };

        let mut generated_playlist = Vec::new();

        generated_playlist.clear();
        playlist.write_to(&mut generated_playlist).unwrap();

        generated_playlist
    }
}
