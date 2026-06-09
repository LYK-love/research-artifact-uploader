use std::{env, fs::File, io::Write, path::{Path, PathBuf}, time::Instant};

use clap::{Parser, Subcommand};

use crate::{
    archive::{compute_sha256, create_archive, make_timestamp, run_id_for_run, write_manifest_snapshot, write_sha256},
    collect::{collect_artifacts, CollectedArtifact},
    gitinfo::read_git_info,
    manifest::{parse_manifest, Manifest},
    metadata::{build_metadata, Metadata},
    oss::{check_access, oss_uri, upload_file},
    records::{append_jsonl, append_markdown, read_records, RecordRow},
};

#[derive(Parser)]
#[command(name = "rau", version = "0.1.0", about = "Upload research artifacts to OSS via ossutil")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Check {
        #[arg(long, short)]
        manifest: PathBuf,
        #[arg(long)]
        allow_outside_project: bool,
    },
    Pack {
        #[arg(long, short)]
        manifest: PathBuf,
        #[arg(long)]
        allow_outside_project: bool,
    },
    Upload {
        #[arg(long, short)]
        manifest: PathBuf,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        no_upload: bool,
        #[arg(long)]
        no_record: bool,
        #[arg(long)]
        allow_outside_project: bool,
    },
    Records {
        #[arg(long)]
        jsonl: PathBuf,
        #[arg(long, default_value_t = 10)]
        last: usize,
    },
}

pub fn run() -> Result<(), String> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Check {
            manifest,
            allow_outside_project,
        } => cmd_check(&manifest, allow_outside_project),
        Commands::Pack {
            manifest,
            allow_outside_project,
        } => cmd_pack(&manifest, allow_outside_project),
        Commands::Upload {
            manifest,
            dry_run,
            no_upload,
            no_record,
            allow_outside_project,
        } => cmd_upload(&manifest, dry_run, no_upload, no_record, allow_outside_project),
        Commands::Records { jsonl, last } => cmd_records(&jsonl, last),
    }
}

fn load_context(
    manifest_path: &Path,
    allow_outside_project: bool,
) -> Result<(Manifest, Vec<CollectedArtifact>, Vec<String>), String> {
    let manifest = parse_manifest(manifest_path)?;
    let project_root = env::current_dir()
        .map_err(|err| format!("resolve current directory failed: {err}"))?;
    let (artifacts, warnings) = collect_artifacts(&manifest, &project_root, allow_outside_project)?;
    Ok((manifest, artifacts, warnings))
}

fn status_str(v: &crate::collect::ArtifactStatus) -> &'static str {
    match v {
        crate::collect::ArtifactStatus::Included => "included",
        crate::collect::ArtifactStatus::Missing => "missing",
    }
}

fn print_artifacts(artifacts: &[CollectedArtifact]) {
    println!("name\ttype\trequired\tstatus\tpaths");
    for artifact in artifacts {
        let paths = if artifact.matched_paths.is_empty() {
            "(none)".to_string()
        } else {
            artifact.matched_paths.join(",")
        };
        println!(
            "{}\t{}\t{}\t{}\t{}",
            artifact.name,
            artifact.artifact_type,
            artifact.required,
            status_str(&artifact.status),
            paths
        );
    }
}

fn remote_dir_for_run(manifest: &Manifest, timestamp: &str) -> String {
    let base = manifest.oss.remote_dir.trim_end_matches('/');
    format!("{base}/{timestamp}")
}

fn build_run_paths(run_id: &str, output_dir: &Path) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    (
        output_dir.join(format!("{run_id}.manifest.yaml")),
        output_dir.join(format!("{run_id}.git_info.json")),
        output_dir.join(format!("{run_id}.tar.gz")),
        output_dir.join(format!("{run_id}.meta.json")),
    )
}

fn write_metadata_snapshot(
    metadata: &Metadata,
    path: &Path,
) -> Result<(), String> {
    let text = serde_json::to_string_pretty(metadata)
        .map_err(|err| format!("serialize metadata failed: {err}"))?;
    File::create(path)
        .map_err(|err| format!("create metadata file failed: {err}"))?
        .write_all(text.as_bytes())
        .map_err(|err| format!("write metadata file failed: {err}"))?;
    Ok(())
}

fn cmd_check(manifest_path: &Path, allow_outside_project: bool) -> Result<(), String> {
    let (manifest, artifacts, warnings) = load_context(manifest_path, allow_outside_project)?;

    if manifest.oss.bucket.trim().is_empty() {
        return Err("oss.bucket is required".to_string());
    }
    if manifest.oss.region.trim().is_empty() {
        return Err("oss.region is required".to_string());
    }
    if manifest.oss.endpoint.trim().is_empty() {
        return Err("oss.endpoint is required".to_string());
    }

    std::fs::create_dir_all(&manifest.archive.output_dir)
        .map_err(|err| format!("create output dir failed: {err}"))?;

    print_artifacts(&artifacts);
    for w in &warnings {
        println!("warning: {w}");
    }

    match check_access(&manifest.oss.bucket, &manifest.oss.endpoint, &manifest.oss.region) {
        Ok((true, _)) => println!("ossutil ls check passed"),
        Ok((false, msg)) => println!("warning: ossutil ls check failed: {msg}"),
        Err(err) => println!("warning: ossutil ls check failed: {err}"),
    }

    println!("check complete");
    Ok(())
}

fn cmd_pack(manifest_path: &Path, allow_outside_project: bool) -> Result<(), String> {
    let (manifest, artifacts, warnings) = load_context(manifest_path, allow_outside_project)?;

    let timestamp = make_timestamp();
    let run_id = run_id_for_run(&manifest.run.name, Some(&timestamp));
    let output_dir = Path::new(&manifest.archive.output_dir);
    let remote_dir = remote_dir_for_run(&manifest, &timestamp);

    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("create output dir failed: {e}"))?;

    let (manifest_snapshot, git_snapshot, archive_path, metadata_path) = build_run_paths(&run_id, output_dir);
    let project_root = env::current_dir()
        .map_err(|err| format!("resolve current directory failed: {err}"))?;
    write_manifest_snapshot(&manifest.to_map()?, &manifest_snapshot)?;

    let git_info = read_git_info();
    let git_text = serde_json::json!({
        "available": git_info.available,
        "repo_root": git_info.repo_root,
        "commit": git_info.commit,
        "branch": git_info.branch,
        "dirty": git_info.dirty,
    });
    File::create(&git_snapshot)
        .map_err(|err| format!("create git snapshot failed: {err}"))?
        .write_all(
            serde_json::to_string_pretty(&git_text)
                .map_err(|err| format!("serialize git info failed: {err}"))?
                .as_bytes(),
        )
        .map_err(|err| format!("write git snapshot failed: {err}"))?;

    let initial_metadata = build_metadata(
        &manifest,
        &run_id,
        &format!("{run_id}.tar.gz"),
        archive_path
            .to_str()
            .ok_or_else(|| "invalid archive path".to_string())?,
        0,
        "",
        &remote_dir,
        &artifacts,
        &git_info,
        "not_uploaded",
        None,
        None,
        &warnings,
    );
    if manifest.archive.include_metadata {
        write_metadata_snapshot(&initial_metadata, &metadata_path)?;
    }

    let archive_path = create_archive(
        &manifest,
        &artifacts,
        &run_id,
        &manifest.run.name,
        output_dir,
        &project_root,
        Some(&manifest_snapshot),
        Some(&git_snapshot),
        if manifest.archive.include_metadata {
            Some(&metadata_path)
        } else {
            None
        },
    )?;

    let (sha, size_bytes) = compute_sha256(&archive_path)?;
    let sha_path = write_sha256(&archive_path, &sha)?;

    let final_metadata = build_metadata(
        &manifest,
        &run_id,
        archive_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| "invalid archive filename".to_string())?,
        &archive_path.to_string_lossy(),
        size_bytes,
        &sha,
        &remote_dir,
        &artifacts,
        &git_info,
        "not_uploaded",
        None,
        None,
        &warnings,
    );
    write_metadata_snapshot(&final_metadata, &metadata_path)?;

    println!("archive: {}", archive_path.display());
    println!("metadata: {}", metadata_path.display());
    println!("sha256: {}", sha_path.display());
    for item in warnings {
        println!("warning: {item}");
    }
    Ok(())
}

fn cmd_upload(
    manifest_path: &Path,
    dry_run: bool,
    no_upload: bool,
    no_record: bool,
    allow_outside_project: bool,
) -> Result<(), String> {
    let (manifest, artifacts, warnings) = load_context(manifest_path, allow_outside_project)?;
    let run_start = Instant::now();

    let timestamp = make_timestamp();
    let run_id = run_id_for_run(&manifest.run.name, Some(&timestamp));
    let output_dir = Path::new(&manifest.archive.output_dir);
    let remote_dir = remote_dir_for_run(&manifest, &timestamp);

    if dry_run {
        println!("manifest: {}", manifest_path.display());
        print_artifacts(&artifacts);
        for w in &warnings {
            println!("warning: {w}");
        }
        println!("planned run_id: {run_id}");
        println!("planned remote dir: {}", oss_uri(&manifest.oss.bucket, &remote_dir, ""));
        return Ok(());
    }

    std::fs::create_dir_all(output_dir)
        .map_err(|err| format!("create output dir failed: {err}"))?;
    let (manifest_snapshot, git_snapshot, archive_path, metadata_path) = build_run_paths(&run_id, output_dir);
    let project_root = env::current_dir()
        .map_err(|err| format!("resolve current directory failed: {err}"))?;

    write_manifest_snapshot(&manifest.to_map()?, &manifest_snapshot)?;

    let git_info = read_git_info();
    let git_text = serde_json::json!({
        "available": git_info.available,
        "repo_root": git_info.repo_root,
        "commit": git_info.commit,
        "branch": git_info.branch,
        "dirty": git_info.dirty,
    });
    File::create(&git_snapshot)
        .map_err(|err| format!("create git snapshot failed: {err}"))?
        .write_all(
            serde_json::to_string_pretty(&git_text)
                .map_err(|err| format!("serialize git info failed: {err}"))?
                .as_bytes(),
        )
        .map_err(|err| format!("write git snapshot failed: {err}"))?;

    let placeholder_metadata = build_metadata(
        &manifest,
        &run_id,
        &format!("{run_id}.tar.gz"),
        &archive_path.to_string_lossy(),
        0,
        "",
        &remote_dir,
        &artifacts,
        &git_info,
        "pending",
        None,
        None,
        &warnings,
    );
    if manifest.archive.include_metadata {
        write_metadata_snapshot(&placeholder_metadata, &metadata_path)?;
    }

    let archive_path = create_archive(
        &manifest,
        &artifacts,
        &run_id,
        &manifest.run.name,
        output_dir,
        &project_root,
        Some(&manifest_snapshot),
        Some(&git_snapshot),
        if manifest.archive.include_metadata {
            Some(&metadata_path)
        } else {
            None
        },
    )?;

    let (sha, size_bytes) = compute_sha256(&archive_path)?;
    let sha_path = write_sha256(&archive_path, &sha)?;

    let upload_start = Instant::now();
    let archive_name = archive_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "invalid archive filename".to_string())?;
    let metadata_name = format!("{run_id}.meta.json");
    let sha_name = format!("{run_id}.sha256");

    if no_upload {
        let final_metadata = build_metadata(
            &manifest,
            &run_id,
            archive_name,
            &archive_path.to_string_lossy(),
            size_bytes,
            &sha,
            &remote_dir,
            &artifacts,
            &git_info,
            "not_uploaded",
            Some(run_start.elapsed().as_secs_f64()),
            None,
            &warnings,
        );
        if manifest.archive.include_metadata {
            write_metadata_snapshot(&final_metadata, &metadata_path)?;
        }
        println!("archive: {}", archive_path.display());
        println!("metadata: {}", metadata_path.display());
        println!("sha256: {}", sha_path.display());
        println!("duration: {:.2}s", run_start.elapsed().as_secs_f64());
        println!("avg MiB/s: {:.2}", 0.0);
        return Ok(());
    }

    let archive_upload = upload_file(&archive_path, &manifest, &remote_dir, archive_name)
        .map_err(|err| format!("upload archive failed: {err}"))?;
    let metadata_upload = upload_file(&metadata_path, &manifest, &remote_dir, &metadata_name)
        .map_err(|err| format!("upload metadata failed: {err}"))?;
    let sha_upload = upload_file(&sha_path, &manifest, &remote_dir, &sha_name)
        .map_err(|err| format!("upload sha256 failed: {err}"))?;

    let duration = upload_start.elapsed().as_secs_f64();
    let avg = [archive_upload.avg_mib_s, metadata_upload.avg_mib_s, sha_upload.avg_mib_s]
        .into_iter()
        .find(|x| x.is_some())
        .flatten();

    let avg = avg.or_else(|| {
        if duration > 0.0 {
            Some((size_bytes as f64) / (1024f64 * 1024f64) / duration)
        } else {
            None
        }
    });

    let final_metadata = build_metadata(
        &manifest,
        &run_id,
        archive_name,
        &archive_path.to_string_lossy(),
        size_bytes,
        &sha,
        &remote_dir,
        &artifacts,
        &git_info,
        "success",
        Some(run_start.elapsed().as_secs_f64()),
        avg,
        &warnings,
    );
    write_metadata_snapshot(&final_metadata, &metadata_path)?;

    let summary_uri = oss_uri(&manifest.oss.bucket, &remote_dir, archive_name);
    let duration = run_start.elapsed().as_secs_f64();
    let avg_mib_s = avg;

    println!("archive: {}", archive_path.display());
    println!("metadata: {}", metadata_path.display());
    println!("sha256: {}", sha_path.display());
    println!("duration: {:.2}s", duration);
    println!("avg MiB/s: {:.2}", avg_mib_s.unwrap_or(0.0));
    println!("remote uri: {summary_uri}");

    if !no_record {
        let record = serde_json::json!({
            "time": final_metadata.timestamp,
            "run_id": final_metadata.run_id,
            "run_name": final_metadata.run_name,
            "project": final_metadata.project,
            "archive_uri": summary_uri,
            "metadata_uri": final_metadata.oss.metadata_uri,
            "sha256": sha,
            "size_bytes": size_bytes,
            "avg_mib_s": avg_mib_s,
            "status": "success",
        });
        append_jsonl(Path::new(&manifest.records.jsonl), &record)?;
        append_markdown(Path::new(&manifest.records.markdown), &record)?;
    }

    Ok(())
}

fn cmd_records(jsonl: &Path, last: usize) -> Result<(), String> {
    let rows = read_records(jsonl, last);
    if rows.is_empty() {
        println!("No records found.");
        return Ok(());
    }

    println!("Time | Run | Project | Size | URI");
    for row in rows {
        println!(
            "{} | {} | {} | {:.1} MiB | {}",
            row.time,
            row.run_name,
            row.project,
            size_to_mib(row.size_bytes),
            row.archive_uri,
        );
    }
    Ok(())
}

fn size_to_mib(size_bytes: u64) -> f64 {
    (size_bytes as f64) / (1024f64 * 1024f64)
}

impl RecordRow {
    #[allow(dead_code)]
    pub fn sample_line(&self) -> String {
        format!(
            "{} | {} | {} | {:.1} MiB | {}",
            self.time,
            self.run_name,
            self.project,
            size_to_mib(self.size_bytes),
            self.archive_uri
        )
    }
}
