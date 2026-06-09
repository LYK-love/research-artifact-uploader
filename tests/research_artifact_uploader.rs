use std::fs;
use tempfile::tempdir;
use flate2::read::GzDecoder;
use sha2::Digest;

use research_artifact_uploader::{archive::{compute_sha256, create_archive, make_timestamp, run_id_for_run}, collect::collect_artifacts, manifest::parse_manifest};

#[test]
fn test_parse_manifest_defaults_and_required() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("artifacts.yaml");
    fs::write(
        &path,
        "run:\n  name: demo\n  project: proj\noss:\n  remote_dir: artifacts/test/demo\n",
    )
    .unwrap();
    let manifest = parse_manifest(&path).unwrap();
    assert_eq!(manifest.run.name, "demo");
    assert_eq!(manifest.archive.output_dir, ".rau/archives");
}

#[test]
fn test_required_file_missing_fails() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("artifacts.yaml");
    fs::write(
        &path,
        "run:\n  name: demo\n  project: proj\noss:\n  remote_dir: artifacts/test/demo\nartifacts:\n  - name: missing\n    path: absent.bin\n    type: file\n    required: true\n",
    )
    .unwrap();
    let manifest = parse_manifest(&path).unwrap();
    let err = collect_artifacts(&manifest, dir.path(), false).unwrap_err();
    assert!(err.contains("required file artifact missing"));
}

#[test]
fn test_optional_glob_missing_warns() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("artifacts.yaml");
    fs::write(
        &path,
        "run:\n  name: demo\n  project: proj\noss:\n  remote_dir: artifacts/test/demo\nartifacts:\n  - name: vids\n    path: videos/*.mp4\n    type: glob\n    required: false\n",
    )
    .unwrap();
    let manifest = parse_manifest(&path).unwrap();
    let (items, warnings) = collect_artifacts(&manifest, dir.path(), false).unwrap();
    assert_eq!(items[0].matched_count(), 0);
    assert!(!warnings.is_empty());
}

#[test]
fn test_metadata_does_not_contain_secret_like_keys() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("artifacts.yaml");
    fs::write(
        &path,
        "run:\n  name: demo\n  project: proj\noss:\n  remote_dir: artifacts/test/demo\n",
    )
    .unwrap();
    let manifest = parse_manifest(&path).unwrap();
    let json = serde_json::to_value(manifest).unwrap();
    let dumped = serde_json::to_string(&json).unwrap();
    let lowered = dumped.to_lowercase();
    assert!(!lowered.contains("accesskey"));
    assert!(!lowered.contains("secret"));
}

#[test]
fn test_archive_has_expected_internal_paths() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("run/ckpt")).unwrap();
    fs::write(dir.path().join("run/ckpt/model.bin"), b"model").unwrap();
    fs::write(dir.path().join("run/metrics.jsonl"), b"{}\n").unwrap();

    let path = dir.path().join("artifacts.yaml");
    fs::write(
        &path,
        "run:\n  name: demo\n  project: proj\noss:\n  remote_dir: artifacts/test/demo\nartifacts:\n  - name: ckpt\n    path: run/ckpt\n    type: directory\n    required: true\n  - name: metrics\n    path: run/metrics.jsonl\n    type: file\n    required: true\n",
    )
    .unwrap();

    let manifest = parse_manifest(&path).unwrap();
    let (items, _) = collect_artifacts(&manifest, dir.path(), false).unwrap();

    let out_dir = dir.path().join(".rau/archives");
    let run_id = run_id_for_run("demo", Some(&make_timestamp()));
    let archive = create_archive(
        &manifest,
        &items,
        &run_id,
        "demo",
        &out_dir,
        dir.path(),
        None,
        None,
        None,
    )
    .unwrap();

    let file = fs::File::open(&archive).unwrap();
    let decoder = GzDecoder::new(file);
    let mut arc = tar::Archive::new(decoder);
    let mut names = Vec::new();
    for entry in arc.entries().unwrap() {
        let entry = entry.unwrap();
        names.push(entry.path().unwrap().to_string_lossy().to_string());
    }

    assert!(names.contains(&"demo/artifacts/ckpt/model.bin".to_string()));
    assert!(names.contains(&"demo/artifacts/metrics/metrics.jsonl".to_string()));
}

#[test]
fn test_sha256_calculation() {
    let dir = tempdir().unwrap();
    let p = dir.path().join("x.bin");
    fs::write(&p, b"abc").unwrap();
    let (sha, size) = compute_sha256(&p).unwrap();
    assert_eq!(size, 3);
    assert_eq!(sha, format!("{:x}", sha2::Sha256::digest(b"abc")));
}
