import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type { Archive, ReadingProgress } from "@/types/api";

export const useReaderStore = defineStore("reader", () => {
  const currentArchive = ref<Archive | null>(null);
  const currentPage = ref(0);
  const readingMode = ref<"single" | "double">("single");
  const imageDisplayMode = ref<"fit" | "fill" | "original">("fit");
  const zoomLevel = ref(1);
  const isFullscreen = ref(false);
  const serverProgress = ref<ReadingProgress | null>(null);

  const setArchive = (archive: Archive) => {
    currentArchive.value = archive;
    currentPage.value = 1; // 从第1页开始
  };

  const nextPage = () => {
    if (currentPage.value < (currentArchive.value?.pageCount ?? 0)) {
      currentPage.value++;
    }
  };

  const prevPage = () => {
    if (currentPage.value > 1) {
      currentPage.value--;
    }
  };

  const goToPage = (page: number) => {
    if (page >= 1 && page <= (currentArchive.value?.pageCount ?? 0)) {
      currentPage.value = page;
    }
  };

  const setReadingMode = (mode: "single" | "double") => {
    readingMode.value = mode;
  };

  const setImageDisplayMode = (mode: "fit" | "fill" | "original") => {
    imageDisplayMode.value = mode;
  };

  const setZoomLevel = (level: number) => {
    zoomLevel.value = Math.max(0.1, Math.min(3, level)); // 限制缩放范围
  };

  const toggleFullscreen = () => {
    isFullscreen.value = !isFullscreen.value;
  };

  const resetReader = () => {
    currentArchive.value = null;
    currentPage.value = 0;
    imageDisplayMode.value = "fit";
    zoomLevel.value = 1;
    isFullscreen.value = false;
    serverProgress.value = null;
  };

  // 设置服务器进度数据
  const setServerProgress = (progress: ReadingProgress | null) => {
    serverProgress.value = progress;
    if (progress && progress.currentPage > 0) {
      currentPage.value = progress.currentPage;
    }
  };

  // 检查是否需要同步到服务器
  const needsServerSync = computed(() => {
    if (!serverProgress.value) return true;
    return serverProgress.value.currentPage !== currentPage.value;
  });

  // 计算属性
  const progress = computed(() => {
    if (!currentArchive.value) return 0;
    return (currentPage.value / currentArchive.value.pageCount) * 100;
  });

  const serverProgressPercentage = computed(() => {
    return serverProgress.value?.progressPercentage ?? 0;
  });

  const canGoNext = computed(() => {
    return currentPage.value < (currentArchive.value?.pageCount ?? 0);
  });

  const canGoPrev = computed(() => {
    return currentPage.value > 1;
  });

  return {
    currentArchive,
    currentPage,
    readingMode,
    imageDisplayMode,
    zoomLevel,
    isFullscreen,
    serverProgress,
    progress,
    serverProgressPercentage,
    needsServerSync,
    canGoNext,
    canGoPrev,
    setArchive,
    setServerProgress,
    nextPage,
    prevPage,
    goToPage,
    setReadingMode,
    setImageDisplayMode,
    setZoomLevel,
    toggleFullscreen,
    resetReader,
  };
});
