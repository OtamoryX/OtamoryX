import type { ScanSettings } from "@/types/api";

export type CacheStrategy = "conservative" | "balanced" | "aggressive" | "custom";

export interface CacheSettingsForm {
  cachePath: string;
  maxSize: number;
  quality: number;
  format: "JPEG" | "PNG" | "WebP";
  strategy: CacheStrategy;
  customConfig: {
    maxMemoryMb: number;
    maxCachedArchives: number;
    cacheTtlHours: number;
    preloadPrevPages: number;
    preloadNextPages: number;
  };
}

export interface CacheStats {
  cached_archives: number;
  memory_usage_mb: number;
  hit_rate: number;
  [key: string]: number;
}

export interface CacheConfigInfo {
  max_memory_mb: number;
  max_cached_archives: number;
  cache_ttl_hours: number;
  preload_next_pages: number;
  preload_prev_pages: number;
  cleanup_threshold_percent: number;
  enable_background_preload: boolean;
  max_concurrent_extractions: number;
}

export interface CacheStatusResponse {
  current_strategy: string;
  stats: CacheStats;
  config: CacheConfigInfo;
}

export interface CacheCustomConfig {
  max_memory_mb: number;
  max_cached_archives: number;
  cache_ttl_hours: number;
  preload_prev_pages: number;
  preload_next_pages: number;
}

export interface ConfigureCacheRequest {
  strategy?: Exclude<CacheStrategy, "custom">;
  custom_config?: CacheCustomConfig;
}

export interface ConfigureCacheResponse {
  message: string;
  note?: string;
  requested_strategy: "Conservative" | "Balanced" | "Aggressive" | "Custom";
}

export interface ScanSettingsResponse {
  scanSettings: ScanSettings;
  monitoring_status: boolean;
}

export interface TriggerScanResponse {
  message: string;
  new_archives_count: number;
}

export interface BatchDeleteForm {
  archiveIds: string;
  categoryId: string;
  tagId: string;
}

export interface BatchOperationRecord {
  operation: string;
  timestamp: string;
  success: boolean;
  result: string;
}
