mod image_io;
mod model_runner;

use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use base64::{engine::general_purpose, Engine as _};
use std::io::Cursor;
use image::ImageFormat;

#[tauri::command]
async fn remove_bg(app: AppHandle, path: String) -> Result<String, String> {
    let input_path = PathBuf::from(&path);
    
    // 1. Load image
    let img = image_io::load_image(&input_path).map_err(|e| e.to_string())?;
    
    // 2. Ensure model is initialized
    let resource_path = app.path().resource_dir().map_err(|e| e.to_string())?
        .join("resources")
        .join("rmbg-1.4.onnx");
    
    // In dev mode, we might want to check a local path too
    let model_path = if resource_path.exists() {
        resource_path
    } else {
        // Fallback for dev
        PathBuf::from("resources/rmbg-1.4.onnx")
    };

    if !model_path.exists() {
        return Err(format!("Model file not found at {:?}. Please download it from Hugging Face (briaai/RMBG-1.4) and place it in the resources folder.", model_path));
    }

    model_runner::init_session(&model_path).await.map_err(|e| e.to_string())?;

    // 3. Process
    let processed = model_runner::remove_background(img).await.map_err(|e| e.to_string())?;

    // 4. Convert to base64 for preview
    let mut buffer = Cursor::new(Vec::new());
    processed.write_to(&mut buffer, ImageFormat::Png).map_err(|e| e.to_string())?;
    let base64 = general_purpose::STANDARD.encode(buffer.into_inner());
    
    Ok(format!("data:image/png;base64,{}", base64))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![remove_bg])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
