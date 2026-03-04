<template>
  <div id="app" class="min-h-screen w-full">
    <!-- 全局导航栏 - 只在非书库页面且认证后显示 -->
    <nav v-if="authStore.isAuthenticated && !isLibraryRoute"
      class="nav-bar sticky top-0 z-30 bg-[#1b1b2f] border-b border-[#2d2d44] shrink-0">
      <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div class="flex justify-between h-14">
          <div class="flex items-center">
            <RouterLink to="/library"
              class="text-lg font-semibold text-[#e0e0e0] hover:text-white transition-colors cursor-pointer flex items-center">
              <svg class="w-5 h-5 mr-2 text-[#7b68ee]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.746 0 3.332.477 4.5 1.253v13C20.832 18.477 19.246 18 17.5 18c-1.746 0-3.332.477-4.5 1.253z" />
              </svg>
              OtamoryX
            </RouterLink>
          </div>
          <div class="flex items-center space-x-3">
            <RouterLink to="/library"
              class="text-[#b0b0b0] hover:text-white hover:bg-white/10 px-3 py-1.5 rounded text-sm transition-colors">
              书库
            </RouterLink>
            <RouterLink to="/settings"
              class="text-[#b0b0b0] hover:text-white hover:bg-white/10 px-3 py-1.5 rounded text-sm transition-colors">
              设置
            </RouterLink>

            <RouterLink v-if="authStore.isAdmin" :to="{ name: 'admin-settings', query: { tab: 'system' } }"
              class="text-[#b0b0b0] hover:text-white hover:bg-white/10 px-3 py-1.5 rounded text-sm transition-colors">
              管理
            </RouterLink>

            <!-- 用户菜单 -->
            <div class="relative">
              <button data-menu="user"
                class="flex items-center text-[#b0b0b0] hover:text-white hover:bg-white/10 px-3 py-1.5 rounded text-sm transition-colors"
                @click="showUserMenu = !showUserMenu">
                {{ authStore.user?.username || "用户" }}
              </button>
              <div v-if="showUserMenu" data-dropdown="user"
                class="absolute right-0 mt-1 w-40 bg-[#1b1b2f] border border-[#2d2d44] rounded shadow-lg z-20">
                <div class="px-4 py-2 text-xs text-[#808080] border-b border-[#2d2d44]">
                  {{ authStore.user?.role === "admin" ? "管理员" : "普通用户" }}
                </div>
                <button
                  class="flex items-center w-full text-left px-4 py-2 text-sm text-red-400 hover:bg-[#2d2d44] transition-colors"
                  @click="handleLogout">
                  退出登录
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </nav>

    <main class="w-full">
      <RouterView class="w-full" />
    </main>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { RouterLink, RouterView, useRouter, useRoute } from "vue-router";
import { useAuthStore } from "@/stores/auth";
import { useTheme } from "@/composables/useTheme";

const router = useRouter();
const route = useRoute();
const authStore = useAuthStore();
const showUserMenu = ref(false);

// 在根组件初始化主题，保证全局生效
useTheme();

const isLibraryRoute = computed(() => route.name === 'library');

const handleLogout = () => {
  authStore.logout();
  showUserMenu.value = false;
  router.push("/login");
};

// 点击外部关闭菜单
const handleClickOutside = (event: Event) => {
  const target = event.target as Element;

  // 检查点击是否在用户菜单区域内
  const userMenuButton = document.querySelector('[data-menu="user"]');
  const userMenuDropdown = document.querySelector('[data-dropdown="user"]');

  if (showUserMenu.value && userMenuButton && userMenuDropdown) {
    if (
      !userMenuButton.contains(target) &&
      !userMenuDropdown.contains(target)
    ) {
      showUserMenu.value = false;
    }
  }

};

onMounted(() => {
  // 初始化认证状态
  authStore.initAuth();

  // 添加点击外部事件监听
  document.addEventListener("click", handleClickOutside);
});
</script>

<style scoped>
@reference "@/assets/main.css";
/* 简洁导航栏样式 */
.nav-bar {
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
}

/* 活跃链接样式 */
.router-link-active {
  color: #e0e0e0;
  background: rgba(255, 255, 255, 0.1);
}

/* 菜单下拉动画 */
.nav-bar div[class*="absolute"] {
  animation: slideDown 0.15s ease-out;
}

@keyframes slideDown {
  from {
    opacity: 0;
    transform: translateY(-4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
