<template>
  <div id="app" class="h-screen flex flex-col">
    <nav class="bg-white shadow-sm border-b flex-shrink-0">
      <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div class="flex justify-between h-16">
          <div class="flex items-center">
            <h1 class="text-xl font-semibold text-gray-900">OtamoryX</h1>
          </div>
          <div class="flex items-center space-x-4">
            <RouterLink 
              to="/library" 
              class="text-gray-700 hover:text-gray-900 px-3 py-2 rounded-md text-sm font-medium"
            >
              书库
            </RouterLink>
            <RouterLink 
              to="/settings" 
              class="text-gray-700 hover:text-gray-900 px-3 py-2 rounded-md text-sm font-medium"
            >
              设置
            </RouterLink>
          </div>
        </div>
      </div>
    </nav>
    
    <main class="flex-1 flex overflow-hidden">
      <RouterView />
    </main>
  </div>
</template>

<script setup lang="ts">
import { RouterLink, RouterView } from 'vue-router'
</script>

<style scoped>
.router-link-active {
  @apply text-blue-600 bg-blue-50;
}
</style>