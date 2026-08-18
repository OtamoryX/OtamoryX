import type { TrashEntry } from "@/types/api";

const LEGACY_REASON_LABELS: Record<string, string> = {
  "user initiated archive deletion": "用户主动删除漫画",
  "user initiated batch archive deletion": "用户主动批量删除漫画",
  "user initiated tag batch deletion": "通过标签批量删除漫画",
  "user initiated category batch deletion": "通过分类批量删除漫画",
  "user deleted collection with members": "删除合集及成员漫画",
  version_cleanup: "按版本清理策略移入回收站",
};

/** Keep persisted backend reasons readable until the full i18n catalog lands. */
export const localizeTrashReason = (
  entry: Pick<TrashEntry, "reason" | "ruleId" | "operationType">,
  source: string,
): string => {
  if (source === "版本清理" || entry.operationType === "version_cleanup") {
    return LEGACY_REASON_LABELS.version_cleanup;
  }

  const reason = entry.reason?.trim();
  if (reason && LEGACY_REASON_LABELS[reason]) {
    return LEGACY_REASON_LABELS[reason];
  }

  if (source === "自动删除" || entry.ruleId) {
    const ruleMatch = reason?.match(/^preference rule (.+) matched$/);
    if (ruleMatch?.[1]) {
      return `命中偏好规则“${ruleMatch[1]}”`;
    }
    return "匹配自动删除规则";
  }

  return "用户主动删除漫画";
};
