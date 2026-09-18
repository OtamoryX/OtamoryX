<template>
  <header
    v-if="showHeader"
    class="app-header sticky top-0 z-40 h-[calc(env(safe-area-inset-top,0px)+3.5rem)] border-b border-[#2d2d44] bg-[#1b1b2f] shadow-[0_1px_3px_rgba(0,0,0,0.3)] md:h-14"
  >
    <div
      class="mx-auto flex h-full max-w-7xl items-end justify-between gap-2 px-3 pb-2 md:items-center md:px-4 md:pb-0"
    >
      <RouterLink
        to="/library"
        class="flex shrink-0 items-center text-[#e0e0e0] transition-colors hover:text-white"
        aria-label="OtamoryX 书库"
      >
        <BookOpenIcon class="h-5 w-5 text-[#7b68ee] min-[390px]:mr-1.5" />
        <span class="hidden text-base font-semibold min-[390px]:inline"
          >OtamoryX</span
        >
      </RouterLink>

      <nav
        class="flex min-w-0 items-center justify-end gap-0.5 sm:gap-1"
        aria-label="主导航"
      >
        <RouterLink
          to="/library"
          :class="navLinkClass('library')"
          aria-label="书库"
          title="书库"
        >
          <BookOpenIcon class="h-4 w-4" aria-hidden="true" />
          <span class="hidden sm:inline">书库</span>
        </RouterLink>
        <RouterLink
          to="/reading-history"
          :class="navLinkClass('reading-history')"
          aria-label="阅读记录"
          title="阅读记录"
        >
          <ClockIcon class="h-4 w-4" aria-hidden="true" />
          <span class="hidden sm:inline">阅读记录</span>
        </RouterLink>
        <RouterLink
          to="/tags"
          :class="navLinkClass('tags')"
          aria-label="标签"
          title="标签"
        >
          <TagIcon class="h-4 w-4" aria-hidden="true" />
          <span class="hidden sm:inline">标签</span>
        </RouterLink>

        <div ref="userMenuRef" class="relative ml-0.5 sm:ml-1">
          <button
            type="button"
            class="flex h-9 min-w-9 items-center justify-center gap-1.5 rounded px-2 text-sm text-[#c0c0c0] transition-colors hover:bg-white/10 hover:text-white"
            :aria-expanded="showUserMenu"
            aria-haspopup="menu"
            aria-label="用户菜单"
            title="用户菜单"
            @click.stop="toggleUserMenu"
            @keydown.esc="closeUserMenu"
          >
            <span
              class="flex h-6 w-6 items-center justify-center rounded-full bg-[#7b68ee] text-xs font-semibold text-white"
              aria-hidden="true"
            >
              {{ userInitial }}
            </span>
            <span class="hidden max-w-32 truncate lg:inline">{{
              userName
            }}</span>
            <ChevronDownIcon
              class="hidden h-3.5 w-3.5 text-[#808090] lg:inline"
              aria-hidden="true"
            />
          </button>

          <Transition name="dropdown">
            <div
              v-if="showUserMenu"
              class="absolute right-0 z-50 mt-1 w-44 overflow-hidden rounded border border-[#2d2d44] bg-[#1b1b2f] shadow-lg"
              role="menu"
              @keydown.esc="closeUserMenu"
            >
              <div
                class="border-b border-[#2d2d44] px-3 py-2 text-xs text-[#808080]"
              >
                {{ userName }}
              </div>
              <RouterLink
                to="/settings"
                class="flex w-full items-center gap-2 px-3 py-2 text-sm text-[#c0c0d0] transition-colors hover:bg-[#2d2d44]"
                role="menuitem"
                @click="closeUserMenu"
              >
                <Cog6ToothIcon class="h-4 w-4" aria-hidden="true" />
                个人设置
              </RouterLink>
              <RouterLink
                v-if="authStore.isAdmin"
                :to="{ name: 'admin-settings', query: { tab: 'system' } }"
                class="flex w-full items-center gap-2 px-3 py-2 text-sm text-[#c0c0d0] transition-colors hover:bg-[#2d2d44]"
                role="menuitem"
                @click="closeUserMenu"
              >
                <WrenchScrewdriverIcon class="h-4 w-4" aria-hidden="true" />
                管理设置
              </RouterLink>
              <button
                type="button"
                class="flex w-full items-center gap-2 px-3 py-2 text-left text-sm text-red-400 transition-colors hover:bg-[#2d2d44]"
                role="menuitem"
                @click="handleLogout"
              >
                <ArrowRightOnRectangleIcon class="h-4 w-4" aria-hidden="true" />
                退出登录
              </button>
            </div>
          </Transition>
        </div>
      </nav>
    </div>
  </header>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { RouterLink, useRoute, useRouter } from "vue-router";
import {
  ArrowRightOnRectangleIcon,
  BookOpenIcon,
  ChevronDownIcon,
  ClockIcon,
  Cog6ToothIcon,
  TagIcon,
  WrenchScrewdriverIcon,
} from "@heroicons/vue/24/outline";
import { useAuthStore } from "@/stores/auth";

const route = useRoute();
const router = useRouter();
const authStore = useAuthStore();
const showUserMenu = ref(false);
const userMenuRef = ref<HTMLElement | null>(null);

const showHeader = computed(
  () => route.name !== "reader" && route.name !== "version-compare",
);
const userName = computed(() => authStore.user?.username || "用户");
const userInitial = computed(() => userName.value.charAt(0).toUpperCase());

const navLinkClass = (routeName: string) => [
  "inline-flex h-9 items-center justify-center gap-1.5 rounded px-2 text-sm text-[#a0a0a0] transition-colors hover:bg-white/10 hover:text-white sm:px-2.5",
  route.name === routeName ? "bg-white/10 text-white" : "",
];

const closeUserMenu = () => {
  showUserMenu.value = false;
};

const toggleUserMenu = () => {
  showUserMenu.value = !showUserMenu.value;
};

const handleLogout = () => {
  authStore.logout();
  closeUserMenu();
  void router.push("/login");
};

const handleClickOutside = (event: MouseEvent) => {
  const target = event.target as Node | null;
  if (target && showUserMenu.value && !userMenuRef.value?.contains(target)) {
    closeUserMenu();
  }
};

watch(
  () => route.fullPath,
  () => closeUserMenu(),
);

onMounted(() => document.addEventListener("click", handleClickOutside));
onUnmounted(() => document.removeEventListener("click", handleClickOutside));
</script>

<style scoped>
.dropdown-enter-active,
.dropdown-leave-active {
  transition: all 0.15s ease;
}

.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}

.dropdown-enter-to,
.dropdown-leave-from {
  opacity: 1;
  transform: translateY(0);
}
</style>
