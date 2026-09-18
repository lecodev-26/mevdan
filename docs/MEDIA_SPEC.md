# MEVDAN — Media Specification

`mevdan-media` handles images, audio and video. It detects formats
and extracts basic metadata without external dependencies.

## Why one crate for three kinds

Images, audio and video share the same conceptual structure:

- Format (PNG, MP3, MP4, ...).
- Asset descriptor (path, format, size).
- Metadata (dimensions, duration, channels).
- Capabilities (transcode? generate? — arrives with providers).

Instead of three near-identical crates, one cohesive crate.

## Concepts

### MediaKind

```rust
pub enum MediaKind {
    Image, Audio, Video, Unknown,
}
MediaFormat
rust
pub enum MediaFormat {
    // Images
    Png, Jpeg, Gif, Webp, Bmp, Svg,
    // Audio
    Mp3, Wav, Ogg, Flac, Aac,
    // Video
    Mp4, Webm, Mkv, Mov, Avi,
    // Unknown
    Unknown,
}
Helpers:

kind() — Image / Audio / Video / Unknown.

has_parsable_header() — true for PNG, JPEG, GIF, WAV.

from_path(path) — detect by extension.

Dimensions
rust
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}
Helpers:

aspect_ratio() — width / height.

total_pixels() — width × height.

is_landscape() / is_portrait() / is_square().

MediaMetadata
All fields optional:

rust
pub struct MediaMetadata {
    pub dimensions: Option<Dimensions>,
    pub duration_secs: Option<f64>,
    pub bitrate_kbps: Option<u32>,
    pub channels: Option<u8>,
    pub sample_rate_hz: Option<u32>,
    pub has_alpha: bool,
    pub is_animated: bool,
}
MediaAsset
Descriptor of a media file:

rust
pub struct MediaAsset {
    pub path: PathBuf,
    pub format: MediaFormat,
    pub kind: MediaKind,
    pub size_bytes: u64,
    pub metadata: MediaMetadata,
}
summary() returns a one-line description.

MediaLoader
rust
let loader = MediaLoader::new();
let asset = loader.load("image.png")?;

assert_eq!(asset.kind, MediaKind::Image);
assert_eq!(asset.metadata.dimensions.unwrap().width, 1920);
Header parsing
Metadata extraction is header-only (no deps):

Format	Dimensions	Other
PNG	✅ (IHDR chunk)	has_alpha from color type
GIF	✅ (header)	is_animated from version (89a)
JPEG	✅ (SOF marker)	—
WAV	❌	channels, sample rate, duration, bitrate
PNG
Bytes 0-7: magic \x89PNG\r\n\x1a\n.

Bytes 16-23: width, height (big-endian u32).

Byte 25: color type (4 or 6 → alpha).

GIF
Bytes 0-5: GIF87a or GIF89a.

Bytes 6-9: width, height (little-endian u16).

is_animated = true iff version is 89a.

JPEG
Scans markers until a SOF (Start of Frame).

Reads height (2 bytes) and width (2 bytes) from the SOF segment.

WAV
Bytes 0-11: RIFF + size + WAVE.

Bytes 22-23: channels.

Bytes 24-27: sample rate.

Bytes 28-31: byte rate (for duration calc).

Duration = (file size - 44) / byte rate.

What this is NOT
No transcoding. Real codec work is a provider's job.

No frame extraction. Frames arrive with providers.

No MP3/MP4 metadata. Complex formats, arrive with providers.

No generation. Images/audio/video generation belongs to
providers, not this crate.

No image processing. No resizing, no filters.

Design notes
Zero dependencies. Only std + serde.

Header-only. Fast, deterministic.

Serializable. Every type roundtrips through JSON.

Provider-ready. Assets are descriptors; providers act on them.
