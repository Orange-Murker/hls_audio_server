use std::{fs::File, io::Write};

use fdk_aac::enc::{Encoder, EncoderParams};

pub fn aac_encode(data: &[i16]) -> Vec<u8> {
    let encoder_parameters = EncoderParams {
        bit_rate: fdk_aac::enc::BitRate::Cbr(250000),
        sample_rate: 44100,
        transport: fdk_aac::enc::Transport::Adts,
        channels: fdk_aac::enc::ChannelMode::Stereo,
    };

    let encoder = Encoder::new(encoder_parameters).unwrap();
    let encoder_info = encoder.info().unwrap();

    let samples_per_chunk = 2 * encoder_info.frameLength as usize;
    let data_chunks = data.chunks(samples_per_chunk);

    let mut output: Vec<u8> = Vec::new();

    // Buffer length taken from the documentation
    // https://github.com/mstorsjo/fdk-aac/blob/master/documentation/aacEncoder.pdf
    let mut buf: [u8; 1536] = [0; 1536];
    for chunk in data_chunks {
        let result = encoder.encode(chunk, &mut buf).unwrap();
        output.extend(&buf[0..result.output_size]);
    }

    let mut file = File::create("test.aac").unwrap();
    file.write_all(&output).unwrap();
    output
}
