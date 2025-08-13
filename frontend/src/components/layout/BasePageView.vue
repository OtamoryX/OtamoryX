<template>
  <div class="base-page min-h-screen w-full relative overflow-hidden">
    <!-- 统一背景层：渐变和装饰 -->
    <div class="absolute inset-0" :class="backgroundClasses">
      <!-- 装饰性网格 -->
      <div class="absolute inset-0 opacity-20">
        <div class="absolute inset-0 bg-grid-pattern"></div>
      </div>
      <!-- 浮动装饰元素 -->
      <div class="floating-elements">
        <div class="floating-element element-1"></div>
        <div class="floating-element element-2"></div>
        <div class="floating-element element-3"></div>
      </div>
    </div>
    
    <!-- 内容容器 - 完全由页面组件控制布局 -->
    <div class="relative z-0 w-full h-full min-h-screen">
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  theme?: 'default' | 'library' | 'settings'
}

const props = withDefaults(defineProps<Props>(), {
  theme: 'default'
})

// 主题配置 - 统一色调
const themes = {
  default: 'bg-gradient-to-br from-slate-900/90 via-blue-900/80 to-indigo-900/80',
  library: 'bg-gradient-to-br from-slate-900/90 via-blue-900/80 to-indigo-900/80',
  settings: 'bg-gradient-to-br from-slate-900/90 via-blue-900/80 to-indigo-900/80'
}

const backgroundClasses = computed(() => themes[props.theme])
</script>

<style scoped>
/* 背景效果 */
.base-page {
  position: relative;
  overflow: hidden;
}

/* 网格图案背景 */
.bg-grid-pattern {
  background-image: 
    linear-gradient(90deg, rgba(255, 255, 255, 0.1) 1px, transparent 1px),
    linear-gradient(rgba(255, 255, 255, 0.1) 1px, transparent 1px);
  background-size: 60px 60px;
  animation: grid-move 20s linear infinite;
}

@keyframes grid-move {
  0% { transform: translate(0, 0); }
  100% { transform: translate(60px, 60px); }
}

/* 浮动装饰元素 */
.floating-elements {
  position: absolute;
  inset: 0;
  pointer-events: none;
  overflow: hidden;
}

.floating-element {
  position: absolute;
  background: linear-gradient(135deg, rgba(255, 255, 255, 0.1), rgba(255, 255, 255, 0.05));
  border-radius: 50%;
  backdrop-filter: blur(10px);
}

.element-1 {
  width: 200px;
  height: 200px;
  top: 10%;
  left: 10%;
  animation: float-1 15s ease-in-out infinite;
}

.element-2 {
  width: 150px;
  height: 150px;
  top: 60%;
  right: 15%;
  animation: float-2 18s ease-in-out infinite reverse;
}

.element-3 {
  width: 100px;
  height: 100px;
  bottom: 20%;
  left: 70%;
  animation: float-3 12s ease-in-out infinite;
}

@keyframes float-1 {
  0%, 100% { transform: translate(0, 0) rotate(0deg); }
  25% { transform: translate(30px, -20px) rotate(90deg); }
  50% { transform: translate(-20px, -40px) rotate(180deg); }
  75% { transform: translate(-40px, 20px) rotate(270deg); }
}

@keyframes float-2 {
  0%, 100% { transform: translate(0, 0) rotate(0deg) scale(1); }
  33% { transform: translate(-25px, 30px) rotate(120deg) scale(1.1); }
  66% { transform: translate(25px, -15px) rotate(240deg) scale(0.9); }
}

@keyframes float-3 {
  0%, 100% { transform: translate(0, 0) rotate(0deg); }
  50% { transform: translate(-30px, -30px) rotate(180deg); }
}

/* 响应式调整 */
@media (max-width: 768px) {
  .floating-element {
    opacity: 0.5;
  }
  
  .element-1 { width: 120px; height: 120px; }
  .element-2 { width: 100px; height: 100px; }
  .element-3 { width: 80px; height: 80px; }
}
</style>