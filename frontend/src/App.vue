<template>
  <div id="app" class="min-h-screen w-full">
    <AppHeader v-if="authStore.isAuthenticated" />

    <main class="w-full">
      <RouterView v-slot="{ Component, route: currentRoute }">
        <KeepAlive :include="cachedViewNames">
          <component :is="Component" :key="currentRoute.name" class="w-full" />
        </KeepAlive>
      </RouterView>
    </main>
  </div>
</template>

<script setup lang="ts">
import { onMounted, watch } from "vue";
import { RouterView } from "vue-router";
import { useAuthStore } from "@/stores/auth";
import { useTheme } from "@/composables/useTheme";
import { useTitleDisplayStore } from "@/stores/titleDisplay";
import AppHeader from "@/components/layout/AppHeader.vue";

const authStore = useAuthStore();
const titleDisplayStore = useTitleDisplayStore();
const cachedViewNames = ["LibraryView", "TagsView"];

// 在根组件初始化主题，保证全局生效
useTheme();

onMounted(() => {
  // 初始化认证状态
  authStore.initAuth();

  if (authStore.isAuthenticated) {
    void titleDisplayStore.load();
  }

});

watch(
  () => authStore.isAuthenticated,
  (authenticated) => {
    if (authenticated) void titleDisplayStore.load(true);
  },
);
</script>
