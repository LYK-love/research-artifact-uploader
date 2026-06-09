use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use serde_json::Value;

use crate::AppResult;

#[derive(Debug)]
pub struct RecordRow {
    pub time: String,
    pub run_id: String,
    pub run_name: String,
    pub project: String,
    pub archive_uri: String,
    pub size_bytes: u64,
    pub avg_mib_s: Option<f64>,
}

pub fn append_jsonl(path: &Path, payload: &Value) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create records directory failed: {e}"))?;
    }

    let text = format!("{}\n", payload);
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("open records jsonl failed: {e}"))?
        .write_all(text.as_bytes())
        .map_err(|e| format!("write records jsonl failed: {e}"))?;

    Ok(())
}

pub fn append_markdown(path: &Path, payload: &Value) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create records directory failed: {e}"))?;
    }

    let exists = path.exists();
    let mut fh = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("open records markdown failed: {e}"))?;

    if !exists {
        let header = "# Upload Records\n\n| Time | Run | Project | Size | Speed | Remote URI | SHA256 |\n| --- | --- | --- | ---: | ---: | --- | --- |\n";
        fh.write_all(header.as_bytes())
            .map_err(|e| format!("write markdown header failed: {e}"))?;
    }

    let speed = payload
        .get("avg_mib_s")
        .and_then(|x| x.as_f64())
        .unwrap_or(0.0);
    let size = payload
        .get("size_bytes")
        .and_then(|x| x.as_u64())
        .unwrap_or(0);
    let row = format!(
        "| {} | {} | {} | {:.1} MiB | {:.2} MiB/s | `{}` | `{}` |\n",
        payload.get("time").and_then(|x| x.as_str()).unwrap_or(""),
        payload.get("run_name").and_then(|x| x.as_str()).unwrap_or(""),
        payload.get("project").and_then(|x| x.as_str()).unwrap_or(""),
        bytes_to_mib(size),
        speed,
        payload
            .get("archive_uri")
            .and_then(|x| x.as_str())
            .unwrap_or(""),
        payload
            .get("sha256")
            .and_then(|x| x.as_str())
            .unwrap_or(""),
    );
    fh.write_all(row.as_bytes())
        .map_err(|e| format!("write markdown row failed: {e}"))?;
    Ok(())
}

pub fn read_records(path: &Path, last: usize) -> Vec<RecordRow> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut rows = Vec::new();
    for line in content.lines().rev().take(last) {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(line) {
            rows.push(RecordRow {
                time: value
                    .get("time")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
                run_id: value
                    .get("run_id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
                run_name: value
                    .get("run_name")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
                project: value
                    .get("project")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
                archive_uri: value
                    .get("archive_uri")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
                size_bytes: value
                    .get("size_bytes")
                    .and_then(|x| x.as_u64())
                    .unwrap_or(0),
                avg_mib_s: value.get("avg_mib_s").and_then(|x| x.as_f64()),
            });
        }
    }

    rows.reverse();
    rows
}

fn bytes_to_mib(size: u64) -> f64 {
    (size as f64) / (1024f64 * 1024f64)
}
