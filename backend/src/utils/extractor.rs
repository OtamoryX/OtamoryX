use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use anyhow::{Context, Result};
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
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        debug!("Extracting archive: {}, format: {}", path.display(), extension);

        match extension.as_str() {
            "cbz" | "zip" => self.extract_zip(path),
            "cbr" | "rar" => self.extract_rar(path),
            "cb7" | "7z" => self.extract_7z(path),
            // 额外支持的格式
            "cbt" | "tar" => self.extract_tar(path),
            "pdf" => self.extract_pdf(path),
            _ => Err(anyhow::anyhow!("Unsupported archive format: {}. Supported formats: CBZ, CBR, CB7, CBT, PDF", extension)),
        }
    }

    pub fn get_image_files(&self, files: Vec<ExtractedFile>) -> Vec<ExtractedFile> {
        files.into_iter()
            .filter(|file| self.is_image_file(&file.name))
            .collect()
    }

    fn is_image_file(&self, filename: &str) -> bool {
        let extension = Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        matches!(extension.as_str(), "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp")
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

    fn extract_rar<P: AsRef<Path>>(&self, path: P) -> Result<Vec<ExtractedFile>> {
        use unrar::Archive;
        
        debug!("Extracting RAR archive: {}", path.as_ref().display());
        
        let mut files = Vec::new();
        let archive_path = path.as_ref().to_string_lossy().to_string();
        
        // 使用unrar库来处理RAR文件
        let mut archive = Archive::new(archive_path).list_files();
        
        // 处理归档内容
        let entries = archive.process()
            .map_err(|e| anyhow::anyhow!("Failed to process RAR archive: {}", e))?;
        
        for entry_result in entries {
            match entry_result {
                Ok(entry) => {
                    let filename = &entry.filename;
                    
                    // 跳过目录
                    if entry.is_directory() {
                        continue;
                    }
                    
                    // 只处理图片文件
                    if !self.is_image_file(filename) {
                        continue;
                    }
                    
                    // 现在需要重新打开档案来提取文件内容
                    let mut extract_archive = Archive::new(archive_path.clone()).extract_to_memory();
                    
                    match extract_archive.process() {
                        Ok(extract_results) => {
                            for extract_result in extract_results {
                                match extract_result {
                                    Ok((file_info, data)) => {
                                        if file_info.filename == *filename {
                                            files.push(ExtractedFile {
                                                name: filename.clone(),
                                                size: data.len(),
                                                data,
                                            });
                                            break; // 找到文件后退出
                                        }
                                    }
                                    Err(e) => {
                                        debug!("Failed to extract {}: {}", filename, e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            debug!("Failed to extract {}: {}", filename, e);
                        }
                    }
                }
                Err(e) => {
                    debug!("Failed to read entry: {}", e);
                    continue;
                }
            }
        }
        
        debug!("Extracted {} files from RAR archive", files.len());
        Ok(files)
    }

    fn extract_7z<P: AsRef<Path>>(&self, path: P) -> Result<Vec<ExtractedFile>> {
        use sevenz_rust::{SevenZReader, Password};
        
        let mut archive = SevenZReader::open(&path, Password::empty())
            .context("Failed to open 7z archive")?;
        
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
            let _bytes_copied = std::io::copy(reader, &mut contents)
                .map_err(|e| sevenz_rust::Error::from(e))?;
            
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