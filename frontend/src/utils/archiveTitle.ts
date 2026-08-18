import type { Archive } from "@/types/api";

const hasTranslatedTitle = (archive?: Archive | null) =>
  Boolean(archive?.subtitle?.trim() && archive.subtitleLanguage);

export const archiveDisplayTitle = (
  archive?: Archive | null,
  displayTranslatedTitle = false,
) => {
  if (!archive) return "";
  return displayTranslatedTitle && hasTranslatedTitle(archive)
    ? archive.subtitle!.trim()
    : archive.title;
};

export const archiveDisplaySubtitle = (
  archive?: Archive | null,
  displayTranslatedTitle = false,
) => {
  if (!archive) return "";
  return displayTranslatedTitle && hasTranslatedTitle(archive)
    ? archive.title
    : archive.subtitle?.trim() ?? "";
};
