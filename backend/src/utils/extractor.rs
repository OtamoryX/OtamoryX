use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use tracing::debug;

pub struct ArchiveExtractor;

#[derive(Debug, Clone)]
pub struct ExtractedFile {
    pub name: String,
    pub data: Vec<u8>,
    pub size: usize,
}

impl ArchiveExtractor {
    pub fn new() -> Self {
        Self
    }

    pub fn extract_files<P: AsRef<Path>>(&self, archive_path: P) -> Result<Vec<ExtractedFile>> {
        let path = archive_path.as_ref();
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        debug!(
            "Extracting archive: {}, format: {}",
            path.display(),
            extension
        );

        match extension.as_str() {
            "cbz" | "zip" => self.extract_zip(path),
            "cbr" | "rar" => self.extract_rar(path),
            "cb7" | "7z" => self.extract_7z(path),
            // 暂不支持的格式，返回错误
            "cbt" | "tar" => Err(anyhow::anyhow!("TAR format not yet implemented")),
            "pdf" => Err(anyhow::anyhow!("PDF format not yet implemented")),
            _ => Err(anyhow::anyhow!(
                "Unsupported archive format: {}. Supported formats: CBZ, CBR, CB7, CBT, PDF",
                extension
            )),
        }
    }

    pub fn get_image_files(&self, files: Vec<ExtractedFile>) -> Vec<ExtractedFile> {
        files
            .into_iter()
            .filter(|file| self.is_image_file(&file.name))
            .collect()
    }

    fn is_image_file(&self, filename: &str) -> bool {
        let extension = Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        matches!(
            extension.as_str(),
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp"
        )
    }

    fn extract_zip<P: AsRef<Path>>(&self, path: P) -> Result<Vec<ExtractedFile>> {
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let mut archive = zip::ZipArchive::new(reader)?;

        let mut files = Vec::new();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;

            if file.is_file() {
                let mut contents = Vec::new();
                file.read_to_end(&mut contents)?;

                files.push(ExtractedFile {
                    name: file.name().to_string(),
                    size: contents.len(),
                    data: contents,
                });
            }
        }

        debug!("Extracted {} files from ZIP archive", files.len());
        Ok(files)
    }

    fn extract_rar<P: AsRef<Path>>(&self, _path: P) -> Result<Vec<ExtractedFile>> {
        // Temporarily disable RAR support until we can fix the API usage
        Err(anyhow::anyhow!(
            "RAR format temporarily disabled due to API incompatibility"
        ))
    }

    fn extract_7z<P: AsRef<Path>>(&self, path: P) -> Result<Vec<ExtractedFile>> {
        use sevenz_rust::{Password, SevenZReader};

        let mut archive =
            SevenZReader::open(&path, Password::empty()).context("Failed to open 7z archive")?;

        let mut files = Vec::new();

        archive.for_each_entries(|entry, reader| {
            if entry.is_directory() {
                return Ok(true);
            }

            let filename = entry.name();
            if !self.is_image_file(filename) {
                return Ok(true);
            }

            let mut contents = Vec::new();
            let _bytes_copied =
                std::io::copy(reader, &mut contents).map_err(|e| sevenz_rust::Error::from(e))?;

            files.push(ExtractedFile {
                name: filename.to_string(),
                size: contents.len(),
                data: contents,
            });

            Ok(true)
        })?;

        debug!("Extracted {} files from 7Z archive", files.len());
        Ok(files)
    }
}
