//! `MediaLoader` — carga archivos de media y extrae metadata básica.
//!
//! Extracción soportada en V5.5:
//!
//! - **PNG** — dimensiones + canal alfa (IHDR).
//! - **GIF** — dimensiones + animación (header).
//! - **JPEG** — dimensiones (SOF marker).
//! - **WAV** — canales, sample rate, duración (header RIFF).
//!
//! Otros formatos (MP3, MP4, WebM, ...) se reconocen pero no se
//! extrae metadata en V5.5. La extracción real (o transcodificación)
//! es responsabilidad de un provider, no de este crate.

use crate::{
    asset::{build_asset, Dimensions, MediaAsset, MediaMetadata},
    error::{MediaError, MediaResult},
    format::MediaFormat,
};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Cargador de media.
#[derive(Debug, Default)]
pub struct MediaLoader;

impl MediaLoader {
    pub fn new() -> Self {
        Self
    }

    /// Carga un asset de media desde una ruta.
    pub fn load(&self, path: impl AsRef<Path>) -> MediaResult<MediaAsset> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(MediaError::NotFound(path.display().to_string()));
        }
        if !path.is_file() {
            return Err(MediaError::InvalidPath(path.display().to_string()));
        }

        let format = MediaFormat::from_path(path)?;
        let mut asset = build_asset(path, format)?;

        // Extrae metadata según formato.
        let metadata = match format {
            MediaFormat::Png => self.extract_png(path)?,
            MediaFormat::Gif => self.extract_gif(path)?,
            MediaFormat::Jpeg => self.extract_jpeg(path)?,
            MediaFormat::Wav => self.extract_wav(path)?,
            _ => MediaMetadata::empty(),
        };

        asset.metadata = metadata;
        Ok(asset)
    }

    /// Extrae dimensiones de un PNG (IHDR chunk).
    ///
    /// Formato del PNG:
    /// - Bytes 0-7: magic `\x89PNG\r\n\x1a\n`
    /// - Bytes 8-11: IHDR length (13)
    /// - Bytes 12-15: "IHDR"
    /// - Bytes 16-19: width (big-endian u32)
    /// - Bytes 20-23: height (big-endian u32)
    /// - Byte 24: bit depth
    /// - Byte 25: color type (6 = RGBA → alpha)
    fn extract_png(&self, path: &Path) -> MediaResult<MediaMetadata> {
        let mut file = File::open(path)?;
        let mut header = [0u8; 26];
        file.read_exact(&mut header)
            .map_err(|_| MediaError::InvalidHeader("PNG file too small".into()))?;

        // Comprueba magic.
        if &header[0..8] != b"\x89PNG\r\n\x1a\n" {
            return Err(MediaError::InvalidHeader("bad PNG magic".into()));
        }

        let width = u32::from_be_bytes([header[16], header[17], header[18], header[19]]);
        let height = u32::from_be_bytes([header[20], header[21], header[22], header[23]]);
        let color_type = header[25];

        // Color types: 0=gray, 2=RGB, 3=palette, 4=gray+alpha, 6=RGBA.
        let has_alpha = matches!(color_type, 4 | 6);

        Ok(MediaMetadata::empty()
            .with_dimensions(Dimensions::new(width, height))
            .with_alpha(has_alpha))
    }

    /// Extrae dimensiones de un GIF (header).
    ///
    /// Formato del GIF:
    /// - Bytes 0-5: "GIF87a" or "GIF89a"
    /// - Bytes 6-7: width (little-endian u16)
    /// - Bytes 8-9: height (little-endian u16)
    ///
    /// La animación se detecta buscando un Graphic Control Extension
    /// (0x21 0xF9) antes del primer Image Descriptor. Por simplicidad,
    /// en V5.5 solo marcamos `is_animated` si el header es "GIF89a"
    /// (que técnicamente habilita animación).
    fn extract_gif(&self, path: &Path) -> MediaResult<MediaMetadata> {
        let mut file = File::open(path)?;
        let mut header = [0u8; 10];
        file.read_exact(&mut header)
            .map_err(|_| MediaError::InvalidHeader("GIF file too small".into()))?;

        if &header[0..3] != b"GIF" {
            return Err(MediaError::InvalidHeader("bad GIF magic".into()));
        }

        let width = u16::from_le_bytes([header[6], header[7]]) as u32;
        let height = u16::from_le_bytes([header[8], header[9]]) as u32;

        // GIF87a = sin animación por spec; GIF89a = con animación posible.
        let is_animated = &header[3..6] == b"89a";

        Ok(MediaMetadata {
            dimensions: Some(Dimensions::new(width, height)),
            is_animated,
            has_alpha: true, // GIF siempre soporta transparencia por palette.
            ..Default::default()
        })
    }

    /// Extrae dimensiones de un JPEG (SOF marker).
    ///
    /// Formato del JPEG:
    /// - Bytes 0-1: 0xFF 0xD8 (SOI)
    /// - Luego segmentos: 0xFF <marker> <length> <data>
    /// - Buscamos SOF0/SOF1/SOF2 (0xC0, 0xC1, 0xC2).
    /// - En el SOF: 2 bytes length, 1 byte precision,
    ///   2 bytes height, 2 bytes width.
    fn extract_jpeg(&self, path: &Path) -> MediaResult<MediaMetadata> {
        let mut file = File::open(path)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;

        if buf.len() < 4 || buf[0] != 0xFF || buf[1] != 0xD8 {
            return Err(MediaError::InvalidHeader("bad JPEG magic".into()));
        }

        let mut i = 2;
        while i + 9 < buf.len() {
            if buf[i] != 0xFF {
                i += 1;
                continue;
            }

            let marker = buf[i + 1];

            // SOF0..SOF3, SOF5..SOF7, SOF9..SOF11, SOF13..SOF15
            let is_sof = matches!(
                marker,
                0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF
            );

            if is_sof {
                if i + 9 >= buf.len() {
                    break;
                }
                let height = u16::from_be_bytes([buf[i + 5], buf[i + 6]]) as u32;
                let width = u16::from_be_bytes([buf[i + 7], buf[i + 8]]) as u32;
                return Ok(MediaMetadata::empty().with_dimensions(Dimensions::new(width, height)));
            }

            // Salta el segmento.
            let seg_len = u16::from_be_bytes([buf[i + 2], buf[i + 3]]) as usize;
            if seg_len < 2 {
                break;
            }
            i += 2 + seg_len;
        }

        Err(MediaError::InvalidHeader(
            "JPEG SOF marker not found".into(),
        ))
    }

    /// Extrae metadata de un WAV (header RIFF).
    ///
    /// Formato del WAV:
    /// - Bytes 0-3: "RIFF"
    /// - Bytes 4-7: file size (little-endian, -8)
    /// - Bytes 8-11: "WAVE"
    /// - Bytes 12-15: "fmt "
    /// - Bytes 16-19: fmt chunk length (16 for PCM)
    /// - Bytes 20-21: audio format (1 = PCM)
    /// - Bytes 22-23: channels
    /// - Bytes 24-27: sample rate
    /// - Bytes 28-31: byte rate
    /// - Bytes 32-33: block align
    /// - Bytes 34-35: bits per sample
    fn extract_wav(&self, path: &Path) -> MediaResult<MediaMetadata> {
        let mut file = File::open(path)?;
        let mut header = [0u8; 36];
        file.read_exact(&mut header)
            .map_err(|_| MediaError::InvalidHeader("WAV file too small".into()))?;

        if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
            return Err(MediaError::InvalidHeader("bad WAV magic".into()));
        }

        let channels = u16::from_le_bytes([header[22], header[23]]) as u8;
        let sample_rate = u32::from_le_bytes([header[24], header[25], header[26], header[27]]);
        let byte_rate = u32::from_le_bytes([header[28], header[29], header[30], header[31]]);

        // Duración = tamaño del audio / byte_rate.
        // El tamaño del audio son los bytes totales - 44 (header WAV estándar).
        let total_size = std::fs::metadata(path)?.len();
        let audio_bytes = total_size.saturating_sub(44) as f64;
        let duration_secs = if byte_rate > 0 {
            Some(audio_bytes / byte_rate as f64)
        } else {
            None
        };

        let mut m = MediaMetadata::empty().with_channels(channels, sample_rate);
        if let Some(d) = duration_secs {
            m = m.with_duration(d);
        }
        m.bitrate_kbps = Some((byte_rate * 8) / 1000);
        Ok(m)
    }
}

// Helper: extensión privada del `MediaMetadata` para builders internos.
impl MediaMetadata {
    fn with_alpha(mut self, has_alpha: bool) -> Self {
        self.has_alpha = has_alpha;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(dir: &Path, name: &str, content: &[u8]) -> std::path::PathBuf {
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    /// PNG mínimo de 1x1 transparente.
    fn minimal_png(width: u32, height: u32) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        // IHDR chunk length (13)
        buf.extend_from_slice(&13u32.to_be_bytes());
        buf.extend_from_slice(b"IHDR");
        buf.extend_from_slice(&width.to_be_bytes());
        buf.extend_from_slice(&height.to_be_bytes());
        buf.push(8); // bit depth
        buf.push(6); // color type RGBA
        buf.push(0); // compression
        buf.push(0); // filter
        buf.push(0); // interlace
                     // Fake CRC (no lo validamos).
        buf.extend_from_slice(&[0u8; 4]);
        buf
    }

    /// GIF mínimo con header.
    fn minimal_gif(width: u16, height: u16, version: &[u8; 3]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"GIF");
        buf.extend_from_slice(version);
        buf.extend_from_slice(&width.to_le_bytes());
        buf.extend_from_slice(&height.to_le_bytes());
        buf
    }

    #[test]
    fn load_fails_when_not_found() {
        let loader = MediaLoader::new();
        let err = loader.load("/definitely/not/a/file.png").unwrap_err();
        assert!(matches!(err, MediaError::NotFound(_)));
    }

    #[test]
    fn load_fails_on_directory() {
        let dir = TempDir::new().unwrap();
        let loader = MediaLoader::new();
        let err = loader.load(dir.path()).unwrap_err();
        assert!(matches!(err, MediaError::InvalidPath(_)));
    }

    #[test]
    fn load_png_extracts_dimensions() {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "img.png", &minimal_png(100, 50));

        let loader = MediaLoader::new();
        let asset = loader.load(&path).unwrap();

        assert_eq!(asset.format, MediaFormat::Png);
        let dim = asset.metadata.dimensions.unwrap();
        assert_eq!(dim.width, 100);
        assert_eq!(dim.height, 50);
        assert!(asset.metadata.has_alpha);
    }

    #[test]
    fn load_png_rejects_bad_magic() {
        let dir = TempDir::new().unwrap();
        let mut bad = minimal_png(10, 10);
        bad[1] = b'X';
        let path = write(dir.path(), "bad.png", &bad);

        let loader = MediaLoader::new();
        let err = loader.load(&path).unwrap_err();
        assert!(matches!(err, MediaError::InvalidHeader(_)));
    }

    #[test]
    fn load_gif_extracts_dimensions() {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "anim.gif", &minimal_gif(200, 100, b"89a"));

        let loader = MediaLoader::new();
        let asset = loader.load(&path).unwrap();

        assert_eq!(asset.format, MediaFormat::Gif);
        let dim = asset.metadata.dimensions.unwrap();
        assert_eq!(dim.width, 200);
        assert_eq!(dim.height, 100);
        assert!(asset.metadata.is_animated);
    }

    #[test]
    fn load_gif87a_not_animated() {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "static.gif", &minimal_gif(50, 50, b"87a"));

        let loader = MediaLoader::new();
        let asset = loader.load(&path).unwrap();
        assert!(!asset.metadata.is_animated);
    }

    #[test]
    fn load_gif_rejects_bad_magic() {
        let dir = TempDir::new().unwrap();
        let mut bad = minimal_gif(10, 10, b"89a");
        bad[0] = b'X';
        let path = write(dir.path(), "bad.gif", &bad);

        let loader = MediaLoader::new();
        let err = loader.load(&path).unwrap_err();
        assert!(matches!(err, MediaError::InvalidHeader(_)));
    }

    /// WAV mínimo con 16-bit stereo PCM, 44100 Hz.
    fn minimal_wav(duration_samples: u32) -> Vec<u8> {
        let mut buf = Vec::new();
        let channels: u16 = 2;
        let sample_rate: u32 = 44100;
        let bits_per_sample: u16 = 16;
        let byte_rate = sample_rate * channels as u32 * (bits_per_sample / 8) as u32;
        let block_align = channels * (bits_per_sample / 8);
        let data_size = byte_rate * duration_samples; // duration_samples segundos

        let riff_size = 36 + data_size;
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&riff_size.to_le_bytes());
        buf.extend_from_slice(b"WAVE");
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
        buf.extend_from_slice(&channels.to_le_bytes());
        buf.extend_from_slice(&sample_rate.to_le_bytes());
        buf.extend_from_slice(&byte_rate.to_le_bytes());
        buf.extend_from_slice(&block_align.to_le_bytes());
        buf.extend_from_slice(&bits_per_sample.to_le_bytes());
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&data_size.to_le_bytes());
        // Silencio.
        buf.extend(std::iter::repeat(0u8).take(data_size as usize));
        buf
    }

    #[test]
    fn load_wav_extracts_metadata() {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "sound.wav", &minimal_wav(1));

        let loader = MediaLoader::new();
        let asset = loader.load(&path).unwrap();

        assert_eq!(asset.format, MediaFormat::Wav);
        assert_eq!(asset.metadata.channels, Some(2));
        assert_eq!(asset.metadata.sample_rate_hz, Some(44100));
        // 1 segundo de audio.
        let d = asset.metadata.duration_secs.unwrap();
        assert!((d - 1.0).abs() < 0.1, "duration was {}", d);
    }

    #[test]
    fn load_wav_rejects_bad_magic() {
        let dir = TempDir::new().unwrap();
        let mut bad = minimal_wav(1);
        bad[0] = b'X';
        let path = write(dir.path(), "bad.wav", &bad);

        let loader = MediaLoader::new();
        let err = loader.load(&path).unwrap_err();
        assert!(matches!(err, MediaError::InvalidHeader(_)));
    }

    #[test]
    fn load_mp3_no_metadata() {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "song.mp3", b"fake mp3 content");

        let loader = MediaLoader::new();
        let asset = loader.load(&path).unwrap();

        assert_eq!(asset.format, MediaFormat::Mp3);
        assert_eq!(asset.kind, crate::format::MediaKind::Audio);
        // En V5.5, MP3 no se parsea.
        assert!(asset.metadata.dimensions.is_none());
        assert!(asset.metadata.duration_secs.is_none());
    }

    #[test]
    fn load_mp4_no_metadata() {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "video.mp4", b"fake mp4 content");

        let loader = MediaLoader::new();
        let asset = loader.load(&path).unwrap();

        assert_eq!(asset.format, MediaFormat::Mp4);
        assert_eq!(asset.kind, crate::format::MediaKind::Video);
    }

    #[test]
    fn load_unknown_format() {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "file.xyz", b"content");

        let loader = MediaLoader::new();
        let asset = loader.load(&path).unwrap();

        assert_eq!(asset.format, MediaFormat::Unknown);
        assert_eq!(asset.kind, crate::format::MediaKind::Unknown);
    }
}
