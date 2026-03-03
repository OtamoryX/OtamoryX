import { defineStore } from "pinia";
import { ref } from "vue";
import type { SystemSettings } from "@/types/api";

export const useSettingsStore = defineStore("settings", () => {
  const settings = ref<SystemSettings>({
    comicsPath: "",
    supportedFormats: ["cbz", "cbr", "cb7", "zip", "rar"],
    maxFileSize: 100 * 1024 * 1024, // 100MB
    imageCacheSize: 1024 * 1024 * 1024, // 1GB
    imageCachePath: "./data/cache",
    scanOnStartup: true,
    scanSettings: {
      enabled: true,
      recursive: true,
      ignoreHidden: true,
      realtimeMonitoring: false,
    },
  });

  const updateSettings = (newSettings: Partial<SystemSettings>) => {
    settings.value = { ...settings.value, ...newSettings };
  };

  const resetSettings = () => {
    settings.value = {
      comicsPath: "",
      supportedFormats: ["cbz", "cbr", "cb7", "zip", "rar"],
      maxFileSize: 100 * 1024 * 1024,
      imageCacheSize: 1024 * 1024 * 1024,
      imageCachePath: "./data/cache",
      scanOnStartup: true,
      scanSettings: {
        enabled: true,
        recursive: true,
        ignoreHidden: true,
        realtimeMonitoring: false,
      },
    };
  };

  return {
    settings,
    updateSettings,
    resetSettings,
  };
});
