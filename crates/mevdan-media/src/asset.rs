//! `MediaAsset` y `MediaMetadata` — descriptor de un archivo de media.

use crate::format::{MediaFormat, MediaKind};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Dimensiones de una imagen o frame de video.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

impl Dimensions {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Proporción de aspecto (width / height).
    pub fn aspect_ratio(&self) -> f64 {
        if self.height == 0 {
            0.0
        } else {
            self.width as f64 / self.height as f64
        }
    }

    /// Número total de píxeles.
    pub fn total_pixels(&self) -> u64 {
        self.width as u64 * self.height as u64
    }

    /// ¿Es horizontal?
    pub fn is_landscape(&self) -> bool {
        self.width > self.height
    }

    /// ¿Es vertical?
    pub fn is_portrait(&self) -> bool {
        self.height > self.width
    }

    /// ¿Es cuadrada?
    pub fn is_square(&self) -> bool {
        self.width == self.height
    }
}

/// Metadata extraída de un archivo de media.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MediaMetadata {
    /// Dimensiones (imagen, o frame de vídeo).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<Dimensions>,

    /// Duración en segundos (audio, vídeo).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_secs: Option<f64>,

    /// Bitrate aproximado en kbps.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitrate_kbps: Option<u32>,

    /// Número de canales (audio).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channels: Option<u8>,

    /// Sample rate en Hz (audio).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_rate_hz: Option<u32>,

    /// ¿Tiene transparencia? (PNG, GIF, WebP)
    #[serde(default)]
    pub has_alpha: bool,

    /// ¿Es animado? (GIF, WebP)
    #[serde(default)]
    pub is_animated: bool,
}

impl MediaMetadata {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn with_dimensions(mut self, dim: Dimensions) -> Self {
        self.dimensions = Some(dim);
        self
    }

    pub fn with_duration(mut self, secs: f64) -> Self {
        self.duration_secs = Some(secs);
        self
    }

    pub fn with_channels(mut self, channels: u8, sample_rate: u32) -> Self {
        self.channels = Some(channels);
        self.sample_rate_hz = Some(sample_rate);
        self
    }
}

/// Descriptor de un archivo de media.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaAsset {
    pub path: PathBuf,
    pub format: MediaFormat,
    pub kind: MediaKind,
    pub size_bytes: u64,
    pub metadata: MediaMetadata,
}

impl MediaAsset {
    pub fn new(path: impl Into<PathBuf>, format: MediaFormat, size_bytes: u64) -> Self {
        Self {
            path: path.into(),
            format,
            kind: format.kind(),
            size_bytes,
            metadata: MediaMetadata::empty(),
        }
    }

    pub fn with_metadata(mut self, metadata: MediaMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Nombre del archivo.
    pub fn file_name(&self) -> String {
        self.path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string()
    }

    /// Tamaño en KB (con 2 decimales).
    pub fn size_kb(&self) -> f64 {
        self.size_bytes as f64 / 1024.0
    }

    /// Resumen textual.
    pub fn summary(&self) -> String {
        let mut parts = vec![
            self.file_name(),
            format!("[{}]", self.format.display_name()),
            format!("{:.1} KB", self.size_kb()),
        ];

        if let Some(dim) = self.metadata.dimensions {
            parts.push(format!("{}x{}", dim.width, dim.height));
        }
        if let Some(d) = self.metadata.duration_secs {
            parts.push(format!("{:.2}s", d));
        }
        if let Some(c) = self.metadata.channels {
            parts.push(format!("{}ch", c));
        }

        parts.join(" ")
    }
}

/// Helper para construir un MediaAsset desde un Path.
pub(crate) fn build_asset(path: &Path, format: MediaFormat) -> std::io::Result<MediaAsset> {
    let size = std::fs::metadata(path)?.len();
    Ok(MediaAsset::new(path.to_path_buf(), format, size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimensions_new() {
        let d = Dimensions::new(1920, 1080);
        assert_eq!(d.width, 1920);
        assert_eq!(d.height, 1080);
    }

    #[test]
    fn dimensions_aspect_ratio() {
        let d = Dimensions::new(1920, 1080);
        assert!((d.aspect_ratio() - 1.7777).abs() < 0.001);

        let d = Dimensions::new(1080, 1920);
        assert!((d.aspect_ratio() - 0.5625).abs() < 0.001);
    }

    #[test]
    fn dimensions_aspect_ratio_zero_height() {
        let d = Dimensions::new(100, 0);
        assert_eq!(d.aspect_ratio(), 0.0);
    }

    #[test]
    fn dimensions_total_pixels() {
        let d = Dimensions::new(1920, 1080);
        assert_eq!(d.total_pixels(), 2_073_600);
    }

    #[test]
    fn dimensions_orientation() {
        let landscape = Dimensions::new(1920, 1080);
        assert!(landscape.is_landscape());
        assert!(!landscape.is_portrait());
        assert!(!landscape.is_square());

        let portrait = Dimensions::new(1080, 1920);
        assert!(portrait.is_portrait());
        assert!(!portrait.is_landscape());

        let square = Dimensions::new(500, 500);
        assert!(square.is_square());
        assert!(!square.is_landscape());
        assert!(!square.is_portrait());
    }

    #[test]
    fn metadata_empty() {
        let m = MediaMetadata::empty();
        assert!(m.dimensions.is_none());
        assert!(m.duration_secs.is_none());
        assert!(!m.has_alpha);
    }

    #[test]
    fn metadata_with_dimensions() {
        let m = MediaMetadata::empty().with_dimensions(Dimensions::new(800, 600));
        assert_eq!(m.dimensions.unwrap().width, 800);
    }

    #[test]
    fn metadata_with_duration() {
        let m = MediaMetadata::empty().with_duration(3.5);
        assert_eq!(m.duration_secs, Some(3.5));
    }

    #[test]
    fn metadata_with_channels() {
        let m = MediaMetadata::empty().with_channels(2, 44100);
        assert_eq!(m.channels, Some(2));
        assert_eq!(m.sample_rate_hz, Some(44100));
    }

    #[test]
    fn asset_new() {
        let a = MediaAsset::new("/tmp/img.png", MediaFormat::Png, 1024);
        assert_eq!(a.format, MediaFormat::Png);
        assert_eq!(a.kind, MediaKind::Image);
        assert_eq!(a.size_bytes, 1024);
    }

    #[test]
    fn asset_with_metadata() {
        let a = MediaAsset::new("/tmp/img.png", MediaFormat::Png, 2048)
            .with_metadata(MediaMetadata::empty().with_dimensions(Dimensions::new(1920, 1080)));
        assert_eq!(a.metadata.dimensions.unwrap().width, 1920);
    }

    #[test]
    fn asset_file_name() {
        let a = MediaAsset::new("/tmp/sub/video.mp4", MediaFormat::Mp4, 0);
        assert_eq!(a.file_name(), "video.mp4");
    }

    #[test]
    fn asset_size_kb() {
        let a = MediaAsset::new("/tmp/img.png", MediaFormat::Png, 2048);
        assert!((a.size_kb() - 2.0).abs() < 0.001);
    }

    #[test]
    fn asset_summary_image() {
        let a = MediaAsset::new("/tmp/img.png", MediaFormat::Png, 102400)
            .with_metadata(MediaMetadata::empty().with_dimensions(Dimensions::new(1920, 1080)));
        let s = a.summary();
        assert!(s.contains("img.png"));
        assert!(s.contains("png"));
        assert!(s.contains("100.0 KB"));
        assert!(s.contains("1920x1080"));
    }

    #[test]
    fn asset_summary_audio() {
        let a = MediaAsset::new("/tmp/song.mp3", MediaFormat::Mp3, 5_000_000).with_metadata(
            MediaMetadata::empty()
                .with_duration(210.5)
                .with_channels(2, 44100),
        );
        let s = a.summary();
        assert!(s.contains("song.mp3"));
        assert!(s.contains("mp3"));
        assert!(s.contains("210.50s"));
        assert!(s.contains("2ch"));
    }

    #[test]
    fn asset_serializes() {
        let a = MediaAsset::new("/tmp/img.png", MediaFormat::Png, 1024);
        let json = serde_json::to_string(&a).unwrap();
        let back: MediaAsset = serde_json::from_str(&json).unwrap();
        assert_eq!(back.format, a.format);
        assert_eq!(back.size_bytes, a.size_bytes);
    }
}
