use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use tracing::debug;

/// Maximum allowed size for a single extracted file (50 MB)
const MAX_SINGLE_FILE_SIZE: u64 = 50 * 1024 * 1024;

pub struct ArchiveExtractor;

#[derive(Debug, Clone)]
pub struct ExtractedFile {
    pub name: String,
    pub data: Vec<u8>,
    pub size: usize,
}

/// Standalone helper to check if a filename has an image extension.
/// Used by both instance methods and static single-page extraction methods.
fn is_image_file(filename: &str) -> bool {
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

/// Helper: resolve the archive format extension from a path, returns lowercase.
fn get_archive_extension(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default()
}

/// Validate that an archive path exists and is non-empty. Returns an error otherwise.
fn validate_archive_path(path: &Path) -> Result<()> {
    if !path.exists() {
        tracing::error!("Archive file does not exist: {}", path.display());
        return Err(anyhow::anyhow!("Archive file does not exist"));
    }
    if let Ok(metadata) = std::fs::metadata(path) {
        if metadata.len() == 0 {
            tracing::error!("Archive file is empty: {}", path.display());
            return Err(anyhow::anyhow!("Archive file is empty"));
        }
    }
    Ok(())
}

impl ArchiveExtractor {
    pub fn new() -> Self {
        Self
    }

    // -----------------------------------------------------------------------
    // Full extraction (legacy, kept for backward-compatibility)
    // -----------------------------------------------------------------------

    pub fn extract_files<P: AsRef<Path>>(&self, archive_path: P) -> Result<Vec<ExtractedFile>> {
        let path = archive_path.as_ref();
        let extension = get_archive_extension(path);

        debug!(
            "Extracting archive: {}, format: {}",
            path.display(),
            extension
        );

        validate_archive_path(path)?;

        let result = match extension.as_str() {
            "cbz" | "zip" => {
                debug!("Using ZIP extraction method");
                self.extract_zip(path)
            }
            "cbr" | "rar" => {
                debug!("Using RAR extraction method");
                self.extract_rar(path)
            }
            "cb7" | "7z" => {
                debug!("Using 7Z extraction method");
                self.extract_7z(path)
            }
            "cbt" | "tar" => {
                tracing::error!("TAR format not yet implemented for: {}", path.display());
                Err(anyhow::anyhow!("TAR format not yet implemented"))
            }
            "pdf" => {
                tracing::error!("PDF format not yet implemented for: {}", path.display());
                Err(anyhow::anyhow!("PDF format not yet implemented"))
            }
            _ => {
                tracing::error!(
                    "Unsupported archive format '{}' for: {}",
                    extension,
                    path.display()
                );
                Err(anyhow::anyhow!(
                    "Unsupported archive format: {}. Supported formats: CBZ, CBR, CB7, CBT, PDF",
                    extension
                ))
            }
        };

        match &result {
            Ok(files) => {
                debug!("Extraction successful: {} files extracted", files.len());
            }
            Err(e) => {
                tracing::error!("Extraction failed for {}: {:?}", path.display(), e);
            }
        }

        result
    }

    pub fn get_image_files(&self, files: Vec<ExtractedFile>) -> Vec<ExtractedFile> {
        files
            .into_iter()
            .filter(|file| is_image_file(&file.name))
            .collect()
    }

    #[allow(dead_code)]
    fn is_image_file_instance(&self, filename: &str) -> bool {
        is_image_file(filename)
    }

    // -----------------------------------------------------------------------
    // Single-page extraction (new -- memory efficient)
    // -----------------------------------------------------------------------

    /// Extract a single page (by 0-based sorted image index) from any supported archive.
    /// Dispatches to the format-specific method based on file extension.
    pub fn extract_single_page<P: AsRef<Path>>(path: P, index: usize) -> Result<ExtractedFile> {
        let path = path.as_ref();
        validate_archive_path(path)?;

        let extension = get_archive_extension(path);
        debug!(
            "Single-page extraction: archive={}, format={}, index={}",
            path.display(),
            extension,
            index
        );

        match extension.as_str() {
            "cbz" | "zip" => Self::extract_single_file_zip(path, index),
            "cb7" | "7z" => Self::extract_single_file_7z(path, index),
            "cbr" | "rar" => Err(anyhow::anyhow!(
                "RAR format temporarily disabled due to API incompatibility"
            )),
            "cbt" | "tar" => Err(anyhow::anyhow!("TAR format not yet implemented")),
            "pdf" => Err(anyhow::anyhow!("PDF format not yet implemented")),
            _ => Err(anyhow::anyhow!("Unsupported archive format: {}", extension)),
        }
    }

    /// Get the total number of image pages in an archive without reading file contents.
    /// Returns (total_image_count, sorted_filenames).
    pub fn get_page_count<P: AsRef<Path>>(path: P) -> Result<(usize, Vec<String>)> {
        let path = path.as_ref();
        validate_archive_path(path)?;

        let extension = get_archive_extension(path);

        match extension.as_str() {
            "cbz" | "zip" => Self::get_page_count_zip(path),
            "cb7" | "7z" => Self::get_page_count_7z(path),
            "cbr" | "rar" => Err(anyhow::anyhow!(
                "RAR format temporarily disabled due to API incompatibility"
            )),
            _ => Err(anyhow::anyhow!("Unsupported archive format: {}", extension)),
        }
    }

    /// Extract a single image file from a ZIP/CBZ archive by sorted image index.
    fn extract_single_file_zip(path: &Path, index: usize) -> Result<ExtractedFile> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut archive = zip::ZipArchive::new(reader)?;

        // Build sorted list of image entries: (zip_internal_index, filename)
        let mut image_entries: Vec<(usize, String)> = Vec::new();
        for i in 0..archive.len() {
            let entry = archive.by_index(i)?;
            if !entry.is_file() {
                continue;
            }
            let name = entry.name().to_string();
            if is_image_file(&name) {
                image_entries.push((i, name));
            }
        }
        image_entries.sort_by(|a, b| natord::compare(&a.1, &b.1));

        if index >= image_entries.len() {
            return Err(anyhow::anyhow!(
                "Page index {} out of range (total: {})",
                index,
                image_entries.len()
            ));
        }

        let (archive_index, name) = &image_entries[index];
        let mut entry = archive.by_index(*archive_index)?;

        // Size guard
        if entry.size() > MAX_SINGLE_FILE_SIZE {
            return Err(anyhow::anyhow!(
                "File too large: {} bytes (limit: {} bytes)",
                entry.size(),
                MAX_SINGLE_FILE_SIZE
            ));
        }

        let mut contents = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut contents)?;

        debug!(
            "Extracted single page {} from ZIP: {} ({}KB)",
            index,
            name,
            contents.len() / 1024
        );

        Ok(ExtractedFile {
            name: name.clone(),
            size: contents.len(),
            data: contents,
        })
    }

    /// Extract a single image file from a 7Z/CB7 archive by sorted image index.
    ///
    /// Note: The sevenz-rust crate's `for_each_entries` API iterates all entries
    /// sequentially but we only read the data for the target entry, skipping the
    /// rest as quickly as the API allows. This is still better than storing all
    /// pages in memory simultaneously.
    fn extract_single_file_7z(path: &Path, index: usize) -> Result<ExtractedFile> {
        use sevenz_rust::{Password, SevenZReader};

        // First pass: collect and sort image entry names to determine target
        let mut archive =
            SevenZReader::open(path, Password::empty()).context("Failed to open 7z archive")?;

        let mut image_names: Vec<String> = Vec::new();
        archive.for_each_entries(|entry, _reader| {
            if !entry.is_directory() {
                let name = entry.name().to_string();
                if is_image_file(&name) {
                    image_names.push(name);
                }
            }
            Ok(true)
        })?;
        image_names.sort_by(|a, b| natord::compare(a, b));

        if index >= image_names.len() {
            return Err(anyhow::anyhow!(
                "Page index {} out of range (total: {})",
                index,
                image_names.len()
            ));
        }

        let target_name = image_names[index].clone();

        // Second pass: extract only the target file
        let mut archive =
            SevenZReader::open(path, Password::empty()).context("Failed to reopen 7z archive")?;

        let mut result: Option<ExtractedFile> = None;

        archive.for_each_entries(|entry, reader| {
            if entry.is_directory() {
                return Ok(true);
            }

            let name = entry.name().to_string();
            if name == target_name {
                let mut contents = Vec::new();
                let mut total_read = 0u64;
                let mut buffer = [0u8; 8192];

                loop {
                    let n = reader
                        .read(&mut buffer)
                        .map_err(|e| sevenz_rust::Error::from(e))?;
                    if n == 0 {
                        break;
                    }

                    total_read += n as u64;
                    if total_read > MAX_SINGLE_FILE_SIZE {
                        return Err(sevenz_rust::Error::from(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!(
                                "File too large: {} bytes (limit: {} bytes)",
                                total_read, MAX_SINGLE_FILE_SIZE
                            ),
                        )));
                    }

                    contents.extend_from_slice(&buffer[..n]);
                }

                result = Some(ExtractedFile {
                    name: name.clone(),
                    size: contents.len(),
                    data: contents,
                });
                return Ok(false); // stop iteration -- we have what we need
            }

            Ok(true)
        })?;

        match result {
            Some(file) => {
                debug!(
                    "Extracted single page {} from 7Z: {} ({}KB)",
                    index,
                    file.name,
                    file.size / 1024
                );
                Ok(file)
            }
            None => Err(anyhow::anyhow!(
                "Target file '{}' not found during extraction pass",
                target_name
            )),
        }
    }

    /// Get page count for ZIP/CBZ without reading file contents.
    fn get_page_count_zip(path: &Path) -> Result<(usize, Vec<String>)> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut archive = zip::ZipArchive::new(reader)?;

        let mut names: Vec<String> = Vec::new();
        for i in 0..archive.len() {
            let entry = archive.by_index(i)?;
            if !entry.is_file() {
                continue;
            }
            let name = entry.name().to_string();
            if is_image_file(&name) {
                names.push(name);
            }
        }
        names.sort_by(|a, b| natord::compare(a, b));

        Ok((names.len(), names))
    }

    /// Get page count for 7Z/CB7 without reading file contents.
    fn get_page_count_7z(path: &Path) -> Result<(usize, Vec<String>)> {
        use sevenz_rust::{Password, SevenZReader};

        let mut archive =
            SevenZReader::open(path, Password::empty()).context("Failed to open 7z archive")?;

        let mut names: Vec<String> = Vec::new();
        archive.for_each_entries(|entry, _reader| {
            if !entry.is_directory() {
                let name = entry.name().to_string();
                if is_image_file(&name) {
                    names.push(name);
                }
            }
            Ok(true)
        })?;
        names.sort_by(|a, b| natord::compare(a, b));

        Ok((names.len(), names))
    }

    // -----------------------------------------------------------------------
    // Full extraction (private format-specific methods)
    // -----------------------------------------------------------------------

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
            if !is_image_file(filename) {
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
