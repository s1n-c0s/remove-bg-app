use anyhow::{anyhow, Result};
use image::{DynamicImage, GenericImageView, ImageBuffer, Luma};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use once_cell::sync::Lazy;
use tract_onnx::prelude::*;

const MODEL_SIZE: u32 = 1024;

type Model = RunnableModel<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>;

static MODEL: Lazy<Arc<Mutex<Option<Model>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));

pub async fn init_session(model_path: &Path) -> Result<()> {
    let mut model_guard = MODEL.lock().await;
    if model_guard.is_none() {
        let model = tract_onnx::onnx()
            .model_for_path(model_path)?
            .with_input_fact(0, f32::fact([1, 3, MODEL_SIZE as usize, MODEL_SIZE as usize]).into())?
            .into_optimized()?
            .into_runnable()?;
        *model_guard = Some(model);
    }
    Ok(())
}

pub async fn remove_background(img: DynamicImage) -> Result<DynamicImage> {
    let (orig_width, orig_height) = img.dimensions();

    // 1. Preprocess
    let resized = img.resize_exact(MODEL_SIZE, MODEL_SIZE, image::imageops::FilterType::Lanczos3);
    let rgb = resized.to_rgb8();
    
    let mut tensor = tract_ndarray::Array4::<f32>::zeros((1, 3, MODEL_SIZE as usize, MODEL_SIZE as usize));
    for (x, y, pixel) in rgb.enumerate_pixels() {
        let r = (pixel[0] as f32 / 255.0 - 0.485) / 0.229;
        let g = (pixel[1] as f32 / 255.0 - 0.456) / 0.224;
        let b = (pixel[2] as f32 / 255.0 - 0.406) / 0.225;
        
        tensor[[0, 0, y as usize, x as usize]] = r;
        tensor[[0, 1, y as usize, x as usize]] = g;
        tensor[[0, 2, y as usize, x as usize]] = b;
    }

    let input: Tensor = tensor.into();

    // 2. Inference
    let model_guard = MODEL.lock().await;
    let model = model_guard.as_ref().ok_or_else(|| anyhow!("Model not initialized"))?;
    
    let result = model.run(tvec!(input.into()))?;
    let output = result[0].to_array_view::<f32>()?;
    
    // 3. Postprocess
    let mut mask_img: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::new(MODEL_SIZE, MODEL_SIZE);
    
    for y in 0..MODEL_SIZE {
        for x in 0..MODEL_SIZE {
            // output is [1, 1, 1024, 1024] or [1, 1024, 1024]
            // We need to check the shape to be safe
            let val = if output.ndim() == 4 {
                output[[0, 0, y as usize, x as usize]]
            } else {
                output[[0, y as usize, x as usize]]
            };
            let pixel_val = (val.clamp(0.0, 1.0) * 255.0) as u8;
            mask_img.put_pixel(x, y, Luma([pixel_val]));
        }
    }

    // Resize mask back to original size
    let mask_resized = DynamicImage::ImageLuma8(mask_img)
        .resize_exact(orig_width, orig_height, image::imageops::FilterType::Lanczos3);
    let mask_luma = mask_resized.to_luma8();

    // Apply mask to original image
    let mut rgba_img = img.to_rgba8();
    for (x, y, pixel) in rgba_img.enumerate_pixels_mut() {
        let mask_pixel = mask_luma.get_pixel(x, y);
        pixel[3] = mask_pixel[0];
    }

    Ok(DynamicImage::ImageRgba8(rgba_img))
}
