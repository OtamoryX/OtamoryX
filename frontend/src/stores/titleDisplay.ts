import { defineStore } from "pinia";
import { ref } from "vue";
import { getTitleDisplayPreference } from "@/utils/api";

export const useTitleDisplayStore = defineStore("titleDisplay", () => {
  const displayTranslatedTitle = ref(false);
  const loaded = ref(false);
  let loading: Promise<void> | null = null;

  const load = async (force = false) => {
    if (loading) return loading;
    if (loaded.value && !force) return;
    loading = getTitleDisplayPreference()
      .then((preference) => {
        displayTranslatedTitle.value = preference.displayTranslatedTitle;
        loaded.value = true;
      })
      .catch(() => {
        // Keep the safe default: original title first.
        displayTranslatedTitle.value = false;
      })
      .finally(() => {
        loading = null;
      });
    return loading;
  };

  const setEnabled = (enabled: boolean) => {
    displayTranslatedTitle.value = enabled;
    loaded.value = true;
  };

  return { displayTranslatedTitle, loaded, load, setEnabled };
});
