<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  What a window waits for, with a spinner, announced to screen readers.
-->
<script setup lang="ts">
defineProps<{ text: string }>();
</script>

<template>
  <p role="status" class="text-muted flex items-center gap-2">
    <UIcon name="i-lucide-loader-circle" class="size-4 shrink-0 motion-safe:animate-spin" />
    {{ text }}
  </p>
</template>
