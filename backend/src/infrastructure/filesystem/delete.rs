/// Delete physical archive file.
/// `NotFound` is treated as success, other IO errors are returned to caller.
pub async fn delete_archive_file(archive_path: &str) -> Result<(), std::io::Error> {
    match tokio::fs::remove_file(archive_path).await {
        Ok(_) => {
            tracing::info!("Deleted archive file: {}", archive_path);
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!(
                "Archive file already missing, skip delete: {}",
                archive_path
            );
            Ok(())
        }
        Err(e) => {
            tracing::warn!("Failed to delete archive file {}: {}", archive_path, e);
            Err(e)
        }
    }
}
