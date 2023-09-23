use fdk_aac::enc::{Encoder, EncoderError, EncoderParams};

pub fn aac_encode(data: &[i16]) -> Result<Vec<u8>, EncoderError> {
    let encoder_parameters = EncoderParams {
        bit_rate: fdk_aac::enc::BitRate::Cbr(320000),
        sample_rate: 48000,
        transport: fdk_aac::enc::Transport::Adts,
        channels: fdk_aac::enc::ChannelMode::Stereo,
    };

    let encoder = Encoder::new(encoder_parameters)?;
    let encoder_info = encoder.info()?;

    let samples_per_chunk = 2 * encoder_info.frameLength as usize;

    let data_chunks = data.chunks(samples_per_chunk);

    let mut output: Vec<u8> = Vec::new();

    // Buffer length taken from the documentation
    // https://github.com/mstorsjo/fdk-aac/blob/master/documentation/aacEncoder.pdf
    let mut buf: [u8; 1536] = [0; 1536];

    // This is necessary because otherwise the encoder would output two frames of silence
    encoder.encode(&data[0..samples_per_chunk], &mut buf)?;
    encoder.encode(&data[samples_per_chunk..samples_per_chunk * 2], &mut buf)?;

    for chunk in data_chunks {
        let result = encoder.encode(chunk, &mut buf)?;
        output.extend(&buf[0..result.output_size]);
    }

    Ok(output)
}
