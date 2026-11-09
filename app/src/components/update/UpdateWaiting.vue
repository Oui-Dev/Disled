<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  A step the update waits for, announced to screen readers.
-->
<script setup lang="ts">
defineProps<{ text: string }>();
</script>

<template>
  <p role="status" class="flex items-center gap-2 text-muted">
    <UIcon name="i-lucide-loader-circle" class="size-4 shrink-0 motion-safe:animate-spin" />
    {{ text }}
  </p>
</template>
