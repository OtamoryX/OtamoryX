<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <div>
        <h2 class="text-lg font-medium text-[var(--text-primary)]">用户管理</h2>
        <p class="mt-1 text-sm text-[var(--text-secondary)]">创建、查看和删除系统用户</p>
      </div>
      <GlassButton size="sm" variant="primary" @click="emit('create-user')">创建用户</GlassButton>
    </div>

    <GlassCard v-if="usersLoading" size="md" radius="lg">
      <div class="py-6 text-center text-[var(--text-secondary)]">加载中...</div>
    </GlassCard>

    <GlassCard v-else-if="!users || users.length === 0" size="md" radius="lg">
      <div class="py-6 text-center text-[var(--text-secondary)]">暂无用户</div>
    </GlassCard>

    <GlassCard v-else size="md" radius="lg" class="!p-0 overflow-hidden">
      <ul class="divide-y divide-[var(--border)]">
        <li v-for="user in users" :key="user.id" class="px-6 py-4">
          <div class="flex items-center justify-between gap-4">
            <div class="flex items-center gap-4">
              <div class="flex h-10 w-10 items-center justify-center rounded-full bg-[var(--bg-tertiary)] text-sm font-medium text-[var(--text-primary)]">
                {{ user.username.charAt(0).toUpperCase() }}
              </div>
              <div>
                <div class="text-sm font-medium text-[var(--text-primary)]">{{ user.username }}</div>
                <div class="text-sm text-[var(--text-secondary)]">{{ user.email || '未设置邮箱' }}</div>
                <div class="text-xs text-[var(--text-tertiary)]">创建于 {{ formatDate(user.createdAt) }}</div>
              </div>
            </div>

            <div class="flex items-center gap-2">
              <GlassButton size="sm" variant="secondary" @click="emit('edit-user', user)">编辑</GlassButton>
              <GlassButton size="sm" variant="danger" @click="emit('delete-user', user)">删除</GlassButton>
            </div>
          </div>
        </li>
      </ul>
    </GlassCard>
  </div>
</template>

<script setup lang="ts">
import GlassButton from "@/components/base/GlassButton.vue";
import GlassCard from "@/components/base/GlassCard.vue";
import type { User } from "@/types/api";

interface Props {
  users?: User[];
  usersLoading: boolean;
}

defineProps<Props>();

const emit = defineEmits<{
  "create-user": [];
  "edit-user": [user: User];
  "delete-user": [user: User];
}>();

const formatDate = (dateString: string) => {
  const date = new Date(dateString);
  return date.toLocaleDateString("zh-CN", {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
};
</script>
