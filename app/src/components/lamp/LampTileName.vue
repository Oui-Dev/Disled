<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The name under a color tile, on two lines at most in the width of its
  cell. A word wider than the cell takes soft hyphens; a name still too long
  ends with an ellipsis and shows whole in its title. Screen readers read the
  name as it is, without the hyphens. `narrow` takes 8 px off the box, which
  useTileNames asks for when two names would nearly touch.

  In the label of a radio of Nuxt UI, `named` is false: Reka names the radio
  with the text of its label, the shown name, and a second copy for screen
  readers would make them read it twice.
-->
<script setup lang="ts">
import { useResizeObserver } from '@vueuse/core';
import { nextTick, ref, useTemplateRef, watch } from 'vue';
import { useSettingsStore } from '@/stores/settings';
import { fitName } from '@/utils/tileName';

const props = withDefaults(defineProps<{ name: string; narrow?: boolean; named?: boolean }>(), {
  narrow: false,
  named: true,
});

const settings = useSettingsStore();
const box = useTemplateRef<HTMLElement>('box');
const shown = useTemplateRef<HTMLElement>('shown');
const text = ref(props.name);
const width = ref(0);
const clamped = ref(false);

useResizeObserver(box, ([entry]) => {
  width.value = entry?.contentRect.width ?? 0;
});

watch(
  [() => props.name, () => props.narrow, width, () => settings.locale],
  async () => {
    const name = props.name;
    if (width.value === 0) {
      text.value = name;
      return;
    }
    const fitted = await fitName(name, width.value - (props.narrow ? 8 : 0) - 0.5, settings.locale);
    // A newer name may have come meanwhile.
    if (name !== props.name) {
      return;
    }
    text.value = fitted;
    await nextTick();
    const element = shown.value;
    clamped.value = element !== null && element.scrollHeight > element.clientHeight + 1;
  },
  { immediate: true },
);
</script>

<template>
  <span ref="box" class="block w-full">
    <span
      ref="shown"
      data-tile-name
      aria-hidden="true"
      :title="clamped ? name : undefined"
      class="line-clamp-2 text-center text-[11px]/3.5 font-normal text-balance text-muted wrap-anywhere group-has-data-[state=checked]:text-highlighted"
      :class="narrow ? 'mx-1' : 'w-full'"
    >
      {{ text }}
    </span>
    <span v-if="named" class="sr-only">{{ name }}</span>
  </span>
</template>
