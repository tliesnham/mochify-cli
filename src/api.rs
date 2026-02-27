use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::fs;

const BASE_URL: &str = "https://api.mochify.xyz";

#[derive(Debug, Default)]
pub struct ProcessParams {
    pub format: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub crop: Option<bool>,
    pub rotation: Option<u32>,
}

pub struct MochifyClient {
    api_key: Option<String>,
    client: reqwest::Client,
}

impl MochifyClient {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }

    pub async fn squish(
        &self,
        file_path: &Path,
        params: &ProcessParams,
        out_dir: &Path,
    ) -> Result<PathBuf> {
        let bytes = fs::read(file_path)
            .await
            .with_context(|| format!("failed to read {}", file_path.display()))?;

        let mime = match file_path.extension().and_then(|e| e.to_str()) {
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("png") => "image/png",
            Some("webp") => "image/webp",
            Some("avif") => "image/avif",
            Some("jxl") => "image/jxl",
            Some("gif") => "image/gif",
            _ => "application/octet-stream",
        };

        let mut query: Vec<(&str, String)> = Vec::new();
        if let Some(ref fmt) = params.format {
            query.push(("type", fmt.clone()));
        }
        if let Some(w) = params.width {
            query.push(("width", w.to_string()));
        }
        if let Some(h) = params.height {
            query.push(("height", h.to_string()));
        }
        if let Some(c) = params.crop {
            query.push(("crop", c.to_string()));
        }
        if let Some(r) = params.rotation {
            query.push(("rotation", r.to_string()));
        }

        let mut req = self
            .client
            .post(format!("{BASE_URL}/v1/squish"))
            .query(&query)
            .header("Content-Type", mime)
            .body(bytes);

        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }

        let response = req.send().await.context("request failed")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API error {status}: {body}");
        }

        let image_bytes = response.bytes().await.context("failed to read response body")?;

        let stem = file_path
            .file_stem()
            .context("invalid file stem")?
            .to_string_lossy();

        let ext = params.format.as_deref().unwrap_or(
            file_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("jpg"),
        );

        let out_path = out_dir.join(format!("{stem}.{ext}"));

        fs::write(&out_path, &image_bytes)
            .await
            .with_context(|| format!("failed to write {}", out_path.display()))?;

        Ok(out_path)
    }
}
