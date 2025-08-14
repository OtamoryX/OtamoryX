import { defineStore } from "pinia";
import { ref } from "vue";
import type { SystemSettings } from "@/types/api";

export const useSettingsStore = defineStore("settings", () => {
  const settings = ref<SystemSettings>({
    comicsPath: "",
    supportedFormats: ["cbz", "cbr", "cb7", "zip", "rar"],
    maxFileSize: 100 * 1024 * 1024, // 100MB
    imageCacheSize: 1024 * 1024 * 1024, // 1GB
    scanOnStartup: true,
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
      scanOnStartup: true,
    };
  };

  return {
    settings,
    updateSettings,
    resetSettings,
  };
});
