//! # mevdan-media
//!
//! Procesamiento de imágenes, audio y vídeo para MEVDAN.
//!
//! ## Concepto
//!
//! Un **asset** es un archivo de media (imagen, audio o vídeo) con
//! su formato, tamaño y metadata. `mevdan-media` detecta el formato
//! por extensión y extrae metadata básica cuando es posible sin
//! dependencias externas.
//!
//! ## Metadata soportada en V5.5
//!
//! | Formato | Dimensiones | Duración | Canales |
//! |---------|-------------|----------|---------|
//! | PNG | ✅ | — | — |
//! | GIF | ✅ | — | — |
//! | JPEG | ✅ | — | — |
//! | WAV | — | ✅ | ✅ |
//! | MP3 | ❌ | ❌ | ❌ |
//! | MP4/WebM/MKV | ❌ | ❌ | ❌ |
//!
//! Otros formatos se reconocen pero no se extrae metadata en V5.5.
//! La transcodificación y la generación son responsabilidad de un
//! provider, no de este crate.
//!
//! ## Estado del proyecto
//!
//! - **V5.5** ✅ — `MediaFormat`, `MediaAsset`, `MediaMetadata`, `MediaLoader`.
//!
//! ## Ejemplo
//!
//! ```no_run
//! use mevdan_media::MediaLoader;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let loader = MediaLoader::new();
//! let asset = loader.load("/path/to/image.png")?;
//!
//! println!("{}", asset.summary());
//! if let Some(dim) = asset.metadata.dimensions {
//!     println!("Dimensions: {}x{}", dim.width, dim.height);
//!     println!("Landscape: {}", dim.is_landscape());
//! }
//! # Ok(())
//! # }
//! ```

pub mod asset;
pub mod error;
pub mod format;
pub mod loader;

// Re-exports de conveniencia.
pub use asset::{Dimensions, MediaAsset, MediaMetadata};
pub use error::{MediaError, MediaResult};
pub use format::{MediaFormat, MediaKind};
pub use loader::MediaLoader;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn minimal_png(width: u32, height: u32) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        buf.extend_from_slice(&13u32.to_be_bytes());
        buf.extend_from_slice(b"IHDR");
        buf.extend_from_slice(&width.to_be_bytes());
        buf.extend_from_slice(&height.to_be_bytes());
        buf.push(8);
        buf.push(6);
        buf.push(0);
        buf.push(0);
        buf.push(0);
        buf.extend_from_slice(&[0u8; 4]);
        buf
    }

    #[test]
    fn full_flow_image_pipeline() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("hero.png");
        fs::write(&path, minimal_png(1920, 1080)).unwrap();

        let loader = MediaLoader::new();
        let asset = loader.load(&path).unwrap();

        assert_eq!(asset.kind, MediaKind::Image);
        assert_eq!(asset.format, MediaFormat::Png);

        let dim = asset.metadata.dimensions.unwrap();
        assert_eq!(dim.width, 1920);
        assert_eq!(dim.height, 1080);
        assert!(dim.is_landscape());
        assert!(!dim.is_portrait());
        assert!((dim.aspect_ratio() - 1.7777).abs() < 0.001);
        assert_eq!(dim.total_pixels(), 2_073_600);

        // Summary contiene info clave.
        let summary = asset.summary();
        assert!(summary.contains("hero.png"));
        assert!(summary.contains("png"));
        assert!(summary.contains("1920x1080"));
    }

    #[test]
    fn full_flow_mixed_media() {
        let dir = TempDir::new().unwrap();

        fs::write(dir.path().join("image.png"), minimal_png(100, 100)).unwrap();
        fs::write(dir.path().join("song.mp3"), b"fake mp3").unwrap();
        fs::write(dir.path().join("video.mp4"), b"fake mp4").unwrap();

        let loader = MediaLoader::new();

        let img = loader.load(dir.path().join("image.png")).unwrap();
        let audio = loader.load(dir.path().join("song.mp3")).unwrap();
        let video = loader.load(dir.path().join("video.mp4")).unwrap();

        assert_eq!(img.kind, MediaKind::Image);
        assert_eq!(audio.kind, MediaKind::Audio);
        assert_eq!(video.kind, MediaKind::Video);

        // Cada uno con su formato.
        assert_eq!(img.format, MediaFormat::Png);
        assert_eq!(audio.format, MediaFormat::Mp3);
        assert_eq!(video.format, MediaFormat::Mp4);
    }
}
