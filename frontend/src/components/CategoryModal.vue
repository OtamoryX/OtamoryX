<template>
  <BaseModal
    :show="true"
    :title="isEditing ? '编辑分类' : '创建分类'"
    width="lg"
    :z-index="9999"
    @close="$emit('close')"
  >
    <div class="space-y-6">
      <!-- 分类类型选择（仅创建时显示） -->
      <div v-if="!isEditing">
        <label class="block text-sm font-medium text-[var(--text-primary)] mb-2">分类类型</label>
        <div class="flex space-x-6">
          <label class="flex items-center cursor-pointer group">
            <input
              v-model="categoryType"
              type="radio"
              value="static"
              class="h-4 w-4 text-[var(--accent)] bg-[var(--bg-tertiary)] border-[var(--border)] focus:ring-[var(--accent)] focus:ring-offset-0 rounded"
            />
            <span class="ml-3 text-sm text-[var(--text-primary)] group-hover:text-[var(--accent)]">静态分类</span>
          </label>
          <label class="flex items-center cursor-pointer group">
            <input
              v-model="categoryType"
              type="radio"
              value="dynamic"
              class="h-4 w-4 text-[var(--accent)] bg-[var(--bg-tertiary)] border-[var(--border)] focus:ring-[var(--accent)] focus:ring-offset-0 rounded"
            />
            <span class="ml-3 text-sm text-[var(--text-primary)] group-hover:text-[var(--accent)]">动态分类</span>
          </label>
        </div>
      </div>

      <!-- 分类名称 -->
      <div>
        <label for="name"
class="block text-sm font-medium text-[var(--text-primary)] mb-2">
          分类名称 <span class="text-red-400">*</span>
        </label>
        <input
          id="name"
          v-model="form.name"
          type="text"
          required
          class="w-full px-4 py-3 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded-lg text-[var(--text-primary)] placeholder-[var(--text-tertiary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] focus:bg-[var(--bg-secondary)] transition-all"
          placeholder="输入分类名称"
        />
      </div>

      <!-- 描述 -->
      <div>
        <label
          for="description"
          class="block text-sm font-medium text-[var(--text-primary)] mb-2"
        >
          描述
        </label>
        <textarea
          id="description"
          v-model="form.description"
          rows="3"
          class="w-full px-4 py-3 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded-lg text-[var(--text-primary)] placeholder-[var(--text-tertiary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] focus:bg-[var(--bg-secondary)] transition-all resize-none"
          placeholder="输入分类描述（可选）"
        />
      </div>

      <!-- 分类类型显示（编辑时显示） -->
      <div
        v-if="isEditing"
        class="bg-[var(--bg-tertiary)] p-4 rounded-lg border border-[var(--border)]"
      >
        <div class="flex items-center space-x-3">
          <div
            :class="[
              'p-2 rounded-lg',
              isStatic
                ? 'bg-purple-500/20 text-purple-300'
                : 'bg-green-500/20 text-green-300',
            ]"
          >
            <svg
              v-if="isStatic"
              class="w-5 h-5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
              />
            </svg>
            <svg
              v-else
              class="w-5 h-5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M13 10V3L4 14h7v7l9-11h-7z"
              />
            </svg>
          </div>
          <div>
            <span class="text-[var(--text-primary)] font-medium">
              {{ isStatic ? "静态分类" : "动态分类" }}
            </span>
            <p class="text-xs text-[var(--text-secondary)] mt-1">
              {{ isStatic ? "手动管理漫画分类" : "根据搜索条件自动分类" }}
            </p>
          </div>
        </div>
      </div>

      <!-- 动态分类的搜索条件 -->
      <div v-if="!isEditing && categoryType === 'dynamic'"
class="space-y-4">
        <h4 class="text-sm font-semibold text-[var(--accent)] flex items-center">
          <svg
            class="w-4 h-4 mr-2"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M13 10V3L4 14h7v7l9-11h-7z"
            />
          </svg>
          搜索条件
        </h4>

        <div class="bg-[var(--bg-tertiary)] rounded-lg p-4 space-y-4">
          <div>
            <label class="block text-sm font-medium text-[var(--text-primary)] mb-2">标题关键词</label>
            <input
              v-model="searchParams.query"
              type="text"
              class="w-full px-4 py-3 bg-[var(--bg-secondary)] border border-[var(--border)] rounded-lg text-[var(--text-primary)] placeholder-[var(--text-tertiary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] focus:bg-[var(--bg-secondary)] transition-all"
              placeholder="例如：海贼王"
            />
          </div>

          <div class="grid grid-cols-2 gap-4">
            <div>
              <label class="block text-sm font-medium text-[var(--text-primary)] mb-2">最小页数</label>
              <input
                v-model.number="searchParams.minPages"
                type="number"
                class="w-full px-4 py-3 bg-[var(--bg-secondary)] border border-[var(--border)] rounded-lg text-[var(--text-primary)] placeholder-[var(--text-tertiary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] focus:bg-[var(--bg-secondary)] transition-all"
                placeholder="0"
              />
            </div>
            <div>
              <label class="block text-sm font-medium text-[var(--text-primary)] mb-2">最大页数</label>
              <input
                v-model.number="searchParams.maxPages"
                type="number"
                class="w-full px-4 py-3 bg-[var(--bg-secondary)] border border-[var(--border)] rounded-lg text-[var(--text-primary)] placeholder-[var(--text-tertiary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] focus:bg-[var(--bg-secondary)] transition-all"
                placeholder="999"
              />
            </div>
          </div>

          <div>
            <label class="block text-sm font-medium text-[var(--text-primary)] mb-2">标签（用逗号分隔）</label>
            <input
              v-model="tagsInput"
              type="text"
              class="w-full px-4 py-3 bg-[var(--bg-secondary)] border border-[var(--border)] rounded-lg text-[var(--text-primary)] placeholder-[var(--text-tertiary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] focus:bg-[var(--bg-secondary)] transition-all"
              placeholder="例如：少年漫画,冒险"
            />
          </div>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="flex justify-between">
        <!-- 删除按钮（仅编辑时显示） -->
        <button
          v-if="isEditing"
          :disabled="isLoading"
          class="px-6 py-2 bg-red-600/20 hover:bg-red-600/30 text-red-300 border border-red-400/30 rounded-lg transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed flex items-center"
          @click="handleDelete"
        >
          <svg
            class="w-4 h-4 mr-2"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
            />
          </svg>
          删除
        </button>
        <div v-else />
        <!-- 占位元素 -->

        <div class="flex space-x-3">
          <button
            type="button"
            class="px-6 py-2 text-[var(--text-secondary)] hover:text-[var(--text-primary)] bg-[var(--bg-tertiary)] hover:bg-[var(--bg-tertiary)] rounded-lg transition-all duration-200"
            @click="$emit('close')"
          >
            取消
          </button>
          <button
            :disabled="isLoading || !form.name?.trim()"
            class="px-6 py-2 bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white rounded-lg transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed flex items-center"
            @click="handleSubmit"
          >
            <svg
              v-if="isLoading"
              class="animate-spin -ml-1 mr-2 h-4 w-4"
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
            {{
              isLoading
                ? isEditing
                  ? "保存中..."
                  : "创建中..."
                : isEditing
                  ? "保存"
                  : "创建"
            }}
          </button>
        </div>
      </div>
    </template>
  </BaseModal>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import BaseModal from "@/components/base/BaseModal.vue";
import {
  createCategory,
  createDynamicCategory,
  updateCategory,
  deleteCategory,
} from "@/utils/api";
import type {
  Category,
  DynamicCategory,
  CreateCategoryRequest,
  CreateDynamicCategoryRequest,
  UpdateCategoryRequest,
  SearchParams,
} from "@/types/api";

interface Props {
  category?: Category | DynamicCategory; // 编辑模式时传入
}

const props = defineProps<Props>();

const emit = defineEmits<{
  close: [];
  created: [];
  updated: [];
}>();

const isLoading = ref(false);
const isEditing = computed(() => !!props.category);
const categoryType = ref<"static" | "dynamic">("static");

const form = ref({
  name: "",
  description: "",
});

const searchParams = ref<SearchParams>({
  query: "",
  minPages: undefined,
  maxPages: undefined,
  tags: [],
});

const tagsInput = ref("");

const isStatic = computed(() => {
  if (!props.category) return true;
  return "isStatic" in props.category ? props.category.isStatic : false;
});

// 处理标签输入
const processedSearchParams = computed(() => ({
  ...searchParams.value,
  tags: tagsInput.value
    ? tagsInput.value
        .split(",")
        .map((tag) => tag.trim())
        .filter(Boolean)
    : undefined,
}));

const handleSubmit = async () => {
  if (!form.value.name?.trim()) return;

  isLoading.value = true;
  try {
    if (isEditing.value) {
      // 编辑模式
      const request: UpdateCategoryRequest = {
        name: form.value.name.trim(),
        description: form.value.description?.trim() || undefined,
      };
      await updateCategory(props.category!.id, request);
      emit("updated");
    } else {
      // 创建模式
      if (categoryType.value === "static") {
        const request: CreateCategoryRequest = {
          name: form.value.name.trim(),
          description: form.value.description.trim() || undefined,
        };
        await createCategory(request);
      } else {
        const request: CreateDynamicCategoryRequest = {
          name: form.value.name.trim(),
          description: form.value.description.trim() || undefined,
          searchParams: processedSearchParams.value,
        };
        await createDynamicCategory(request);
      }
      emit("created");
    }
  } catch (error) {
    console.error("Failed to save category:", error);
    // TODO: 显示错误提示
  } finally {
    isLoading.value = false;
  }
};

const handleDelete = async () => {
  if (!props.category || !confirm("确定要删除这个分类吗？此操作不可撤销。")) {
    return;
  }

  isLoading.value = true;
  try {
    await deleteCategory(props.category.id);
    emit("updated");
  } catch (error) {
    console.error("Failed to delete category:", error);
    // TODO: 显示错误提示
  } finally {
    isLoading.value = false;
  }
};

onMounted(() => {
  if (props.category) {
    // 编辑模式：填充现有数据
    form.value.name = props.category.name;
    form.value.description = props.category.description || "";
  }
});
</script>
