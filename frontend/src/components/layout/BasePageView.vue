<template>
  <div class="base-page min-h-screen w-full bg-[var(--bg-secondary)]">
    <div class="relative z-0 w-full h-full min-h-screen">
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
interface Props {
  theme?: "default" | "library" | "settings";
}

withDefaults(defineProps<Props>(), {
  theme: "default",
});
</script>
