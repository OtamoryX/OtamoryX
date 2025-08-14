<template>
  <div class="tag-manager space-y-4">
    <!-- 标题和添加按钮 -->
    <div class="flex items-center justify-between">
      <h3 class="text-base font-medium text-green-400 flex items-center">
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
            d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z"
          />
        </svg>
        标签管理
      </h3>
      <GlassButton
        variant="success"
        size="sm"
        class="flex items-center"
        @click="showAddTagModal = true"
      >
        <svg
          class="w-4 h-4 mr-1"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M12 6v6m0 0v6m0-6h6m-6 0H6"
          />
        </svg>
        添加标签
      </GlassButton>
    </div>

    <!-- 标签列表 -->
    <div class="space-y-2">
      <div v-if="tags?.length"
class="flex flex-wrap gap-2">
        <TagChip
          v-for="tag in tags"
          :key="tag.id"
          :tag="tag"
          removable
          @remove="handleRemoveTag"
        />
      </div>
      <p
v-else class="text-white/50 text-sm italic">暂无标签</p>
    </div>

    <!-- 添加标签模态框 -->
    <BaseModal
      :show="showAddTagModal"
      title="添加标签"
      width="md"
      @close="showAddTagModal = false"
    >
      <div class="space-y-4">
        <GlassInput
          v-model="newTag.namespace"
          label="命名空间"
          placeholder="例如: 作者, 分类, 语言"
        >
          <template #prefix>
            <svg
              class="w-4 h-4"
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
          </template>
        </GlassInput>

        <GlassInput
          v-model="newTag.name"
          label="标签值"
          placeholder="例如: 尾田荣一郎, 少年漫画, 中文"
          @keyup.enter="handleAddTag"
        >
          <template #prefix>
            <svg
              class="w-4 h-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z"
              />
            </svg>
          </template>
        </GlassInput>
      </div>

      <template #footer>
        <div class="flex justify-end space-x-3">
          <GlassButton
variant="ghost" @click="showAddTagModal = false"
>
            取消
          </GlassButton>
          <GlassButton
            :disabled="!newTag.namespace || !newTag.name"
            variant="primary"
            glow-effect
            @click="handleAddTag"
          >
            添加
          </GlassButton>
        </div>
      </template>
    </BaseModal>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import type { Tag } from "@/types/api";
import BaseModal from "@/components/base/BaseModal.vue";
import TagChip from "@/components/base/TagChip.vue";
import GlassButton from "@/components/base/GlassButton.vue";
import GlassInput from "@/components/base/GlassInput.vue";

interface Props {
  tags?: Tag[];
}

defineProps<Props>();

const emit = defineEmits<{
  "add-tag": [tag: { namespace: string; name: string }];
  "remove-tag": [tagId: string];
}>();

const showAddTagModal = ref(false);
const newTag = ref({
  namespace: "",
  name: "",
});

const handleAddTag = () => {
  if (!newTag.value.namespace || !newTag.value.name) return;

  emit("add-tag", { ...newTag.value });

  // 重置表单
  newTag.value = { namespace: "", name: "" };
  showAddTagModal.value = false;
};

const handleRemoveTag = (tagId: string) => {
  emit("remove-tag", tagId);
};

// 监听模态框关闭，重置表单
watch(showAddTagModal, (show) => {
  if (!show) {
    newTag.value = { namespace: "", name: "" };
  }
});
</script>

<style scoped>
/* 样式已在Tailwind中定义，无需额外样式 */
</style>
