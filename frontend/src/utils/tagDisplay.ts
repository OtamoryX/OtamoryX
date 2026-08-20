import type { Tag } from "@/types/api";

const namespaceLabels: Record<string, string> = {
  general: "通用",
  sensitive: "敏感",
  artist: "作者",
  group: "社团",
  parody: "原作",
  character: "角色",
  category: "分类",
  language: "语言",
  male: "男性",
  female: "女性",
  mixed: "混合",
  other: "其他",
  cosplayer: "Coser",
  reclass: "重分类",
  series: "系列",
  publisher: "出版社",
  scanlator: "汉化组",
  genre: "题材",
};

export const tagNamespaceLabel = (namespace: string): string =>
  namespaceLabels[namespace.trim().toLowerCase()] || namespace;

export const tagDisplayName = (tag: Pick<Tag, "name" | "localizedName">): string =>
  tag.localizedName?.trim() || tag.name;

export const tagDisplayText = (tag: Tag, showNamespace = true): string =>
  showNamespace && tag.namespace
    ? `${tagNamespaceLabel(tag.namespace)}:${tagDisplayName(tag)}`
    : tagDisplayName(tag);

/** Match both the canonical English identity and the localized UI label. */
export const tagSearchText = (tag: Tag): string =>
  `${tag.namespace}:${tag.name} ${tagNamespaceLabel(tag.namespace)}:${tagDisplayName(tag)}`.toLowerCase();
