<template>
  <BasePageView theme="library">
    <!-- Library特有的侧边栏+主内容布局 -->
    <div class="library-view flex h-full w-full min-h-screen relative">
      <!-- 分类侧边栏 -->
      <CategorySidebar
        :selected-category-id="selectedCategoryId"
        :total-archives="allArchivesCount"
        :collapsed="sidebarCollapsed"
        @select-category="handleSelectCategory"
        @create-category="showCreateCategoryModal = true"
        @edit-category="handleEditCategory"
        @toggle-collapse="handleToggleCollapse"
      />

      <!-- 主内容区域 -->
      <div 
        :class="[
          'flex-1 overflow-y-auto transition-all duration-300',
          // 移动端在侧边栏展开时添加一些margin，防止被遮挡
          'p-6',
        ]"
      >
        <div class="mb-6">
          <GlassCard size="sm"
radius="lg" class="mb-6">
            <div class="flex items-center justify-between">
              <h1 class="text-2xl font-bold text-white">
                <svg
                  class="w-8 h-8 inline-block mr-3 text-blue-400"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 011-1h6a1 1 0 011 1v2M7 7v2m10-2v2"
                  />
                </svg>
                {{ currentCategoryName }}
              </h1>
              <div
                class="text-sm text-white/70 bg-white/10 px-3 py-1 rounded-full"
              >
                共 {{ totalArchives }} 部漫画
              </div>
            </div>
          </GlassCard>

          <!-- 搜索栏 -->
          <GlassCard size="sm"
radius="lg" class="max-w-2xl mb-6">
            <div class="space-y-4">
              <div class="flex gap-3">
                <div class="flex-1 relative">
                  <GlassInput
                    v-model="searchQuery"
                    placeholder="搜索漫画..."
                    @keyup.enter="handleSearch"
                  >
                    <template #suffix>
                      <GlassButton
                        :disabled="isSearching"
                        size="sm"
                        variant="ghost"
                        class="p-2!"
                        @click="handleSearch"
                      >
                        <svg
                          v-if="!isSearching"
                          class="w-5 h-5"
                          fill="none"
                          stroke="currentColor"
                          viewBox="0 0 24 24"
                        >
                          <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                          />
                        </svg>
                        <svg
                          v-else
                          class="w-5 h-5 animate-spin"
                          fill="none"
                          viewBox="0 0 24 24"
                        >
                          <circle
                            class="opacity-25"
                            cx="12"
                            cy="12"
                            r="10"
                            stroke="currentColor"
                            stroke-width="4"
                          />
                          <path
                            class="opacity-75"
                            fill="currentColor"
                            d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                          />
                        </svg>
                      </GlassButton>
                    </template>
                  </GlassInput>
                </div>
                <GlassButton
                  variant="secondary"
                  class="px-4! py-2!"
                  @click="showAdvancedSearch = !showAdvancedSearch"
                >
                  <svg
                    class="w-5 h-5"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 100 4m0-4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 100 4m0-4v2m0-6V4"
                    />
                  </svg>
                </GlassButton>
              </div>

              <!-- 高级搜索面板 -->
              <div v-if="showAdvancedSearch"
class="space-y-4">
                <GlassCard size="sm"
radius="md" class="bg-black/20">
                  <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <!-- 添加时间筛选 -->
                    <div>
                      <label class="block text-sm font-medium text-white mb-2"
                        >添加时间</label
                      >
                      <div class="space-y-2">
                        <div>
                          <label class="block text-xs text-white/70 mb-1"
                            >从</label
                          >
                          <input
                            v-model="advancedSearch.createdAfter"
                            type="date"
                            class="w-full px-3 py-2 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:ring-2 focus:ring-blue-400 focus:bg-white/15"
                          >
                        </div>
                        <div>
                          <label class="block text-xs text-white/70 mb-1"
                            >到</label
                          >
                          <input
                            v-model="advancedSearch.createdBefore"
                            type="date"
                            class="w-full px-3 py-2 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:ring-2 focus:ring-blue-400 focus:bg-white/15"
                          >
                        </div>
                      </div>
                    </div>

                    <!-- 最后阅读时间筛选 -->
                    <div>
                      <label class="block text-sm font-medium text-white mb-2"
                        >最后阅读时间</label
                      >
                      <div class="space-y-2">
                        <div>
                          <label class="block text-xs text-white/70 mb-1"
                            >从</label
                          >
                          <input
                            v-model="advancedSearch.lastReadAfter"
                            type="date"
                            class="w-full px-3 py-2 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:ring-2 focus:ring-blue-400 focus:bg-white/15"
                          >
                        </div>
                        <div>
                          <label class="block text-xs text-white/70 mb-1"
                            >到</label
                          >
                          <input
                            v-model="advancedSearch.lastReadBefore"
                            type="date"
                            class="w-full px-3 py-2 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:ring-2 focus:ring-blue-400 focus:bg-white/15"
                          >
                        </div>
                      </div>
                    </div>

                    <!-- 页数筛选 -->
                    <div>
                      <label class="block text-sm font-medium text-white mb-2"
                        >页数范围</label
                      >
                      <div class="grid grid-cols-2 gap-2">
                        <div>
                          <label class="block text-xs text-white/70 mb-1"
                            >最少</label
                          >
                          <input
                            v-model.number="advancedSearch.minPages"
                            type="number"
                            min="1"
                            class="w-full px-3 py-2 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:ring-2 focus:ring-blue-400 focus:bg-white/15"
                            placeholder="0"
                          />
                        </div>
                        <div>
                          <label class="block text-xs text-white/70 mb-1"
                            >最多</label
                          >
                          <input
                            v-model.number="advancedSearch.maxPages"
                            type="number"
                            min="1"
                            class="w-full px-3 py-2 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:ring-2 focus:ring-blue-400 focus:bg-white/15"
                            placeholder="999"
                          />
                        </div>
                      </div>
                    </div>

                    <!-- 排序 -->
                    <div>
                      <label class="block text-sm font-medium text-white mb-2"
                        >排序方式</label
                      >
                      <div class="space-y-2">
                        <select
                          v-model="advancedSearch.sortBy"
                          class="w-full px-3 py-2 bg-white/10 border border-white/20 rounded-lg text-white focus:outline-none focus:ring-2 focus:ring-blue-400 focus:bg-white/15"
                        >
                          <option
value="" class="text-gray-900">默认</option>
                          <option value="title"
class="text-gray-900">
                            标题
                          </option>
                          <option value="createdAt"
class="text-gray-900">
                            添加时间
                          </option>
                          <option value="lastReadAt"
class="text-gray-900">
                            最后阅读时间
                          </option>
                          <option value="pageCount"
class="text-gray-900">
                            页数
                          </option>
                          <option value="fileSize"
class="text-gray-900">
                            文件大小
                          </option>
                        </select>
                        <select
                          v-model="advancedSearch.sortOrder"
                          class="w-full px-3 py-2 bg-white/10 border border-white/20 rounded-lg text-white focus:outline-none focus:ring-2 focus:ring-blue-400 focus:bg-white/15"
                        >
                          <option value="desc"
class="text-gray-900">
                            降序
                          </option>
                          <option value="asc"
class="text-gray-900">
                            升序
                          </option>
                        </select>
                      </div>
                    </div>
                  </div>

                  <!-- 操作按钮 -->
                  <div
                    class="flex justify-end space-x-3 pt-4 border-t border-white/20"
                  >
                    <GlassButton variant="ghost"
@click="clearAdvancedSearch">
                      清空
                    </GlassButton>
                    <GlassButton
                      variant="primary"
                      @click="handleAdvancedSearch"
                    >
                      搜索
                    </GlassButton>
                  </div>
                </GlassCard>
              </div>
            </div>
          </GlassCard>
        </div>

        <!-- 错误信息 -->
        <GlassCard
          v-if="error"
          size="sm"
          radius="lg"
          class="mb-6 bg-red-500/20 border-red-400/30"
        >
          <div class="flex items-center text-red-200">
            <svg
              class="w-5 h-5 mr-3"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
            加载失败: {{ error.message }}
          </div>
        </GlassCard>

        <!-- 漫画网格 -->
        <GlassCard v-if="isLoading"
size="md" radius="lg" class="text-center">
          <div class="flex items-center justify-center">
            <div
              class="w-6 h-6 border-2 border-white/30 border-t-white rounded-full animate-spin mr-3"
            />
            <div class="text-white/80">加载中...</div>
          </div>
        </GlassCard>

        <GlassCard
          v-else-if="archives.length === 0"
          size="md"
          radius="lg"
          class="text-center"
        >
          <div class="text-white/70">
            <svg
              class="w-16 h-16 mx-auto mb-4 text-white/40"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="1"
                d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 011-1h6a1 1 0 011 1v2M7 7v2m10-2v2"
              />
            </svg>
            {{ selectedCategoryId ? "该分类下没有漫画" : "没有找到漫画" }}
          </div>
        </GlassCard>

        <div
          v-else
          class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6 gap-4"
        >
          <ArchiveCard
            v-for="archive in archives"
            :key="archive.id"
            :archive="archive"
            :progress-percentage="
              progressData.get(archive.id)?.progressPercentage
            "
            @click="openReader(archive.id)"
            @contextmenu="handleArchiveContextMenu"
          />
        </div>

        <!-- 分页控件 -->
        <GlassCard v-if="totalPages > 1"
size="sm" radius="lg" class="mt-8">
          <div class="flex items-center justify-center space-x-2">
            <GlassButton
              :disabled="currentPage === 1"
              variant="ghost"
              size="sm"
              @click="goToPage(currentPage - 1)"
            >
              上一页
            </GlassButton>

            <!-- 页码显示 -->
            <div class="flex items-center space-x-1">
              <!-- 第一页 -->
              <GlassButton
                v-if="showFirstPage"
                :variant="currentPage === 1 ? 'primary' : 'ghost'"
                size="sm"
                @click="goToPage(1)"
              >
                1
              </GlassButton>

              <!-- 左侧省略号 -->
              <span
v-if="showLeftEllipsis" class="px-2 text-white/50"
              >...</span>

              <!-- 中间页码 -->
              <GlassButton
                v-for="page in visiblePages"
                :key="page"
                :variant="currentPage === page ? 'primary' : 'ghost'"
                size="sm"
                @click="goToPage(page)"
              >
                {{ page }}
              </GlassButton>

              <!-- 右侧省略号 -->
              <span
v-if="showRightEllipsis" class="px-2 text-white/50"
              >...</span>

              <!-- 最后一页 -->
              <GlassButton
                v-if="showLastPage"
                :variant="currentPage === totalPages ? 'primary' : 'ghost'"
                size="sm"
                @click="goToPage(totalPages)"
              >
                {{ totalPages }}
              </GlassButton>
            </div>

            <GlassButton
              :disabled="currentPage === totalPages"
              variant="ghost"
              size="sm"
              @click="goToPage(currentPage + 1)"
            >
              下一页
            </GlassButton>
          </div>
        </GlassCard>
      </div>

      <!-- 分类模态框 -->
      <CategoryModal
        v-if="showCreateCategoryModal"
        @close="showCreateCategoryModal = false"
        @created="handleCategoryCreated"
      />

      <CategoryModal
        v-if="showEditCategoryModal && selectedCategory"
        :category="selectedCategory"
        @close="showEditCategoryModal = false"
        @updated="handleCategoryUpdated"
      />

      <!-- 右键菜单 -->
      <ArchiveContextMenu
        :show="showContextMenu"
        :archive="contextMenuArchive"
        :position="contextMenuPosition"
        @close="closeContextMenu"
        @add-tag="handleAddTagFromContext"
        @add-to-category="handleAddToCategoryFromContext"
        @remove-from-category="handleRemoveFromCategoryFromContext"
        @delete-archive="handleDeleteArchiveFromContext"
      />

      <!-- 标签添加模态框 -->
      <TagModal
        v-if="showTagModal"
        :archive="tagModalArchive!"
        @close="closeTagModal"
        @submit="handleTagModalSubmit"
      />
    </div>
  </BasePageView>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import { useRouter, useRoute } from "vue-router";
import { useQuery, useQueryClient } from "@tanstack/vue-query";
import BasePageView from "@/components/layout/BasePageView.vue";
import ArchiveCard from "@/components/ArchiveCard.vue";
import CategorySidebar from "@/components/CategorySidebar.vue";
import CategoryModal from "@/components/CategoryModal.vue";
import ArchiveContextMenu from "@/components/ArchiveContextMenu.vue";
import TagModal from "@/components/common/TagModal.vue";
import GlassCard from "@/components/base/GlassCard.vue";
import GlassButton from "@/components/base/GlassButton.vue";
import GlassInput from "@/components/base/GlassInput.vue";
import {
  searchArchives,
  getCategoryArchives,
  getBatchProgress,
  createTag,
  addTagToArchive,
  addArchivesToCategory,
  removeArchivesFromCategory,
  deleteArchive,
} from "@/utils/api";
import type {
  Archive,
  SearchParams,
  Category,
  DynamicCategory,
  ReadingProgress,
} from "@/types/api";

const router = useRouter();
const route = useRoute();
const queryClient = useQueryClient();
const searchQuery = ref("");
const isSearching = ref(false);
const selectedCategoryId = ref<string | null>(null);
const showCreateCategoryModal = ref(false);
const showEditCategoryModal = ref(false);
const selectedCategory = ref<Category | DynamicCategory | null>(null);
const showAdvancedSearch = ref(false);
const currentPage = ref(1);
const pageSize = ref(20);

// 侧边栏折叠状态管理
const sidebarCollapsed = ref(false);
const COLLAPSE_BREAKPOINT = 768; // 定义断点宽度 (768px = md)，更适合移动端

// 右键菜单状态管理
const showContextMenu = ref(false);
const contextMenuArchive = ref<Archive | null>(null);
const contextMenuPosition = ref({ x: 0, y: 0 });

// 标签模态框状态管理
const showTagModal = ref(false);
const tagModalArchive = ref<Archive | null>(null);

// 进度数据管理
const progressData = ref<Map<string, ReadingProgress>>(new Map());

// 高级搜索参数
const advancedSearch = ref({
  createdAfter: "",
  createdBefore: "",
  lastReadAfter: "",
  lastReadBefore: "",
  minPages: undefined as number | undefined,
  maxPages: undefined as number | undefined,
  sortBy: "createdAt",
  sortOrder: "asc",
});

// 当前分类名称
const currentCategoryName = computed(() => {
  if (!selectedCategoryId.value) return "全部漫画";
  return selectedCategory.value?.name || "分类漫画";
});

// 搜索参数
const searchParams = computed<SearchParams>(() => {
  const params: SearchParams = {
    query: searchQuery.value || undefined,
    pageNumb: currentPage.value,
    pageSize: pageSize.value,
  };

  // 如果有高级搜索参数，则添加到搜索参数中
  if (advancedSearch.value.createdAfter) {
    params.createdAfter = advancedSearch.value.createdAfter;
  }
  if (advancedSearch.value.createdBefore) {
    params.createdBefore = advancedSearch.value.createdBefore;
  }
  if (advancedSearch.value.lastReadAfter) {
    params.lastReadAfter = advancedSearch.value.lastReadAfter;
  }
  if (advancedSearch.value.lastReadBefore) {
    params.lastReadBefore = advancedSearch.value.lastReadBefore;
  }
  if (advancedSearch.value.minPages) {
    params.minPages = advancedSearch.value.minPages;
  }
  if (advancedSearch.value.maxPages) {
    params.maxPages = advancedSearch.value.maxPages;
  }
  if (advancedSearch.value.sortBy) {
    params.sortBy = advancedSearch.value.sortBy;
    params.sortOrder = advancedSearch.value.sortOrder;
  }

  return params;
});

// 创建一个计算属性作为稳定的 queryKey
const queryKey = computed(() => [
  "archives",
  selectedCategoryId.value,
  currentPage.value,
  pageSize.value,
  searchQuery.value,
  advancedSearch.value.createdAfter,
  advancedSearch.value.createdBefore,
  advancedSearch.value.lastReadAfter,
  advancedSearch.value.lastReadBefore,
  advancedSearch.value.minPages,
  advancedSearch.value.maxPages,
  advancedSearch.value.sortBy,
  advancedSearch.value.sortOrder,
]);

// 根据选择的分类和搜索查询决定使用哪个 API
const { data, isLoading, refetch, error } = useQuery({
  queryKey,
  queryFn: async () => {
    try {
      // 如果有任何搜索条件（包括搜索词、高级搜索参数），都使用 search API
      const hasSearchCriteria =
        searchQuery.value.trim() ||
        advancedSearch.value.createdAfter ||
        advancedSearch.value.createdBefore ||
        advancedSearch.value.lastReadAfter ||
        advancedSearch.value.lastReadBefore ||
        advancedSearch.value.minPages ||
        advancedSearch.value.maxPages ||
        advancedSearch.value.sortBy;

      if (hasSearchCriteria) {
        // 使用搜索API，如果有分类则添加分类标签过滤
        const params = { ...searchParams.value };
        if (selectedCategoryId.value) {
          // 当搜索时，切换到获取分类下的档案而不是全局搜索
          console.log(
            "Getting category archives with search params:",
            selectedCategoryId.value,
            searchParams.value,
          );
          const result = await getCategoryArchives(
            selectedCategoryId.value,
            searchParams.value,
          );
          console.log("Category archives with search result:", result);
          return result;
        }
        console.log("Searching with params:", params);
        const result = await searchArchives(params);
        console.log("Search result:", result);
        return result;
      } else if (selectedCategoryId.value) {
        // 没有搜索条件时，获取分类下的漫画
        console.log(
          "Getting category archives:",
          selectedCategoryId.value,
          "with params:",
          searchParams.value,
        );
        const result = await getCategoryArchives(
          selectedCategoryId.value,
          searchParams.value,
        );
        console.log("Category archives result:", result);
        return result;
      } else {
        // 获取所有漫画
        console.log("Getting all archives with params:", searchParams.value);
        const result = await searchArchives(searchParams.value);
        console.log("Archives result:", result);
        return result;
      }
    } catch (err) {
      console.error("API Error:", err);
      throw err;
    }
  },
  retry: 1,
});

// 使用计算属性从 Vue Query 获取数据
const archives = computed<Archive[]>(() => {
  return data.value?.data || [];
});

// 总漫画数量（当前查询的结果）
const totalArchives = computed(() => {
  return data.value?.total || 0;
});

// 获取所有漫画的总数量（用于侧边栏显示）
const { data: allArchivesData } = useQuery({
  queryKey: ["allArchivesCount"],
  queryFn: async () => {
    // 获取所有漫画的数量，使用最简单的查询参数
    const result = await searchArchives({ pageNumb: 1, pageSize: 1 });
    return result;
  },
  staleTime: 5 * 60 * 1000, // 5分钟内认为数据是新鲜的
  cacheTime: 10 * 60 * 1000, // 10分钟后清除缓存
});

// 所有漫画的总数量（不随分类切换而变化）
const allArchivesCount = computed(() => {
  return allArchivesData.value?.total || 0;
});

// 总页数
const totalPages = computed(() => {
  return Math.ceil(totalArchives.value / pageSize.value);
});

// 分页显示逻辑
const visiblePages = computed(() => {
  const pages: number[] = [];
  const maxVisible = 5;
  let start = Math.max(1, currentPage.value - Math.floor(maxVisible / 2));
  let end = Math.min(totalPages.value, start + maxVisible - 1);

  if (end - start + 1 < maxVisible) {
    start = Math.max(1, end - maxVisible + 1);
  }

  for (let i = start; i <= end; i++) {
    if (i !== 1 && i !== totalPages.value) {
      pages.push(i);
    }
  }

  return pages;
});

const showFirstPage = computed(() => {
  return !visiblePages.value.includes(1) && totalPages.value > 1;
});

const showLastPage = computed(() => {
  return !visiblePages.value.includes(totalPages.value) && totalPages.value > 1;
});

const showLeftEllipsis = computed(() => {
  return visiblePages.value.length > 0 && visiblePages.value[0] > 2;
});

const showRightEllipsis = computed(() => {
  return (
    visiblePages.value.length > 0 &&
    visiblePages.value[visiblePages.value.length - 1] < totalPages.value - 1
  );
});

// 获取当前页面所有漫画的进度数据
const { data: batchProgressData } = useQuery({
  queryKey: ["batchProgress", archives],
  queryFn: async () => {
    const archiveIds = archives.value.map((archive) => archive.id);
    if (archiveIds.length === 0) return [];

    console.log(`Loading progress for ${archiveIds.length} archives`);
    const result = await getBatchProgress(archiveIds);
    console.log(`Loaded progress for ${result.length} archives`);
    return result;
  },
  enabled: computed(() => archives.value.length > 0),
  retry: false,
  staleTime: 5 * 60 * 1000, // 5分钟内认为数据是新鲜的
  cacheTime: 10 * 60 * 1000, // 10分钟后清除缓存
});

// 将进度数据转换为Map格式便于查找
watch(
  batchProgressData,
  (newProgressData) => {
    if (newProgressData) {
      const progressMap = new Map<string, ReadingProgress>();
      newProgressData.forEach((progress) => {
        progressMap.set(progress.archiveId, progress);
      });
      progressData.value = progressMap;
    }
  },
  { immediate: true },
);

// 调试用：监视数据变化
watch(
  data,
  (newData) => {
    console.log("Archives data updated:", newData);
  },
  { immediate: true },
);

// 调试：监视分页变化
watch(
  currentPage,
  (newPage) => {
    console.log("Current page changed to:", newPage);
  },
  { immediate: true },
);

// 调试：监视搜索参数变化
watch(
  searchParams,
  (newParams) => {
    console.log("Search params changed:", newParams);
  },
  { immediate: true, deep: true },
);

const handleSearch = async () => {
  console.log("Searching for:", searchQuery.value);
  isSearching.value = true;
  currentPage.value = 1; // Reset to first page on new search
  try {
    await refetch();
  } finally {
    isSearching.value = false;
  }
};

const handleSelectCategory = async (categoryId: string | null) => {
  console.log("Selected category:", categoryId);
  selectedCategoryId.value = categoryId;
  // 清空搜索查询
  searchQuery.value = "";
  currentPage.value = 1; // Reset to first page on category change
  
  // 强制刷新数据以确保立即显示分类下的漫画
  try {
    await refetch();
  } catch (error) {
    console.error("Failed to refetch data after category change:", error);
  }
  
  // 在移动端选择分类后自动折叠侧边栏
  if (window.innerWidth < COLLAPSE_BREAKPOINT) {
    sidebarCollapsed.value = true;
  }
};

const handleEditCategory = (category: Category | DynamicCategory) => {
  selectedCategory.value = category;
  showEditCategoryModal.value = true;
};

const handleCategoryCreated = () => {
  showCreateCategoryModal.value = false;
  // 重新获取分类数据
  queryClient.invalidateQueries({ queryKey: ["categories"] });
  // 重新获取漫画数据
  refetch();
  // 重新获取总数量
  queryClient.invalidateQueries({ queryKey: ["allArchivesCount"] });
};

const handleCategoryUpdated = () => {
  showEditCategoryModal.value = false;
  selectedCategory.value = null;
  // 重新获取分类数据
  queryClient.invalidateQueries({ queryKey: ["categories"] });
  // 重新获取漫画数据
  refetch();
  // 重新获取总数量
  queryClient.invalidateQueries({ queryKey: ["allArchivesCount"] });
};

const openReader = (archiveId: string) => {
  router.push(`/reader/${archiveId}`);
};

const handleAdvancedSearch = async () => {
  console.log("Advanced search with params:", advancedSearch.value);
  isSearching.value = true;
  currentPage.value = 1; // Reset to first page on advanced search
  try {
    await refetch();
  } finally {
    isSearching.value = false;
  }
};

const goToPage = async (page: number) => {
  if (page >= 1 && page <= totalPages.value) {
    console.log("Navigating to page:", page);
    currentPage.value = page;

    // 强制刷新查询以确保新数据被加载
    console.log("Triggering refetch for page:", page);
    await refetch();

    // Scroll to top of content area
    const contentArea = document.querySelector(
      ".library-view .overflow-y-auto",
    );
    if (contentArea) {
      contentArea.scrollTop = 0;
    }
  }
};

const clearAdvancedSearch = () => {
  advancedSearch.value = {
    createdAfter: "",
    createdBefore: "",
    lastReadAfter: "",
    lastReadBefore: "",
    minPages: undefined,
    maxPages: undefined,
    sortBy: "createdAt",
    sortOrder: "asc",
  };
};

// 当从阅读器返回时刷新进度数据
const refreshProgressData = () => {
  console.log("Refreshing progress data");
  // 刷新进度数据查询
  queryClient.invalidateQueries({ queryKey: ["batchProgress"] });
};

// 监听路由变化，当从 reader 返回到 library 时刷新进度
watch(route, (newRoute, oldRoute) => {
  console.log("Route changed:", { from: oldRoute?.name, to: newRoute.name });
  if (newRoute.name === "library" && oldRoute?.name === "reader") {
    console.log("Returned from reader to library, refreshing progress");
    // 延迟一点刷新，确保进度已经保存
    setTimeout(refreshProgressData, 100);
  }
});

// 响应式侧边栏逻辑
const checkScreenWidth = () => {
  const width = window.innerWidth;
  const shouldCollapse = width < COLLAPSE_BREAKPOINT;
  if (sidebarCollapsed.value !== shouldCollapse) {
    console.log(
      `Screen width: ${width}px, setting sidebar collapsed: ${shouldCollapse}`,
    );
    sidebarCollapsed.value = shouldCollapse;
  }
};

// 防抖处理 resize 事件
let resizeTimeout: number | null = null;
const handleWindowResize = () => {
  if (resizeTimeout) {
    clearTimeout(resizeTimeout);
  }
  resizeTimeout = setTimeout(() => {
    checkScreenWidth();
    resizeTimeout = null;
  }, 150);
};

// 处理侧边栏手动切换
const handleToggleCollapse = (collapsed: boolean) => {
  console.log("Sidebar manually toggled:", collapsed);
  sidebarCollapsed.value = collapsed;
};

// 右键菜单处理函数
const handleArchiveContextMenu = (event: MouseEvent, archive: Archive) => {
  contextMenuArchive.value = archive;
  contextMenuPosition.value = { x: event.clientX, y: event.clientY };
  showContextMenu.value = true;
};

const closeContextMenu = () => {
  showContextMenu.value = false;
  contextMenuArchive.value = null;
};

// 从右键菜单添加标签
const handleAddTagFromContext = () => {
  if (!contextMenuArchive.value) return;
  
  tagModalArchive.value = contextMenuArchive.value;
  showTagModal.value = true;
  closeContextMenu();
};

// 添加标签到档案
const handleAddTagToArchive = async (archiveId: string, tagName: string, namespace: string) => {
  try {
    // 先创建或获取标签
    const tag = await createTag(tagName, namespace);
    
    // 将标签关联到档案
    await addTagToArchive(archiveId, tag.id);
    
    // 刷新档案数据
    refetch();
    
    console.log(`Successfully added tag "${tagName}" to archive`);
  } catch (error) {
    console.error('Failed to add tag to archive:', error);
    alert('添加标签失败，请稍后重试');
  }
};

// 从右键菜单添加到分类
const handleAddToCategoryFromContext = (categoryId: string) => {
  if (!contextMenuArchive.value) return;
  
  handleAddArchiveToCategory(contextMenuArchive.value.id, categoryId);
  closeContextMenu();
};

// 从右键菜单移出分类
const handleRemoveFromCategoryFromContext = (categoryId: string) => {
  if (!contextMenuArchive.value) return;
  
  handleRemoveArchiveFromCategory(contextMenuArchive.value.id, categoryId);
  closeContextMenu();
};

// 添加档案到分类
const handleAddArchiveToCategory = async (archiveId: string, categoryId: string) => {
  try {
    await addArchivesToCategory(categoryId, { archiveIds: [archiveId] });
    
    // 刷新档案数据和分类数据
    queryClient.invalidateQueries({ queryKey: ["categories"] });
    queryClient.invalidateQueries({ queryKey: ["archiveCategories", archiveId] });
    refetch();
    
    console.log('Successfully added archive to category');
  } catch (error) {
    console.error('Failed to add archive to category:', error);
    alert('添加到分类失败，请稍后重试');
  }
};

// 从分类移出档案
const handleRemoveArchiveFromCategory = async (archiveId: string, categoryId: string) => {
  try {
    await removeArchivesFromCategory(categoryId, { archiveIds: [archiveId] });
    
    // 刷新档案数据和分类数据
    queryClient.invalidateQueries({ queryKey: ["categories"] });
    queryClient.invalidateQueries({ queryKey: ["archiveCategories", archiveId] });
    refetch();
    
    console.log('Successfully removed archive from category');
  } catch (error) {
    console.error('Failed to remove archive from category:', error);
    alert('移出分类失败，请稍后重试');
  }
};

// 从右键菜单删除档案
const handleDeleteArchiveFromContext = () => {
  if (!contextMenuArchive.value) return;
  
  const archive = contextMenuArchive.value;
  const confirmDelete = confirm(`确定要删除漫画《${archive.title}》吗？此操作不可撤销。`);
  
  if (confirmDelete) {
    handleDeleteArchiveAction(archive.id);
  }
  
  closeContextMenu();
};

// 删除档案
const handleDeleteArchiveAction = async (archiveId: string) => {
  try {
    await deleteArchive(archiveId);
    
    // 刷新档案数据和分类数据
    queryClient.invalidateQueries({ queryKey: ["categories"] });
    // 重新获取总数量
    queryClient.invalidateQueries({ queryKey: ["allArchivesCount"] });
    refetch();
    
    console.log('Successfully deleted archive');
  } catch (error) {
    console.error('Failed to delete archive:', error);
    alert('删除失败，请稍后重试');
  }
};

// 处理标签模态框提交
const handleTagModalSubmit = async (tagName: string, namespace: string) => {
  if (!tagModalArchive.value) return;
  
  await handleAddTagToArchive(tagModalArchive.value.id, tagName, namespace);
  showTagModal.value = false;
  tagModalArchive.value = null;
};

// 关闭标签模态框
const closeTagModal = () => {
  showTagModal.value = false;
  tagModalArchive.value = null;
};

onMounted(() => {
  console.log("LibraryView mounted");
  // 初始检查屏幕宽度
  checkScreenWidth();
  // 添加响应式事件监听器
  window.addEventListener("resize", handleWindowResize);
  // 添加全局点击事件监听，用于关闭右键菜单
  document.addEventListener("click", closeContextMenu);
  // 页面加载时刷新一次进度数据（仅在库页面）
  if (route.name === "library") {
    refreshProgressData();
  }
});

onUnmounted(() => {
  window.removeEventListener("resize", handleWindowResize);
  document.removeEventListener("click", closeContextMenu);
});

// 移除 onActivated，因为它只在 KeepAlive 下工作
// onActivated(() => {
//   console.log('LibraryView activated, refreshing progress data')
//   // 刷新进度数据查询
//   queryClient.invalidateQueries({ queryKey: ['batchProgress'] })
// })
</script>

<style scoped>
/* 只保留Library页面特有的样式 */
.library-view {
  /* Library特有样式如需要可在此处添加 */
}
</style>
