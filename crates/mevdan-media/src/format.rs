//! Formatos de media y detección.

use crate::error::{MediaError, MediaResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Categoría general de un archivo de media.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Image,
    Audio,
    Video,
    Unknown,
}

impl MediaKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            MediaKind::Image => "image",
            MediaKind::Audio => "audio",
            MediaKind::Video => "video",
            MediaKind::Unknown => "unknown",
        }
    }
}

/// Formato concreto de un archivo de media.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaFormat {
    // Imágenes
    Png,
    Jpeg,
    Gif,
    Webp,
    Bmp,
    Svg,
    // Audio
    Mp3,
    Wav,
    Ogg,
    Flac,
    Aac,
    // Video
    Mp4,
    Webm,
    Mkv,
    Mov,
    Avi,
    // Desconocido
    Unknown,
}

impl MediaFormat {
    pub fn display_name(&self) -> &'static str {
        match self {
            MediaFormat::Png => "png",
            MediaFormat::Jpeg => "jpeg",
            MediaFormat::Gif => "gif",
            MediaFormat::Webp => "webp",
            MediaFormat::Bmp => "bmp",
            MediaFormat::Svg => "svg",
            MediaFormat::Mp3 => "mp3",
            MediaFormat::Wav => "wav",
            MediaFormat::Ogg => "ogg",
            MediaFormat::Flac => "flac",
            MediaFormat::Aac => "aac",
            MediaFormat::Mp4 => "mp4",
            MediaFormat::Webm => "webm",
            MediaFormat::Mkv => "mkv",
            MediaFormat::Mov => "mov",
            MediaFormat::Avi => "avi",
            MediaFormat::Unknown => "unknown",
        }
    }

    /// Categoría a la que pertenece.
    pub fn kind(&self) -> MediaKind {
        match self {
            MediaFormat::Png
            | MediaFormat::Jpeg
            | MediaFormat::Gif
            | MediaFormat::Webp
            | MediaFormat::Bmp
            | MediaFormat::Svg => MediaKind::Image,

            MediaFormat::Mp3
            | MediaFormat::Wav
            | MediaFormat::Ogg
            | MediaFormat::Flac
            | MediaFormat::Aac => MediaKind::Audio,

            MediaFormat::Mp4
            | MediaFormat::Webm
            | MediaFormat::Mkv
            | MediaFormat::Mov
            | MediaFormat::Avi => MediaKind::Video,

            MediaFormat::Unknown => MediaKind::Unknown,
        }
    }

    /// ¿Podemos leer la cabecera para obtener dimensiones/duración?
    pub fn has_parsable_header(&self) -> bool {
        matches!(
            self,
            MediaFormat::Png | MediaFormat::Jpeg | MediaFormat::Gif | MediaFormat::Wav
        )
    }

    /// Detecta el formato desde un path.
    pub fn from_path(path: &Path) -> MediaResult<Self> {
        let ext = match path.extension().and_then(|s| s.to_str()) {
            Some(e) => e.to_lowercase(),
            None => return Err(MediaError::UnsupportedFormat(path.display().to_string())),
        };

        Ok(match ext.as_str() {
            // Imagen
            "png" => MediaFormat::Png,
            "jpg" | "jpeg" => MediaFormat::Jpeg,
            "gif" => MediaFormat::Gif,
            "webp" => MediaFormat::Webp,
            "bmp" => MediaFormat::Bmp,
            "svg" => MediaFormat::Svg,
            // Audio
            "mp3" => MediaFormat::Mp3,
            "wav" => MediaFormat::Wav,
            "ogg" | "oga" => MediaFormat::Ogg,
            "flac" => MediaFormat::Flac,
            "aac" | "m4a" => MediaFormat::Aac,
            // Video
            "mp4" | "m4v" => MediaFormat::Mp4,
            "webm" => MediaFormat::Webm,
            "mkv" => MediaFormat::Mkv,
            "mov" => MediaFormat::Mov,
            "avi" => MediaFormat::Avi,
            _ => MediaFormat::Unknown,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn detect(path: &str) -> MediaResult<MediaFormat> {
        MediaFormat::from_path(&PathBuf::from(path))
    }

    #[test]
    fn kind_display_names() {
        assert_eq!(MediaKind::Image.display_name(), "image");
        assert_eq!(MediaKind::Audio.display_name(), "audio");
        assert_eq!(MediaKind::Video.display_name(), "video");
        assert_eq!(MediaKind::Unknown.display_name(), "unknown");
    }

    #[test]
    fn detect_png() {
        assert_eq!(detect("img.png").unwrap(), MediaFormat::Png);
        assert_eq!(detect("img.png").unwrap().kind(), MediaKind::Image);
    }

    #[test]
    fn detect_jpeg() {
        assert_eq!(detect("img.jpg").unwrap(), MediaFormat::Jpeg);
        assert_eq!(detect("img.jpeg").unwrap(), MediaFormat::Jpeg);
        assert_eq!(detect("img.jpeg").unwrap().kind(), MediaKind::Image);
    }

    #[test]
    fn detect_gif() {
        assert_eq!(detect("anim.gif").unwrap(), MediaFormat::Gif);
    }

    #[test]
    fn detect_webp() {
        assert_eq!(detect("img.webp").unwrap(), MediaFormat::Webp);
    }

    #[test]
    fn detect_bmp() {
        assert_eq!(detect("img.bmp").unwrap(), MediaFormat::Bmp);
    }

    #[test]
    fn detect_svg() {
        assert_eq!(detect("logo.svg").unwrap(), MediaFormat::Svg);
    }

    #[test]
    fn detect_mp3() {
        assert_eq!(detect("song.mp3").unwrap(), MediaFormat::Mp3);
        assert_eq!(detect("song.mp3").unwrap().kind(), MediaKind::Audio);
    }

    #[test]
    fn detect_wav() {
        assert_eq!(detect("sound.wav").unwrap(), MediaFormat::Wav);
    }

    #[test]
    fn detect_ogg() {
        assert_eq!(detect("sound.ogg").unwrap(), MediaFormat::Ogg);
    }

    #[test]
    fn detect_flac() {
        assert_eq!(detect("audio.flac").unwrap(), MediaFormat::Flac);
    }

    #[test]
    fn detect_aac() {
        assert_eq!(detect("audio.aac").unwrap(), MediaFormat::Aac);
        assert_eq!(detect("audio.m4a").unwrap(), MediaFormat::Aac);
    }

    #[test]
    fn detect_mp4() {
        assert_eq!(detect("video.mp4").unwrap(), MediaFormat::Mp4);
        assert_eq!(detect("video.mp4").unwrap().kind(), MediaKind::Video);
    }

    #[test]
    fn detect_webm() {
        assert_eq!(detect("clip.webm").unwrap(), MediaFormat::Webm);
    }

    #[test]
    fn detect_mkv() {
        assert_eq!(detect("movie.mkv").unwrap(), MediaFormat::Mkv);
    }

    #[test]
    fn detect_mov() {
        assert_eq!(detect("clip.mov").unwrap(), MediaFormat::Mov);
    }

    #[test]
    fn detect_avi() {
        assert_eq!(detect("old.avi").unwrap(), MediaFormat::Avi);
    }

    #[test]
    fn detect_unknown() {
        assert_eq!(detect("file.xyz").unwrap(), MediaFormat::Unknown);
    }

    #[test]
    fn detect_no_extension_fails() {
        assert!(detect("README").is_err());
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(detect("IMG.PNG").unwrap(), MediaFormat::Png);
        assert_eq!(detect("Song.MP3").unwrap(), MediaFormat::Mp3);
    }

    #[test]
    fn has_parsable_header() {
        assert!(MediaFormat::Png.has_parsable_header());
        assert!(MediaFormat::Jpeg.has_parsable_header());
        assert!(MediaFormat::Gif.has_parsable_header());
        assert!(MediaFormat::Wav.has_parsable_header());
        assert!(!MediaFormat::Mp3.has_parsable_header());
        assert!(!MediaFormat::Mp4.has_parsable_header());
        assert!(!MediaFormat::Unknown.has_parsable_header());
    }

    #[test]
    fn serializes_snake_case() {
        assert_eq!(serde_json::to_string(&MediaFormat::Png).unwrap(), "\"png\"");
        assert_eq!(
            serde_json::to_string(&MediaKind::Image).unwrap(),
            "\"image\""
        );
    }
}
