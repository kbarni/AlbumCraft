//! The slideshow's background music player; the settings and motion are
//! `albumcraft_core::slides` (re-exported here).

pub use albumcraft_core::slides::*;

/// The music file, decoded and looping.
fn open_music(path: &str) -> Result<rodio::decoder::LoopedDecoder<std::io::BufReader<std::fs::File>>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{path}: {e}"))?;
    rodio::Decoder::new_looped(std::io::BufReader::new(file)).map_err(|e| format!("this music file can't be played ({e})"))
}

/// Background music, looping until dropped.
pub struct Music {
    _sink: rodio::MixerDeviceSink,
    player: rodio::Player,
}

impl Music {
    pub fn start(path: &str) -> Result<Music, String> {
        let source = open_music(path)?;
        let mut sink = rodio::DeviceSinkBuilder::open_default_sink().map_err(|e| format!("no sound output ({e})"))?;
        sink.log_on_drop(false);
        let player = rodio::Player::connect_new(sink.mixer());
        player.set_volume(0.8);
        player.append(source);
        Ok(Music { _sink: sink, player })
    }

    pub fn set_playing(&self, on: bool) {
        if on {
            self.player.play();
        } else {
            self.player.pause();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wav_file_decodes() {
        // 0.1 s of 8 kHz mono 16-bit silence
        let samples = 800u32;
        let mut wav = b"RIFF".to_vec();
        wav.extend((36 + samples * 2).to_le_bytes());
        wav.extend(b"WAVEfmt ");
        wav.extend(16u32.to_le_bytes());
        wav.extend(1u16.to_le_bytes()); // PCM
        wav.extend(1u16.to_le_bytes()); // mono
        wav.extend(8000u32.to_le_bytes());
        wav.extend(16000u32.to_le_bytes());
        wav.extend(2u16.to_le_bytes());
        wav.extend(16u16.to_le_bytes());
        wav.extend(b"data");
        wav.extend((samples * 2).to_le_bytes());
        wav.extend(vec![0u8; samples as usize * 2]);
        let path = std::env::temp_dir().join(format!("albumcraft-music-{}.wav", std::process::id()));
        std::fs::write(&path, wav).unwrap();
        let ok = open_music(&path.to_string_lossy()).is_ok();
        let _ = std::fs::remove_file(&path);
        assert!(ok);
    }

    #[test]
    fn missing_music_is_an_error_not_a_crash() {
        assert!(Music::start("/no/such/song.mp3").is_err());
    }
}
