use std::{path::Path, sync::OnceLock};
use regex::Regex;

use crate::{AppResult, manifest::Manifest, subprocess_utils::{run_capture, run_stream}};

#[derive(Debug)]
pub struct OssUploadResult {
    pub success: bool,
    pub avg_mib_s: Option<f64>,
    pub stdout: String,
    pub stderr: String,
    pub command: Vec<String>,
}

static RE_MIB: OnceLock<Regex> = OnceLock::new();

pub fn oss_uri(bucket: &str, remote_dir: &str, filename: &str) -> String {
    format!("oss://{}/{}/{}", bucket, remote_dir.trim_start_matches('/'), filename)
}

pub fn check_access(bucket: &str, endpoint: &str, region: &str) -> AppResult<(bool, String)> {
    let cmd = vec![
        "ossutil".to_string(),
        "ls".to_string(),
        format!("oss://{bucket}"),
        "-e".to_string(),
        endpoint.to_string(),
        "--region".to_string(),
        region.to_string(),
    ];

    let result = run_capture(&cmd, None).map_err(|e| format!("ossutil check failed: {e}"))?;
    if result.returncode == 0 {
        Ok((true, String::new()))
    } else {
        Ok((false, format!("{}", result.stdout.trim())))
    }
}

pub fn upload_file(
    local_path: &Path,
    manifest: &Manifest,
    remote_dir: &str,
    filename: &str,
) -> AppResult<OssUploadResult> {
    let uri = oss_uri(&manifest.oss.bucket, remote_dir, filename);
    let cmd = vec![
        "ossutil".to_string(),
        "cp".to_string(),
        local_path
            .to_str()
            .ok_or_else(|| "invalid local path".to_string())?
            .to_string(),
        uri,
        "-e".to_string(),
        manifest.oss.endpoint.clone(),
        "--region".to_string(),
        manifest.oss.region.clone(),
    ];

    let result = run_stream(&cmd, None).map_err(|e| format!("ossutil execution failed: {e}"))?;
    if result.returncode != 0 {
        let summary = (result.stderr.as_str()).trim();
        let summary = if summary.is_empty() {
            result.stdout.trim()
        } else {
            summary
        };
        let command = cmd.join(" ");
        return Err(format!(
            "ossutil cp failed (exit code {}).\ncommand: {command}\nerror: {}\nsuggestion: retry command manually after fixing the root cause.",
            result.returncode,
            summary,
        ));
    }

    Ok(OssUploadResult {
        success: true,
        avg_mib_s: parse_avg_speed(&result.stdout),
        stdout: result.stdout,
        stderr: result.stderr,
        command: result.command,
    })
}

fn parse_avg_speed(text: &str) -> Option<f64> {
    let re = RE_MIB.get_or_init(|| {
        Regex::new(r"(?i)(?:avg|average)\s+([0-9]+(?:\.[0-9]+)?)\s*MiB/s").unwrap()
    });
    re.captures(text)
        .and_then(|m| m.get(1))
        .and_then(|v| v.as_str().parse::<f64>().ok())
}
