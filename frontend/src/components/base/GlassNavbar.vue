<template>
  <nav
    :class="[
      'glass-navbar sticky top-0 z-50 transition-all duration-300',
      'bg-white/10 backdrop-blur-md border-b border-white/20',
      'shadow-lg',
      fixed && 'fixed left-0 right-0',
      transparent && 'bg-transparent border-transparent',
      className,
    ]"
  >
    <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
      <div class="flex items-center justify-between h-16">
        <!-- 左侧：Logo 和导航链接 -->
        <div class="flex items-center space-x-8">
          <!-- Logo -->
          <div class="flex items-center">
            <slot name="logo">
              <div class="text-2xl font-bold text-white">
                {{ title }}
              </div>
            </slot>
          </div>

          <!-- 导航链接 -->
          <div
            v-if="$slots.navigation"
            class="hidden md:flex items-center space-x-6"
          >
            <slot name="navigation" />
          </div>
        </div>

        <!-- 中间：搜索框 -->
        <div v-if="$slots.search"
class="flex-1 max-w-lg mx-8 hidden sm:block">
          <slot name="search" />
        </div>

        <!-- 右侧：用户操作 -->
        <div class="flex items-center space-x-4">
          <slot name="actions" />

          <!-- 移动端菜单按钮 -->
          <button
            v-if="showMobileMenu"
            class="md:hidden p-2 rounded-lg text-white/80 hover:text-white hover:bg-white/10 transition-colors"
            type="button"
            @click="toggleMobileMenu"
          >
            <svg
              class="w-6 h-6"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                v-if="!mobileMenuOpen"
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M4 6h16M4 12h16M4 18h16"
              />
              <path
                v-else
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>
      </div>
    </div>

    <!-- 移动端菜单 -->
    <transition
      enter-active-class="transition-all duration-300 ease-out"
      enter-from-class="opacity-0 transform -translate-y-2"
      enter-to-class="opacity-100 transform translate-y-0"
      leave-active-class="transition-all duration-200 ease-in"
      leave-from-class="opacity-100 transform translate-y-0"
      leave-to-class="opacity-0 transform -translate-y-2"
    >
      <div
        v-if="mobileMenuOpen && showMobileMenu"
        class="md:hidden border-t border-white/20 bg-white/5 backdrop-blur-md"
      >
        <div class="px-4 py-6 space-y-4">
          <!-- 移动端搜索 -->
          <div v-if="$slots.search"
class="sm:hidden">
            <slot name="search" />
          </div>

          <!-- 移动端导航 -->
          <div v-if="$slots.mobileNavigation"
class="space-y-2">
            <slot name="mobileNavigation" />
          </div>

          <!-- 移动端操作 -->
          <div
            v-if="$slots.mobileActions"
            class="pt-4 border-t border-white/20"
          >
            <slot name="mobileActions" />
          </div>
        </div>
      </div>
    </transition>
  </nav>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
interface Props {
  title?: string;
  fixed?: boolean;
  transparent?: boolean;
  showMobileMenu?: boolean;
  className?: string;
}

const props = withDefaults(defineProps<Props>(), {
  title: "OtamoryX",
  fixed: false,
  transparent: false,
  showMobileMenu: true,
  className: "",
});

const emit = defineEmits<{
  mobileMenuToggle: [open: boolean];
}>();

const mobileMenuOpen = ref(false);

const toggleMobileMenu = () => {
  mobileMenuOpen.value = !mobileMenuOpen.value;
  emit("mobileMenuToggle", mobileMenuOpen.value);
};

// 监听窗口大小变化，大屏幕时关闭移动菜单
const handleResize = () => {
  if (window.innerWidth >= 768 && mobileMenuOpen.value) {
    mobileMenuOpen.value = false;
    emit("mobileMenuToggle", false);
  }
};

onMounted(() => {
  window.addEventListener("resize", handleResize);
});

onUnmounted(() => {
  window.removeEventListener("resize", handleResize);
});
</script>

<style scoped>
.glass-navbar {
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
}

.glass-navbar::before {
  content: "";
  position: absolute;
  inset: 0;
  background: linear-gradient(
    135deg,
    rgba(255, 255, 255, 0.1) 0%,
    rgba(255, 255, 255, 0.05) 50%,
    rgba(255, 255, 255, 0.1) 100%
  );
  pointer-events: none;
}

/* 响应式调整 */
@media (max-width: 640px) {
  .glass-navbar {
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
  }
}
</style>
