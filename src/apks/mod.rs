use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use tempfile::TempDir;
use zip::ZipArchive;

#[derive(Debug)]
pub struct PreparedApkSet {
    pub apk_paths: Vec<PathBuf>,
    pub obb_files: Vec<ObbFile>,
    _temp_dir: Option<TempDir>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ObbFile {
    pub local_path: PathBuf,
    pub relative_device_path: PathBuf,
}

impl ObbFile {
    pub fn device_relative_path(&self, package_name: &str) -> PathBuf {
        if self.relative_device_path.components().count() > 1 {
            self.relative_device_path.clone()
        } else {
            PathBuf::from(package_name).join(&self.relative_device_path)
        }
    }
}

impl PreparedApkSet {
    pub fn from_inputs(inputs: &[PathBuf]) -> Result<Self> {
        anyhow::ensure!(
            !inputs.is_empty(),
            "at least one APK, .apks, or .xapk path is required"
        );

        let uses_archive = inputs.iter().any(|path| is_archive_input(path));
        let temp_dir = if uses_archive {
            Some(tempfile::tempdir().context("failed to create temporary APK workspace")?)
        } else {
            None
        };

        let mut apk_paths = Vec::new();
        let mut obb_files = Vec::new();

        for input in inputs {
            anyhow::ensure!(input.is_file(), "APK input not found: {}", input.display());

            match input_extension(input).as_deref() {
                Some("apk") => apk_paths.push(input.clone()),
                Some("apks") => {
                    extract_archive_entries(
                        input,
                        temp_dir
                            .as_ref()
                            .expect("archive extraction requires a temp dir")
                            .path(),
                        ArchiveKind::Apks,
                        &mut apk_paths,
                        &mut obb_files,
                    )?;
                }
                Some("xapk") => {
                    extract_archive_entries(
                        input,
                        temp_dir
                            .as_ref()
                            .expect("archive extraction requires a temp dir")
                            .path(),
                        ArchiveKind::Xapk,
                        &mut apk_paths,
                        &mut obb_files,
                    )?;
                }
                _ => bail!(
                    "unsupported input '{}'; expected .apk, .apks, or .xapk",
                    input.display()
                ),
            }
        }

        anyhow::ensure!(
            !apk_paths.is_empty(),
            "no APK entries were found in the provided inputs"
        );

        Ok(Self {
            apk_paths,
            obb_files,
            _temp_dir: temp_dir,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ArchiveKind {
    Apks,
    Xapk,
}

fn extract_archive_entries(
    archive_path: &Path,
    workspace_root: &Path,
    kind: ArchiveKind,
    apk_paths: &mut Vec<PathBuf>,
    obb_files: &mut Vec<ObbFile>,
) -> Result<()> {
    let archive_file = File::open(archive_path)
        .with_context(|| format!("failed to open archive '{}'", archive_path.display()))?;
    let mut archive = ZipArchive::new(archive_file)
        .with_context(|| format!("failed to read archive '{}'", archive_path.display()))?;
    let archive_label = sanitize_component(
        archive_path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("archive"),
    );
    let archive_dir = workspace_root.join(archive_label);
    fs::create_dir_all(&archive_dir)
        .with_context(|| format!("failed to create '{}'", archive_dir.display()))?;

    let mut extracted_apks = Vec::new();
    let mut found_apk = false;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).with_context(|| {
            format!(
                "failed to read entry {} from '{}'",
                index,
                archive_path.display()
            )
        })?;

        if !entry.is_file() {
            continue;
        }

        let Some(enclosed_name) = entry.enclosed_name() else {
            continue;
        };

        let lower_name = enclosed_name.to_string_lossy().to_ascii_lowercase();
        if lower_name.ends_with(".apk") {
            let output_path = extracted_member_path(&archive_dir, index, &enclosed_name);
            write_zip_entry(&mut entry, &output_path)?;
            extracted_apks.push((
                apk_entry_priority(&enclosed_name),
                enclosed_name.to_string_lossy().into_owned(),
                output_path,
            ));
            found_apk = true;
            continue;
        }

        if kind == ArchiveKind::Xapk && lower_name.ends_with(".obb") {
            let output_path = extracted_member_path(&archive_dir, index, &enclosed_name);
            write_zip_entry(&mut entry, &output_path)?;
            obb_files.push(ObbFile {
                local_path: output_path,
                relative_device_path: obb_relative_device_path(&enclosed_name),
            });
        }
    }

    anyhow::ensure!(
        found_apk,
        "archive '{}' does not contain any APK entries",
        archive_path.display()
    );

    extracted_apks.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
    apk_paths.extend(extracted_apks.into_iter().map(|(_, _, path)| path));
    Ok(())
}

fn write_zip_entry<R: io::Read>(entry: &mut R, output_path: &Path) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create '{}'", parent.display()))?;
    }
    let mut output = File::create(output_path)
        .with_context(|| format!("failed to create '{}'", output_path.display()))?;
    io::copy(entry, &mut output)
        .with_context(|| format!("failed to extract '{}'", output_path.display()))?;
    Ok(())
}

fn extracted_member_path(archive_dir: &Path, index: usize, enclosed_name: &Path) -> PathBuf {
    let file_name = enclosed_name
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("member.bin");
    archive_dir.join(format!("{index}-{}", sanitize_component(file_name)))
}

fn obb_relative_device_path(enclosed_name: &Path) -> PathBuf {
    let normalized = enclosed_name.to_string_lossy().replace('\\', "/");
    for prefix in ["Android/obb/", "android/obb/", "obb/"] {
        if let Some(stripped) = normalized.strip_prefix(prefix) {
            return PathBuf::from(stripped);
        }
    }

    enclosed_name
        .file_name()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("main.obb"))
}

fn apk_entry_priority(enclosed_name: &Path) -> u8 {
    let file_name = enclosed_name
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if file_name == "base.apk"
        || file_name.starts_with("base-")
        || file_name.contains("split-base")
        || file_name.contains("base-master")
    {
        0
    } else {
        1
    }
}

fn sanitize_component(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '-' | '_' => character,
            _ => '_',
        })
        .collect()
}

fn input_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
}

fn is_archive_input(path: &Path) -> bool {
    matches!(input_extension(path).as_deref(), Some("apks" | "xapk"))
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File},
        io::Write,
        path::{Path, PathBuf},
    };

    use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

    use super::PreparedApkSet;

    #[test]
    fn single_apk_inputs_keep_the_fast_path() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let apk_path = temp_dir.path().join("app.apk");
        fs::write(&apk_path, b"apk").expect("write apk");

        let prepared = PreparedApkSet::from_inputs(std::slice::from_ref(&apk_path))
            .expect("single apk should resolve");

        assert_eq!(prepared.apk_paths, vec![apk_path]);
        assert!(prepared.obb_files.is_empty());
    }

    #[test]
    fn apks_archives_extract_all_apk_members_with_base_first() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let archive_path = temp_dir.path().join("bundle.apks");
        write_zip_archive(
            &archive_path,
            &[
                ("splits/config.en.apk", b"config".as_slice()),
                ("base.apk", b"base".as_slice()),
                ("toc.pb", b"ignored".as_slice()),
            ],
        );

        let prepared = PreparedApkSet::from_inputs(std::slice::from_ref(&archive_path))
            .expect("archive should resolve");

        let names: Vec<_> = prepared
            .apk_paths
            .iter()
            .map(|path| {
                path.file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default()
                    .to_owned()
            })
            .collect();
        assert_eq!(
            names,
            vec!["1-base.apk".to_owned(), "0-config.en.apk".to_owned()]
        );
        assert!(prepared.obb_files.is_empty());
    }

    #[test]
    fn xapk_archives_preserve_obb_target_paths() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let archive_path = temp_dir.path().join("bundle.xapk");
        write_zip_archive(
            &archive_path,
            &[
                ("app.apk", b"apk".as_slice()),
                (
                    "Android/obb/com.example.app/main.1.com.example.app.obb",
                    b"obb".as_slice(),
                ),
            ],
        );

        let prepared = PreparedApkSet::from_inputs(std::slice::from_ref(&archive_path))
            .expect("xapk should resolve");

        assert_eq!(prepared.apk_paths.len(), 1);
        assert_eq!(prepared.obb_files.len(), 1);
        assert_eq!(
            prepared.obb_files[0].device_relative_path("fallback.package"),
            PathBuf::from("com.example.app/main.1.com.example.app.obb")
        );
    }

    fn write_zip_archive(path: &Path, entries: &[(&str, &[u8])]) {
        let file = File::create(path).expect("archive");
        let mut archive = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

        for (name, bytes) in entries {
            archive.start_file(*name, options).expect("zip entry");
            archive.write_all(bytes).expect("zip bytes");
        }

        archive.finish().expect("finish archive");
    }
}
