<template>
  <div
    class="loading-placeholder w-full h-full flex justify-center items-center relative overflow-hidden pointer-events-none"
  >
    <!-- 动态背景粒子 -->
    <div class="absolute inset-0 opacity-40">
      <div class="loading-particles absolute inset-0" />
    </div>

    <!-- 立体书本翻页动画容器 -->
    <div
      class="relative z-0 flex flex-col justify-center items-center pointer-events-none"
    >
      <div class="relative perspective-1000">
        <!-- 书本底座 -->
        <div
          class="w-28 h-36 bg-linear-to-br from-gray-800 via-gray-850 to-gray-900 rounded-lg shadow-2xl relative transform-gpu"
        >
          <!-- 书脊边缘高光 -->
          <div
            class="absolute left-0 top-2 bottom-2 w-px bg-linear-to-b from-gray-500 to-gray-700 opacity-80 rounded-full"
          />
          <!-- 书脊中线 -->
          <div
            class="absolute left-1.5 top-3 bottom-3 w-0.5 bg-linear-to-b from-gray-600 to-gray-700 rounded-full"
          />
          <!-- 装饰线条 -->
          <div
            class="absolute left-0.5 top-4 bottom-4 w-px bg-linear-to-b from-gray-500 to-gray-600 opacity-50"
          />

          <!-- 静止的底层页面 -->
          <div
            class="absolute inset-0.5 bg-linear-to-br from-white to-gray-100 rounded-md border border-gray-200 shadow-inner"
          >
            <div class="absolute inset-2 bg-white rounded-sm">
              <!-- 底层页面的模拟文字 -->
              <div
                class="absolute top-2 left-1.5 right-1.5 h-0.5 bg-gray-300 opacity-60 rounded"
              />
              <div
                class="absolute top-3 left-1.5 right-2 h-0.5 bg-gray-300 opacity-50 rounded"
              />
              <div
                class="absolute top-4 left-1.5 right-1.5 h-0.5 bg-gray-300 opacity-45 rounded"
              />
              <div
                class="absolute top-5 left-1.5 right-2.5 h-0.5 bg-gray-300 opacity-40 rounded"
              />
            </div>
          </div>

          <!-- 翻页动画 - 第一页（带角度） -->
          <div
            class="book-page-angled absolute inset-0.5 bg-linear-to-r from-white via-gray-50 to-gray-100 rounded-md border border-gray-200 transform-gpu"
          >
            <div
              class="absolute inset-2 bg-linear-to-br from-white to-gray-50 rounded-sm"
            >
              <!-- 模拟文字行 -->
              <div
                class="absolute top-1.5 left-1.5 right-1 h-0.5 bg-gray-500 opacity-50 rounded"
              />
              <div
                class="absolute top-2.5 left-1.5 right-1.5 h-0.5 bg-gray-500 opacity-45 rounded"
              />
              <div
                class="absolute top-3.5 left-1.5 right-2 h-0.5 bg-gray-500 opacity-40 rounded"
              />
              <div
                class="absolute top-4.5 left-1.5 right-1.5 h-0.5 bg-gray-500 opacity-35 rounded"
              />
              <div
                class="absolute top-5.5 left-1.5 right-2.5 h-0.5 bg-gray-500 opacity-30 rounded"
              />
              <div
                class="absolute top-6.5 left-1.5 right-1.5 h-0.5 bg-gray-500 opacity-25 rounded"
              />
              <!-- 页面折痕和阴影 -->
              <div
                class="absolute top-0 left-0 bottom-0 w-px bg-linear-to-b from-transparent via-gray-400 to-transparent opacity-40"
              />
            </div>
            <!-- 翻页时的动态阴影 -->
            <div
              class="page-shadow absolute inset-0 bg-linear-to-l from-black/20 via-transparent to-transparent rounded-md opacity-0"
            />
          </div>

          <!-- 翻页动画 - 第二页（带角度） -->
          <div
            class="book-page-angled-2 absolute inset-0.5 bg-linear-to-r from-blue-50 via-blue-25 to-white rounded-md border border-blue-200 transform-gpu"
          >
            <div
              class="absolute inset-2 bg-linear-to-br from-blue-50 to-white rounded-sm"
            >
              <!-- 模拟文字行 - 蓝色主题 -->
              <div
                class="absolute top-1.5 left-1.5 right-1 h-0.5 bg-blue-600 opacity-45 rounded"
              />
              <div
                class="absolute top-2.5 left-1.5 right-1.5 h-0.5 bg-blue-600 opacity-40 rounded"
              />
              <div
                class="absolute top-3.5 left-1.5 right-2 h-0.5 bg-blue-600 opacity-35 rounded"
              />
              <div
                class="absolute top-4.5 left-1.5 right-1.5 h-0.5 bg-blue-600 opacity-30 rounded"
              />
              <div
                class="absolute top-5.5 left-1.5 right-2.5 h-0.5 bg-blue-600 opacity-25 rounded"
              />
              <div
                class="absolute top-6.5 left-1.5 right-1.5 h-0.5 bg-blue-600 opacity-20 rounded"
              />
              <!-- 页面折痕和阴影 -->
              <div
                class="absolute top-0 left-0 bottom-0 w-px bg-linear-to-b from-transparent via-blue-400 to-transparent opacity-50"
              />
            </div>
            <!-- 翻页时的动态阴影 -->
            <div
              class="page-shadow-2 absolute inset-0 bg-linear-to-l from-black/25 via-transparent to-transparent rounded-md opacity-0"
            />
          </div>
        </div>

        <!-- 发光光环效果 -->
        <div
          class="absolute -inset-8 bg-gradient-radial from-blue-400/25 via-blue-400/10 to-transparent rounded-full blur-xl animate-pulse"
        />
        <!-- 内层光环 -->
        <div
          class="absolute -inset-4 bg-gradient-radial from-white/15 via-transparent to-transparent rounded-full blur-lg"
        />
      </div>

      <!-- 可选的加载文字 -->
      <div
        v-if="showText"
        class="mt-6 text-white/70 text-sm font-medium tracking-wide animate-pulse"
      >
        {{ text }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
interface Props {
  text?: string;
  showText?: boolean;
}

withDefaults(defineProps<Props>(), {
  text: "正在加载...",
  showText: false,
});
</script>

<style scoped>
/* 背景粒子动画 */
.loading-particles::before,
.loading-particles::after {
  content: "";
  position: absolute;
  width: 4px;
  height: 4px;
  background: radial-gradient(
    circle,
    rgba(59, 130, 246, 0.8) 0%,
    transparent 70%
  );
  border-radius: 50%;
  animation: float-particles 4s ease-in-out infinite;
}

.loading-particles::before {
  top: 20%;
  left: 15%;
  animation-delay: -2s;
}

.loading-particles::after {
  top: 60%;
  right: 20%;
  animation-delay: -1s;
}

@keyframes float-particles {
  0%,
  100% {
    transform: translateY(0px) scale(1);
    opacity: 0.3;
  }
  50% {
    transform: translateY(-20px) scale(1.2);
    opacity: 0.8;
  }
}

/* 立体翻页动画 - 增强版 */
.perspective-1000 {
  perspective: 1000px;
}

.book-page-angled {
  animation: angled-page-flip 1.2s ease-in-out infinite;
  transform-origin: left center;
  transform-style: preserve-3d;
  backface-visibility: hidden;
}

.book-page-angled-2 {
  animation: angled-page-flip-2 1.2s ease-in-out infinite;
  animation-delay: -0.6s;
  transform-origin: left center;
  transform-style: preserve-3d;
  backface-visibility: hidden;
}

/* 第一页的翻页动画 - 带角度 */
@keyframes angled-page-flip {
  0% {
    transform: rotateY(0deg) rotateX(0deg) rotateZ(0deg);
    opacity: 1;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  }
  20% {
    transform: rotateY(-25deg) rotateX(-8deg) rotateZ(2deg);
    opacity: 1;
    box-shadow: 4px 8px 20px rgba(0, 0, 0, 0.3);
  }
  40% {
    transform: rotateY(-110deg) rotateX(-15deg) rotateZ(5deg);
    opacity: 0.7;
    box-shadow: 8px 12px 24px rgba(0, 0, 0, 0.4);
  }
  60% {
    transform: rotateY(-160deg) rotateX(-12deg) rotateZ(3deg);
    opacity: 0.2;
    box-shadow: 6px 10px 22px rgba(0, 0, 0, 0.35);
  }
  80%,
  100% {
    transform: rotateY(-180deg) rotateX(0deg) rotateZ(0deg);
    opacity: 0;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  }
}

/* 第二页的翻页动画 - 带角度 */
@keyframes angled-page-flip-2 {
  0% {
    transform: rotateY(-180deg) rotateX(0deg) rotateZ(0deg);
    opacity: 0;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  }
  20% {
    transform: rotateY(-155deg) rotateX(-8deg) rotateZ(-2deg);
    opacity: 0.2;
    box-shadow: 4px 8px 20px rgba(0, 0, 0, 0.3);
  }
  40% {
    transform: rotateY(-70deg) rotateX(-15deg) rotateZ(-5deg);
    opacity: 0.7;
    box-shadow: 8px 12px 24px rgba(0, 0, 0, 0.4);
  }
  60% {
    transform: rotateY(-20deg) rotateX(-12deg) rotateZ(-3deg);
    opacity: 1;
    box-shadow: 6px 10px 22px rgba(0, 0, 0, 0.35);
  }
  80%,
  100% {
    transform: rotateY(0deg) rotateX(0deg) rotateZ(0deg);
    opacity: 1;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  }
}

/* 动态阴影效果 */
.book-page-angled .page-shadow {
  animation: shadow-fade 1.2s ease-in-out infinite;
}

.book-page-angled-2 .page-shadow-2 {
  animation: shadow-fade-2 1.2s ease-in-out infinite;
  animation-delay: -0.6s;
}

@keyframes shadow-fade {
  0%,
  100% {
    opacity: 0;
  }
  20% {
    opacity: 0.2;
  }
  40% {
    opacity: 0.6;
  }
  60% {
    opacity: 0.3;
  }
  80% {
    opacity: 0;
  }
}

@keyframes shadow-fade-2 {
  0%,
  100% {
    opacity: 0;
  }
  20% {
    opacity: 0.3;
  }
  40% {
    opacity: 0.7;
  }
  60% {
    opacity: 0.2;
  }
  80% {
    opacity: 0;
  }
}

/* 径向渐变支持 */
.bg-gradient-radial {
  background-image: radial-gradient(var(--tw-gradient-stops));
}

/* 额外的粒子效果 */
.loading-particles {
  background-image:
    radial-gradient(2px 2px at 20px 30px, rgba(59, 130, 246, 0.3), transparent),
    radial-gradient(
      2px 2px at 40px 70px,
      rgba(147, 197, 253, 0.4),
      transparent
    ),
    radial-gradient(1px 1px at 90px 40px, rgba(59, 130, 246, 0.5), transparent),
    radial-gradient(
      1px 1px at 130px 80px,
      rgba(147, 197, 253, 0.3),
      transparent
    ),
    radial-gradient(2px 2px at 160px 30px, rgba(59, 130, 246, 0.4), transparent);
  background-repeat: repeat;
  background-size: 200px 100px;
  animation: particles-drift 20s linear infinite;
}

@keyframes particles-drift {
  0% {
    background-position:
      0 0,
      0 0,
      0 0,
      0 0,
      0 0;
  }
  100% {
    background-position:
      -200px 0,
      -200px 0,
      -200px 0,
      -200px 0,
      -200px 0;
  }
}
</style>
