//! Encoding of raw mono PCM16 audio into files that Matrix clients can play.

use ogg::writing::{PacketWriteEndInfo, PacketWriter};
use opusic_c::{Application, Channels, Encoder, SampleRate};

/// Opus works with 20ms frames here: 480 samples at 24kHz.
const OPUS_FRAME_DURATION_MS: u32 = 20;

/// Ogg Opus granule positions always count 48kHz samples, whatever the input rate (RFC 7845).
const OGG_OPUS_GRANULE_RATE: u32 = 48_000;

/// The Ogg stream serial number. Each file holds a single logical stream, so any value works.
const OGG_STREAM_SERIAL: u32 = 0x6261_6962;

/// The largest Opus packet we accept from the encoder (recommended by libopus).
const OPUS_MAX_PACKET_SIZE: usize = 4000;

/// Converts little-endian PCM16 bytes into samples (a trailing odd byte is dropped).
pub fn pcm16_le_bytes_to_samples(bytes: &[u8]) -> Vec<i16> {
    bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| i16::from_le_bytes(*pair))
        .collect()
}

/// Encodes mono PCM16 samples into an Ogg Opus file (`audio/ogg`).
pub fn encode_pcm16_mono_to_ogg_opus(samples: &[i16], sample_rate: u32) -> anyhow::Result<Vec<u8>> {
    let opus_sample_rate = match sample_rate {
        8_000 => SampleRate::Hz8000,
        12_000 => SampleRate::Hz12000,
        16_000 => SampleRate::Hz16000,
        24_000 => SampleRate::Hz24000,
        48_000 => SampleRate::Hz48000,
        _ => {
            return Err(anyhow::anyhow!(
                "Opus does not support a sample rate of {sample_rate}Hz"
            ));
        }
    };

    let mut encoder = Encoder::new(Channels::Mono, opus_sample_rate, Application::Voip)
        .map_err(|err| anyhow::anyhow!("Failed creating the Opus encoder: {err:?}"))?;

    let granule_per_input_sample = (OGG_OPUS_GRANULE_RATE / sample_rate) as u64;
    let frame_size = (sample_rate * OPUS_FRAME_DURATION_MS / 1000) as usize;

    // The encoder's look-ahead delays its output: decoders skip that many samples at the start.
    let look_ahead = encoder
        .get_look_ahead()
        .map_err(|err| anyhow::anyhow!("Failed getting the Opus look-ahead: {err:?}"))?
        as usize;
    let pre_skip = look_ahead as u64 * granule_per_input_sample;

    // Pad with silence to flush the look-ahead and to fill the last frame.
    // The final granule position tells decoders to drop the padding again.
    let mut padded = samples.to_vec();
    padded.resize(samples.len() + look_ahead, 0);
    padded.resize(padded.len().div_ceil(frame_size) * frame_size, 0);

    let end_granule = pre_skip + samples.len() as u64 * granule_per_input_sample;

    let mut output = Vec::new();
    let mut writer = PacketWriter::new(&mut output);

    writer.write_packet(
        opus_head(pre_skip as u16, sample_rate),
        OGG_STREAM_SERIAL,
        PacketWriteEndInfo::EndPage,
        0,
    )?;
    writer.write_packet(
        opus_tags(),
        OGG_STREAM_SERIAL,
        PacketWriteEndInfo::EndPage,
        0,
    )?;

    let frame_count = padded.len() / frame_size;
    let mut packet = Vec::with_capacity(OPUS_MAX_PACKET_SIZE);

    for (index, frame) in padded.chunks_exact(frame_size).enumerate() {
        // libopus takes PCM16 samples, typed as u16 by these bindings.
        let frame: Vec<u16> = frame.iter().map(|sample| *sample as u16).collect();

        packet.clear();
        encoder
            .encode_to_vec(&frame, &mut packet)
            .map_err(|err| anyhow::anyhow!("Failed encoding Opus audio: {err:?}"))?;

        let is_last = index + 1 == frame_count;
        let (end_info, granule) = if is_last {
            (PacketWriteEndInfo::EndStream, end_granule)
        } else {
            (
                PacketWriteEndInfo::NormalPacket,
                ((index + 1) * frame_size) as u64 * granule_per_input_sample,
            )
        };

        writer.write_packet(packet.clone(), OGG_STREAM_SERIAL, end_info, granule)?;
    }

    drop(writer);

    Ok(output)
}

/// Encodes mono PCM16 samples into a WAV file (`audio/wav`).
pub fn encode_pcm16_mono_to_wav(samples: &[i16], sample_rate: u32) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;

    let mut output = Vec::with_capacity(44 + data_len as usize);
    output.extend_from_slice(b"RIFF");
    output.extend_from_slice(&(36 + data_len).to_le_bytes());
    output.extend_from_slice(b"WAVE");
    output.extend_from_slice(b"fmt ");
    output.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    output.extend_from_slice(&1u16.to_le_bytes()); // PCM
    output.extend_from_slice(&1u16.to_le_bytes()); // mono
    output.extend_from_slice(&sample_rate.to_le_bytes());
    output.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
    output.extend_from_slice(&2u16.to_le_bytes()); // block align
    output.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    output.extend_from_slice(b"data");
    output.extend_from_slice(&data_len.to_le_bytes());

    for sample in samples {
        output.extend_from_slice(&sample.to_le_bytes());
    }

    output
}

fn opus_head(pre_skip: u16, input_sample_rate: u32) -> Vec<u8> {
    let mut head = Vec::with_capacity(19);
    head.extend_from_slice(b"OpusHead");
    head.push(1); // version
    head.push(1); // channel count
    head.extend_from_slice(&pre_skip.to_le_bytes());
    head.extend_from_slice(&input_sample_rate.to_le_bytes());
    head.extend_from_slice(&0i16.to_le_bytes()); // output gain
    head.push(0); // channel mapping family: mono/stereo
    head
}

fn opus_tags() -> Vec<u8> {
    let vendor = b"baibot";

    let mut tags = Vec::with_capacity(16 + vendor.len());
    tags.extend_from_slice(b"OpusTags");
    tags.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    tags.extend_from_slice(vendor);
    tags.extend_from_slice(&0u32.to_le_bytes()); // no user comments
    tags
}

#[cfg(test)]
mod tests {
    use ogg::reading::PacketReader;

    use super::*;

    fn sine(sample_rate: u32, samples: usize) -> Vec<i16> {
        (0..samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                ((t * 440.0 * std::f32::consts::TAU).sin() * 8000.0) as i16
            })
            .collect()
    }

    #[test]
    fn ogg_opus_has_valid_headers_and_final_granule() {
        // 1.01 seconds: not a whole number of frames, so the last frame needs padding.
        let samples = sine(24_000, 24_240);

        let file = encode_pcm16_mono_to_ogg_opus(&samples, 24_000).unwrap();

        let mut reader = PacketReader::new(std::io::Cursor::new(file));

        let head = reader.read_packet_expected().unwrap();
        assert_eq!(&head.data[0..8], b"OpusHead");
        assert_eq!(head.data[9], 1);
        let pre_skip = u16::from_le_bytes([head.data[10], head.data[11]]) as u64;
        assert!(pre_skip > 0);
        assert_eq!(
            u32::from_le_bytes(head.data[12..16].try_into().unwrap()),
            24_000
        );

        let tags = reader.read_packet_expected().unwrap();
        assert_eq!(&tags.data[0..8], b"OpusTags");

        let mut audio_packets = 0;
        let mut last = None;
        while let Some(packet) = reader.read_packet().unwrap() {
            audio_packets += 1;
            last = Some(packet);
        }
        let last = last.unwrap();

        assert!(last.last_in_stream());
        // Decoders play exactly the input length: the end granule minus the pre-skip.
        assert_eq!(last.absgp_page() - pre_skip, 24_240 * 2);
        // Every 20ms packet decodes to 960 samples at 48kHz; together they cover the end granule.
        assert!(audio_packets * 960 >= last.absgp_page());
    }

    #[test]
    fn ogg_opus_handles_empty_input() {
        let file = encode_pcm16_mono_to_ogg_opus(&[], 24_000).unwrap();
        assert!(file.starts_with(b"OggS"));
    }

    #[test]
    fn ogg_opus_rejects_unsupported_sample_rates() {
        assert!(encode_pcm16_mono_to_ogg_opus(&[0; 100], 44_100).is_err());
    }

    #[test]
    fn wav_header_describes_the_data() {
        let file = encode_pcm16_mono_to_wav(&[1, -1, 256], 24_000);

        assert_eq!(&file[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(file[4..8].try_into().unwrap()), 36 + 6);
        assert_eq!(&file[8..12], b"WAVE");
        assert_eq!(u32::from_le_bytes(file[24..28].try_into().unwrap()), 24_000);
        assert_eq!(&file[36..40], b"data");
        assert_eq!(u32::from_le_bytes(file[40..44].try_into().unwrap()), 6);
        assert_eq!(&file[44..], &[1, 0, 255, 255, 0, 1]);
    }

    #[test]
    fn pcm16_bytes_become_samples() {
        assert_eq!(pcm16_le_bytes_to_samples(&[1, 0, 255, 255, 7]), vec![1, -1]);
    }
}
