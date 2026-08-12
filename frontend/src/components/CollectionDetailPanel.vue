<template>
  <BaseSidePanel
    :show="show"
    title="合集详情"
    width="wide"
    @close="$emit('close')"
  >
    <div v-if="isLoading" class="p-4 text-sm text-[var(--text-tertiary)]">
      加载中...
    </div>
    <div v-else-if="!detail" class="p-4 text-sm text-[var(--text-tertiary)]">
      合集不存在或没有访问权限。
    </div>
    <div v-else class="p-0 md:p-4">
      <div class="flex items-start gap-3 pb-4 border-b border-[var(--border)]">
        <div
          class="w-16 h-24 rounded-sm overflow-hidden bg-[var(--bg-tertiary)] flex-shrink-0"
        >
          <img
            v-if="coverUrl"
            :src="coverUrl"
            :alt="detail.collection.displayTitle"
            class="w-full h-full object-cover"
          />
        </div>
        <div class="min-w-0 flex-1">
          <h2
            class="text-base font-semibold text-[var(--text-primary)] break-words"
          >
            {{ detail.collection.displayTitle }}
          </h2>
          <p
            v-if="detail.collection.subtitle"
            class="mt-1 text-sm text-[var(--text-tertiary)] break-words"
          >
            {{ detail.collection.subtitle }}
          </p>
          <p class="mt-1 text-xs text-[var(--text-tertiary)]">
            {{ detail.collection.contentCount }} 个内容 ·
            {{ detail.collection.memberCount }} 个文件
          </p>
          <button
            class="mt-3 inline-flex h-11 items-center gap-1.5 rounded px-3 text-sm bg-[var(--accent)] text-white hover:opacity-90"
            @click="openReader(nextMember?.archive.id)"
          >
            <BookOpenIcon class="w-3.5 h-3.5" />继续阅读
          </button>
        </div>
      </div>

      <section v-if="collectionReviews.length" class="pt-4">
        <div class="flex items-center justify-between gap-3">
          <h3 class="text-sm font-medium text-[var(--text-primary)]">
            待确认成员
          </h3>
          <span class="text-[10px] text-amber-400"
            >{{ collectionReviews.length }} 项</span
          >
        </div>
        <article
          v-for="review in collectionReviews"
          :key="review.id"
          class="mt-2 border-t border-[var(--border)] py-3"
        >
          <div class="flex items-start gap-2.5">
            <div
              class="h-14 w-10 shrink-0 overflow-hidden rounded-sm bg-[var(--bg-tertiary)]"
            >
              <img
                v-if="memberCovers[review.archive.id]"
                :src="memberCovers[review.archive.id]"
                :alt="review.archive.title"
                class="h-full w-full object-cover"
              />
            </div>
            <div class="min-w-0 flex-1">
              <p
                class="break-words text-xs font-medium text-[var(--text-primary)]"
              >
                {{ review.archive.title }}
              </p>
              <p class="mt-0.5 text-[10px] text-[var(--text-tertiary)]">
                {{ review.archive.pageCount }} 页 ·
                {{ formatSize(review.archive.fileSize) }}
              </p>
              <p class="mt-1 text-[10px] text-amber-400">{{ review.reason }}</p>
              <button
                class="mt-2 text-xs text-[var(--accent)] hover:underline"
                @click="openReader(review.archive.id)"
              >
                查看内容
              </button>
              <div class="mt-2 grid grid-cols-2 gap-2">
                <button
                  class="h-11 rounded border border-[var(--border)] text-xs font-medium text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] disabled:opacity-50"
                  :disabled="busyReviewId === review.id"
                  @click="applyReview(review, 'reject')"
                >
                  不加入
                </button>
                <button
                  class="h-11 rounded bg-[var(--accent)] text-xs font-medium text-white hover:opacity-90 disabled:opacity-50"
                  :disabled="busyReviewId === review.id"
                  @click="applyReview(review, 'approve')"
                >
                  确认加入
                </button>
              </div>
            </div>
          </div>
        </article>
      </section>

      <div class="pt-4">
        <div class="flex items-center justify-between mb-2">
          <h3 class="text-sm font-medium text-[var(--text-primary)]">
            已确认成员
          </h3>
          <label
            v-if="hasMemberFilter"
            class="flex items-center gap-1.5 text-[10px] text-[var(--text-secondary)]"
            ><input
              v-model="onlyMatching"
              type="checkbox"
              class="accent-[var(--accent)]"
            />仅看命中</label
          >
          <span v-else class="text-[10px] text-[var(--text-tertiary)]"
            >按识别顺序</span
          >
        </div>
        <div
          v-for="member in visibleMembers"
          :key="member.archive.id"
          role="button"
          tabindex="0"
          class="relative w-full py-2.5 border-t border-[var(--border)] flex items-center gap-2.5 text-left cursor-pointer"
          :class="member.matchesFilter ? 'bg-[var(--accent)]/5' : ''"
          @click="openReader(member.archive.id)"
          @keydown.enter="openReader(member.archive.id)"
          @keydown.space.prevent="openReader(member.archive.id)"
        >
          <div
            class="w-7 h-10 rounded-sm bg-[var(--bg-tertiary)] flex-shrink-0 overflow-hidden"
          >
            <img
              v-if="memberCovers[member.archive.id]"
              :src="memberCovers[member.archive.id]"
              :alt="member.archive.title"
              class="w-full h-full object-cover"
            />
          </div>
          <div class="min-w-0 flex-1">
            <div
              class="text-xs font-medium text-[var(--text-primary)] truncate"
            >
              {{ memberLabel(member) }}
            </div>
            <div
              class="mt-0.5 text-[10px] text-[var(--text-tertiary)] truncate"
            >
              {{ member.archive.title }}
            </div>
            <div
              v-if="member.matchesFilter"
              class="mt-0.5 text-[10px] text-[var(--accent)]"
            >
              命中当前筛选
            </div>
            <div class="mt-1 flex items-center gap-2">
              <div
                class="h-1 flex-1 overflow-hidden rounded-full bg-[var(--bg-tertiary)]"
              >
                <div
                  class="h-full bg-[var(--accent)]"
                  :style="{
                    width: `${memberProgress[member.archive.id]?.progressPercentage ?? 0}%`,
                  }"
                />
              </div>
              <span class="shrink-0 text-[10px] text-[var(--text-tertiary)]">{{
                progressLabel(member.archive.id)
              }}</span>
            </div>
          </div>
          <div class="flex items-center gap-0.5">
            <span class="hidden md:inline-flex p-1 text-[var(--text-tertiary)]"
              ><BookOpenIcon class="w-4 h-4"
            /></span>
            <button
              class="hidden md:flex h-11 w-11 items-center justify-center text-[var(--text-tertiary)] hover:text-red-400"
              title="从合集中移出"
              @click.stop="$emit('removeMember', member.archive.id)"
            >
              <XMarkIcon class="w-4 h-4" />
            </button>
            <button
              class="flex md:hidden h-11 w-11 items-center justify-center text-[var(--text-tertiary)] hover:text-[var(--text-primary)]"
              title="更多操作"
              @click.stop="
                memberActionId =
                  memberActionId === member.archive.id
                    ? null
                    : member.archive.id
              "
            >
              <EllipsisVerticalIcon class="h-5 w-5" />
            </button>
          </div>
          <div
            v-if="memberActionId === member.archive.id"
            class="absolute bottom-1 right-2 z-10 rounded border border-[var(--border)] bg-[var(--bg-card)] p-1 shadow-lg"
            @click.stop
          >
            <button
              class="flex h-10 items-center px-3 text-xs text-red-400 hover:bg-red-500/10"
              @click="
                $emit('removeMember', member.archive.id);
                memberActionId = null;
              "
            >
              从合集中移出
            </button>
          </div>
        </div>
      </div>
    </div>
  </BaseSidePanel>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import {
  BookOpenIcon,
  EllipsisVerticalIcon,
  XMarkIcon,
} from "@heroicons/vue/24/outline";
import BaseSidePanel from "@/components/base/BaseSidePanel.vue";
import type {
  CollectionDetail,
  CollectionMember,
  ReadingProgress,
} from "@/types/api";
import {
  applyCollectionReview,
  getArchiveThumbnail,
  getBatchProgress,
} from "@/utils/api";

const props = defineProps<{
  show: boolean;
  detail: CollectionDetail | null;
  isLoading?: boolean;
  hasMemberFilter?: boolean;
}>();
const emit = defineEmits<{
  close: [];
  openReader: [archiveId: string, collectionId?: string];
  removeMember: [archiveId: string];
  reviewsChanged: [];
}>();
const coverUrl = ref<string | null>(null);
const memberCovers = ref<Record<string, string>>({});
const memberProgress = ref<Record<string, ReadingProgress>>({});
const busyReviewId = ref<string | null>(null);
const onlyMatching = ref(false);
const memberActionId = ref<string | null>(null);
const nextMember = computed(
  () =>
    props.detail?.members.find((member) => member.confidence >= 0.75) ||
    props.detail?.members[0],
);
const collectionReviews = computed(() =>
  (props.detail?.members ?? []).flatMap((member) =>
    member.review
      ? [
          {
            id: member.review.id,
            archive: member.archive,
            reason: member.review.reason,
          },
        ]
      : [],
  ),
);
const confirmedMembers = computed(
  () => props.detail?.members.filter((member) => !member.review) ?? [],
);
const visibleMembers = computed(() =>
  onlyMatching.value
    ? confirmedMembers.value.filter((member) => member.matchesFilter)
    : confirmedMembers.value,
);

const loadCovers = async (detail: CollectionDetail | null) => {
  coverUrl.value = null;
  memberCovers.value = {};
  if (!detail) return;
  const coverId =
    detail.collection.coverArchiveId || detail.members[0]?.archive.id;
  if (coverId)
    coverUrl.value = await getArchiveThumbnail(coverId).catch(() => null);
  const archiveIds = new Set(
    detail.members.slice(0, 12).map((member) => member.archive.id),
  );
  collectionReviews.value.forEach((review) =>
    archiveIds.add(review.archive.id),
  );
  const entries = await Promise.all(
    [...archiveIds].map(
      async (id) =>
        [id, await getArchiveThumbnail(id).catch(() => "")] as const,
    ),
  );
  memberCovers.value = Object.fromEntries(entries.filter(([, url]) => url));
};

const memberLabel = (member: CollectionMember) => {
  if (member.volumeNumber)
    return `第 ${member.volumeNumber} 卷${member.chapterNumber ? ` / 第 ${member.chapterNumber} 话` : ""}`;
  if (member.chapterNumber) return `第 ${member.chapterNumber} 话`;
  if (member.issueNumber) return `期号 ${member.issueNumber}`;
  return member.rawNumber ? `编号 ${member.rawNumber}` : "未编号成员";
};
const loadMemberProgress = async (detail: CollectionDetail | null) => {
  memberProgress.value = {};
  if (!detail) return;
  const progress = await getBatchProgress(
    detail.members.map((member) => member.archive.id),
  );
  if (props.detail?.collection.id === detail.collection.id) {
    memberProgress.value = Object.fromEntries(
      progress.map((item) => [item.archiveId, item]),
    );
  }
};
const progressLabel = (archiveId: string) => {
  const progress = memberProgress.value[archiveId];
  return progress ? `${Math.round(progress.progressPercentage)}%` : "未读";
};
const openReader = (archiveId?: string) => {
  if (archiveId) emit("openReader", archiveId, props.detail?.collection.id);
};
const applyReview = async (
  review: { id: string },
  action: "approve" | "reject",
) => {
  busyReviewId.value = review.id;
  try {
    await applyCollectionReview(review.id, action);
    emit("reviewsChanged");
  } finally {
    busyReviewId.value = null;
  }
};
const formatSize = (bytes: number) =>
  bytes >= 1024 * 1024
    ? `${(bytes / 1024 / 1024).toFixed(1)} MB`
    : `${Math.ceil(bytes / 1024)} KB`;
watch(
  [() => props.detail, collectionReviews],
  ([detail]) => {
    void loadCovers(detail);
  },
  { immediate: true },
);
watch(
  () => props.detail,
  (detail) => {
    onlyMatching.value = false;
    memberActionId.value = null;
    void loadMemberProgress(detail);
  },
  { immediate: true },
);
onMounted(() => {
  if (props.detail) void loadCovers(props.detail);
});
</script>
