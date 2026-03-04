use crate::models::ScanSettings;
use crate::services::{ArchiveCacheService, ArchiveProcessingService};
use anyhow::{Context, Result};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use sqlx::{Pool, Sqlite};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, trace, warn};

/// 文件系统监控服务
pub struct FileMonitorService {
    processing_service: Arc<ArchiveProcessingService>,
    archive_cache: Arc<ArchiveCacheService>,
    watcher: Arc<RwLock<Option<RecommendedWatcher>>>,
    settings: Arc<RwLock<ScanSettings>>,
}

#[derive(Debug)]
pub struct FileEvent {
    pub path: PathBuf,
    pub kind: FileEventKind,
}

#[derive(Debug)]
pub enum FileEventKind {
    Created,
    Modified,
    Removed,
}

impl FileMonitorService {
    pub fn new(db: Pool<Sqlite>, archive_cache: Arc<ArchiveCacheService>) -> Self {
        Self {
            processing_service: Arc::new(ArchiveProcessingService::new(db.clone())),
            archive_cache,
            watcher: Arc::new(RwLock::new(None)),
            settings: Arc::new(RwLock::new(ScanSettings::default())),
        }
    }

    /// 启动文件监控
    pub async fn start_monitoring(
        &self,
        comics_path: &str,
        scan_settings: ScanSettings,
    ) -> Result<()> {
        info!("Starting file system monitoring for path: {}", comics_path);

        // 更新设置
        *self.settings.write().await = scan_settings.clone();

        // 如果实时监控被禁用，停止监控
        if !scan_settings.realtime_monitoring {
            self.stop_monitoring().await?;
            return Ok(());
        }

        let path = Path::new(comics_path);
        if !path.exists() {
            return Err(anyhow::anyhow!("监控路径不存在: {}", comics_path));
        }
        if !path.is_dir() {
            return Err(anyhow::anyhow!("监控路径不是目录: {}", comics_path));
        }

        // 停止现有的监控器
        self.stop_monitoring().await?;

        // 创建事件通道
        let (tx, mut rx) = mpsc::channel::<Result<Event, notify::Error>>(1000);

        // 创建监控器
        let mut watcher = RecommendedWatcher::new(
            move |res| {
                if let Err(e) = tx.blocking_send(res) {
                    error!("Failed to send file event: {}", e);
                }
            },
            Config::default().with_poll_interval(Duration::from_secs(1)),
        )
        .context("创建文件监控器失败")?;

        // 设置监控模式
        let mode = if scan_settings.recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        // 开始监控路径
        watcher
            .watch(path, mode)
            .with_context(|| format!("监控路径失败: {}", comics_path))?;

        // 保存监控器实例
        *self.watcher.write().await = Some(watcher);

        // 克隆必要的数据用于异步任务
        let processing_service = Arc::clone(&self.processing_service);
        let archive_cache = Arc::clone(&self.archive_cache);
        let settings_ref = Arc::clone(&self.settings);

        // 启动事件处理任务
        tokio::spawn(async move {
            info!("File monitor event handler started");

            while let Some(event_result) = rx.recv().await {
                match event_result {
                    Ok(event) => {
                        let settings = settings_ref.read().await;
                        if let Err(e) = Self::handle_file_event(
                            &event,
                            &settings,
                            &processing_service,
                            &archive_cache,
                        )
                        .await
                        {
                            error!("处理文件事件失败: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("File watch error: {}", e);
                    }
                }
            }

            info!("File monitor event handler stopped");
        });

        info!("File system monitoring started successfully");
        Ok(())
    }

    /// 停止文件监控
    pub async fn stop_monitoring(&self) -> Result<()> {
        let mut watcher_guard = self.watcher.write().await;
        if watcher_guard.take().is_some() {
            info!("File system monitoring stopped");
        }
        Ok(())
    }

    /// 更新监控设置
    pub async fn update_settings(
        &self,
        comics_path: &str,
        scan_settings: ScanSettings,
    ) -> Result<()> {
        info!("Updating file monitor settings");

        // 如果实时监控设置发生变化，重新启动监控
        let current_settings = self.settings.read().await;
        let needs_restart = current_settings.realtime_monitoring
            != scan_settings.realtime_monitoring
            || current_settings.recursive != scan_settings.recursive
            || current_settings.ignore_hidden != scan_settings.ignore_hidden;

        drop(current_settings);

        if needs_restart || scan_settings.realtime_monitoring {
            self.start_monitoring(comics_path, scan_settings).await?;
        } else {
            *self.settings.write().await = scan_settings;
        }

        Ok(())
    }

    /// 处理单个文件事件
    async fn handle_file_event(
        event: &Event,
        settings: &ScanSettings,
        processing_service: &ArchiveProcessingService,
        archive_cache: &ArchiveCacheService,
    ) -> Result<()> {
        debug!("File event: {:?}", event);

        // 如果自动扫描被禁用，忽略事件
        if !settings.enabled {
            return Ok(());
        }

        // 双端重命名事件包含 from/to 两个路径，需要成对处理。
        if let EventKind::Modify(notify::event::ModifyKind::Name(notify::event::RenameMode::Both)) =
            event.kind
        {
            if event.paths.len() >= 2 {
                let from_path = &event.paths[0];
                let to_path = &event.paths[1];

                Self::handle_file_rename_both(
                    from_path,
                    to_path,
                    settings,
                    processing_service,
                    archive_cache,
                )
                .await?;
            } else {
                warn!(
                    "RenameMode::Both event missing paths, expected 2 got {}",
                    event.paths.len()
                );
            }

            return Ok(());
        }

        for path in &event.paths {
            // 检查是否应该忽略隐藏文件
            if settings.ignore_hidden && Self::is_hidden_file(path) {
                debug!("Ignoring hidden file: {}", path.display());
                continue;
            }

            match event.kind {
                EventKind::Create(_) => {
                    // 对于创建事件，我们只记录但不处理，等待 Close(Write) 事件
                    if path.is_file() && crate::services::ArchiveService::is_supported_format(path)
                    {
                        debug!(
                            "New archive file created (waiting for write completion): {}",
                            path.display()
                        );
                    }
                }
                EventKind::Access(access_kind) => {
                    if let notify::event::AccessKind::Close(notify::event::AccessMode::Write) =
                        access_kind
                    {
                        // 文件写入完成，可以安全处理
                        if path.is_file()
                            && crate::services::ArchiveService::is_supported_format(path)
                        {
                            info!("Archive file write completed: {}", path.display());
                            Self::handle_file_created(path, processing_service).await?;
                        }
                    } else {
                        trace!(
                            "Ignoring access event: {:?} for {}",
                            access_kind,
                            path.display()
                        );
                    }
                }
                EventKind::Remove(_) => {
                    Self::handle_file_removed(path, processing_service, archive_cache).await?;
                }
                EventKind::Modify(modify_kind) => {
                    match modify_kind {
                        notify::event::ModifyKind::Name(name_mode) => {
                            match name_mode {
                                notify::event::RenameMode::From => {
                                    // 文件从监控目录移出（删除）
                                    if crate::services::ArchiveService::is_supported_format(path) {
                                        info!(
                                            "Archive file moved out (treating as delete): {}",
                                            path.display()
                                        );
                                        Self::handle_file_removed(
                                            path,
                                            processing_service,
                                            archive_cache,
                                        )
                                        .await?;
                                    }
                                }
                                notify::event::RenameMode::To => {
                                    // 文件移入监控目录（新增）
                                    if path.is_file()
                                        && crate::services::ArchiveService::is_supported_format(
                                            path,
                                        )
                                    {
                                        info!(
                                            "Archive file moved in (treating as new): {}",
                                            path.display()
                                        );
                                        Self::handle_file_created(path, processing_service).await?;
                                    }
                                }
                                _ => {
                                    debug!(
                                        "File rename operation: {:?} for {}",
                                        name_mode,
                                        path.display()
                                    );
                                }
                            }
                        }
                        _ => {
                            // 其他修改事件，通常漫画文件不会被修改，所以暂时忽略
                            debug!("File content modified (ignored): {}", path.display());
                        }
                    }
                }
                _ => {
                    debug!("Unhandled file event kind: {:?}", event.kind);
                }
            }
        }

        Ok(())
    }

    /// 处理文件创建事件（在文件写入完成后调用）
    async fn handle_file_created(
        path: &Path,
        processing_service: &ArchiveProcessingService,
    ) -> Result<()> {
        info!("Processing completed archive file: {}", path.display());

        match processing_service.process_new_archive(path).await {
            Ok(archive) => {
                info!(
                    "Successfully processed new archive: {} (ID: {})",
                    archive.title, archive.id
                );
            }
            Err(e) => {
                warn!("Failed to process new archive {}: {}", path.display(), e);
            }
        }

        Ok(())
    }

    /// 处理文件删除事件
    async fn handle_file_removed(
        path: &Path,
        processing_service: &ArchiveProcessingService,
        archive_cache: &ArchiveCacheService,
    ) -> Result<()> {
        info!("File removed: {}", path.display());

        // 查找数据库中对应的存档记录并删除
        if let Err(e) =
            Self::remove_archive_by_path(processing_service.get_db(), path, archive_cache).await
        {
            warn!(
                "Failed to remove archive from database for path {}: {}",
                path.display(),
                e
            );
        }

        Ok(())
    }

    async fn handle_file_rename_both(
        from_path: &Path,
        to_path: &Path,
        settings: &ScanSettings,
        processing_service: &ArchiveProcessingService,
        archive_cache: &ArchiveCacheService,
    ) -> Result<()> {
        let from_supported = crate::services::ArchiveService::is_supported_format(from_path);
        let to_supported = crate::services::ArchiveService::is_supported_format(to_path);

        let from_visible = !settings.ignore_hidden || !Self::is_hidden_file(from_path);
        let to_visible = !settings.ignore_hidden || !Self::is_hidden_file(to_path);

        match (from_supported && from_visible, to_supported && to_visible) {
            (true, true) => {
                if Self::update_archive_path(processing_service.get_db(), from_path, to_path)
                    .await?
                {
                    info!(
                        "Archive file renamed in-place: {} -> {}",
                        from_path.display(),
                        to_path.display()
                    );
                } else if to_path.is_file() {
                    // 兜底：旧路径未命中数据库时按新文件处理，避免漏收录
                    info!(
                        "Rename target not found by source path, processing as new archive: {}",
                        to_path.display()
                    );
                    Self::handle_file_created(to_path, processing_service).await?;
                }
            }
            (true, false) => {
                // 从可跟踪文件变成不可跟踪文件，等同删除
                Self::handle_file_removed(from_path, processing_service, archive_cache).await?;
            }
            (false, true) => {
                // 从不可跟踪文件变成可跟踪文件，等同新增
                if to_path.is_file() {
                    Self::handle_file_created(to_path, processing_service).await?;
                }
            }
            (false, false) => {
                trace!(
                    "Ignoring rename for unsupported/hidden paths: {} -> {}",
                    from_path.display(),
                    to_path.display()
                );
            }
        }

        Ok(())
    }

    async fn update_archive_path(
        db: &Pool<Sqlite>,
        from_path: &Path,
        to_path: &Path,
    ) -> Result<bool> {
        let from_str = from_path.to_string_lossy().to_string();
        let to_str = to_path.to_string_lossy().to_string();

        let result = sqlx::query!(
            "UPDATE archives SET path = ?, updated_at = CURRENT_TIMESTAMP WHERE path = ?",
            to_str,
            from_str
        )
        .execute(db)
        .await
        .context("更新重命名后的存档路径失败")?;

        Ok(result.rows_affected() > 0)
    }

    /// 从数据库中删除指定路径的存档记录
    async fn remove_archive_by_path(
        db: &Pool<Sqlite>,
        path: &Path,
        archive_cache: &ArchiveCacheService,
    ) -> Result<()> {
        let path_str = path.to_string_lossy().to_string();

        let rows = sqlx::query!("SELECT id FROM archives WHERE path = ?", path_str)
            .fetch_all(db)
            .await
            .context("查询待删除存档失败")?;

        let archive_ids: Vec<String> = rows.into_iter().filter_map(|row| row.id).collect();

        let result = sqlx::query!("DELETE FROM archives WHERE path = ?", path_str)
            .execute(db)
            .await
            .context("删除存档记录失败")?;

        if result.rows_affected() > 0 {
            info!("Removed archive record from database: {}", path_str);
            for archive_id in archive_ids {
                archive_cache.clear_archive_cache(&archive_id).await;
            }
        }

        Ok(())
    }

    /// 检查是否为隐藏文件
    fn is_hidden_file(path: &Path) -> bool {
        if let Some(file_name) = path.file_name() {
            if let Some(name_str) = file_name.to_str() {
                return name_str.starts_with('.');
            }
        }
        false
    }

    /// 获取当前监控状态
    pub async fn is_monitoring(&self) -> bool {
        self.watcher.read().await.is_some()
    }

    /// 获取当前设置
    pub async fn get_settings(&self) -> ScanSettings {
        self.settings.read().await.clone()
    }
}
