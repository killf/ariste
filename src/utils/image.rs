use base64::engine::general_purpose::STANDARD as base64;
use base64::Engine;
use bytes::Bytes;

use crate::error::Error;

pub async fn load_image(image_url: &str) -> Result<Bytes, Error> {
    let client = reqwest::Client::new();
    let resp = client.get(image_url).send().await?;
    let buf = resp.bytes().await?;
    Ok(buf)
}

pub async fn load_image_as_base64(image_url: &str) -> Result<String, Error> {
    let buf = load_image(image_url).await?;

    Ok(base64.encode(buf))
}

pub async fn download_image(image_url: &str, target_file: &str) -> Result<(), Error> {
    let buf = load_image(image_url).await?;
    tokio::fs::write(target_file, buf).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_image() {
        let result = load_image_as_base64("http://172.16.200.202:9000/api/view?filename=ComfyUI_00811_.png&subfolder=&type=output").await;
        if let Ok(result) = result {
            println!("base64: len={}", result.len());
        }
    }
}
