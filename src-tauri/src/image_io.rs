use anyhow::{anyhow, Result};
use image::{DynamicImage, RgbaImage};
use std::path::Path;
use libheif_rs::{HeifContext, ColorSpace, RgbChroma, LibHeif};

pub fn load_image<P: AsRef<Path>>(path: P) -> Result<DynamicImage> {
    let path = path.as_ref();
    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase());

    match extension.as_deref() {
        Some("heic") | Some("heif") => {
            let lib_heif = LibHeif::new();
            let ctx = HeifContext::read_from_file(path.to_str().ok_or_else(|| anyhow!("Invalid path"))?)
                .map_err(|e| anyhow!("Failed to read HEIC: {}", e))?;
            let handle = ctx.primary_image_handle()
                .map_err(|e| anyhow!("Failed to get HEIC handle: {}", e))?;
            let img = lib_heif.decode(&handle, ColorSpace::Rgb(RgbChroma::Rgba), None)
                .map_err(|e| anyhow!("Failed to decode HEIC: {}", e))?;
            
            let width = img.width();
            let height = img.height();
            let planes = img.planes();
            let interp = planes.interleaved.ok_or_else(|| anyhow!("Failed to get HEIC planes"))?;
            
            let data = interp.data;
            let stride = interp.stride;
            
            let buffer = if stride == width as usize * 4 {
                data.to_vec()
            } else {
                let mut buf = Vec::with_capacity(width as usize * height as usize * 4);
                for y in 0..height as usize {
                    let start = y * stride;
                    let end = start + width as usize * 4;
                    buf.extend_from_slice(&data[start..end]);
                }
                buf
            };

            let rgba = RgbaImage::from_raw(width, height, buffer)
                .ok_or_else(|| anyhow!("Failed to create RgbaImage from HEIC data"))?;
            
            Ok(DynamicImage::ImageRgba8(rgba))
        }
        _ => {
            let img = image::open(path)?;
            Ok(img)
        }
    }
}
