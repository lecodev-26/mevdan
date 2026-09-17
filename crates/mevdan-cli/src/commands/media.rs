//! `mevdan media` — inspeccionar archivos de media.

use mevdan_media::MediaLoader;

pub fn run_info(path: &str, json: bool) -> anyhow::Result<()> {
    let loader = MediaLoader::new();
    let asset = loader.load(path)?;

    if json {
        let json = serde_json::to_string_pretty(&asset)?;
        println!("{}", json);
        return Ok(());
    }

    println!("MEVDAN — Media");
    println!("───────────────────────────────────────");
    println!("File:      {}", asset.file_name());
    println!("Path:      {}", asset.path.display());
    println!("Kind:      {}", asset.kind.display_name());
    println!("Format:    {}", asset.format.display_name());
    println!("Size:      {:.2} KB", asset.size_kb());

    if let Some(dim) = asset.metadata.dimensions {
        println!();
        println!("Dimensions: {}x{}", dim.width, dim.height);
        println!("Aspect:     {:.3}", dim.aspect_ratio());
        println!("Pixels:     {}", dim.total_pixels());
        println!(
            "Orientation: {}",
            if dim.is_landscape() {
                "landscape"
            } else if dim.is_portrait() {
                "portrait"
            } else {
                "square"
            }
        );
    }

    if let Some(d) = asset.metadata.duration_secs {
        println!();
        println!("Duration:  {:.2}s", d);
    }
    if let Some(c) = asset.metadata.channels {
        println!("Channels:  {}", c);
    }
    if let Some(sr) = asset.metadata.sample_rate_hz {
        println!("Sample:    {} Hz", sr);
    }
    if asset.metadata.has_alpha {
        println!("Alpha:     yes");
    }
    if asset.metadata.is_animated {
        println!("Animated:  yes");
    }

    Ok(())
}
