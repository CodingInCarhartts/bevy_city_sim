use std::fs;
use std::path::Path;
use std::io::Write;

/// On startup, downloads the Kenney city sprites if missing.
async fn download_city_sprites() {
    let url = "https://kenney.nl/assets/roguelike-modern-city/download";
    let out_path = "assets/roguelike_city.zip";
    let assets_folder = Path::new("assets");

    if !assets_folder.exists() {
        let _ = fs::create_dir_all(assets_folder);
    }

    if !Path::new(out_path).exists() {
        println!("Downloading city sprite asset pack from {url} (Kenney Roguelike Modern City CC0)...");
        if let Ok(mut resp) = reqwest::get(url).await {
            if let Ok(bytes) = resp.bytes().await {
                if let Ok(mut file) = fs::File::create(out_path) {
                    let _ = file.write_all(&bytes);
                    println!("Downloaded {out_path} - unzip to /assets");
                }
            }
        } else {
            println!("Sprite download failed - check your connection or download manually.");
        }
    } else {
        println!("Asset pack already present: {out_path}.");
    }
}

// USAGE EXAMPLE (put at the top of main() function in an async block or tokio::main):
// tokio::spawn(download_city_sprites());
