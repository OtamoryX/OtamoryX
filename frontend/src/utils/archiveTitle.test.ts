import { describe, expect, it } from "vitest";
import type { Archive } from "@/types/api";
import { archiveDisplaySubtitle, archiveDisplayTitle } from "./archiveTitle";

const archive = (subtitle?: string, subtitleLanguage?: string): Archive => ({
  id: "archive-1",
  title: "Original title",
  subtitle,
  subtitleLanguage,
  path: "/library/archive.cbz",
  pageCount: 20,
  fileSize: 1024,
  hash: "hash",
  createdAt: "2026-08-19T00:00:00Z",
  updatedAt: "2026-08-19T00:00:00Z",
  tags: [],
});

describe("archive title display", () => {
  it("keeps the original title first when the preference is disabled", () => {
    const item = archive("Translated title", "zh-CN");

    expect(archiveDisplayTitle(item, false)).toBe("Original title");
    expect(archiveDisplaySubtitle(item, false)).toBe("Translated title");
  });

  it("promotes a translated title and preserves the original as the subtitle", () => {
    const item = archive("Translated title", "zh-CN");

    expect(archiveDisplayTitle(item, true)).toBe("Translated title");
    expect(archiveDisplaySubtitle(item, true)).toBe("Original title");
  });

  it("falls back to the original title when no translated title is available", () => {
    const item = archive();

    expect(archiveDisplayTitle(item, true)).toBe("Original title");
    expect(archiveDisplaySubtitle(item, true)).toBe("");
  });
});
