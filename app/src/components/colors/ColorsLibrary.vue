<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The colors page: the four built-in colors, then the custom ones, each
  with its badge in the touch cycle and its menu; then the cycle in words,
  what the lamp keeps without the app, and the link toward sharing. Every
  change goes at once to the backend, which tells the lamp.
-->
<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui';
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { type ShownColor, useColorsStore } from '@/stores/colors';

const emit = defineEmits<{ edit: [slot: number]; create: []; share: [] }>();

const { t, locale } = useI18n();
const colors = useColorsStore();

/** Read out politely by screen readers: a change of the cycle or a deletion. */
const announcement = ref('');

/** "1re", "2e" in French, "1st", "2nd", "3rd" in English. */
function rank(position: number): string {
  const category = new Intl.PluralRules(locale.value, { type: 'ordinal' }).select(position);
  const key = (['one', 'two', 'few'] as const).find((known) => known === category) ?? 'other';
  return t(`colors.cycle.rank.${key}`, { n: position });
}

/** The place of a color in the cycle, from 1, or 0 outside it. */
function place(slot: number): number {
  return colors.cycle.indexOf(slot) + 1;
}

async function toggle(color: ShownColor): Promise<void> {
  const cycle = colors.cycle;
  const at = cycle.indexOf(color.slot);
  if (at < 0) {
    await colors.setCycle([...cycle, color.slot]);
    announcement.value = t('colors.cycle.added', {
      name: color.name,
      rank: rank(cycle.length + 1),
    });
  } else if (cycle.length === 1) {
    // The cycle keeps at least one color.
    announcement.value = t('colors.cycle.last', { name: color.name });
  } else {
    await colors.setCycle(cycle.filter((slot) => slot !== color.slot));
    announcement.value = t('colors.cycle.removed', { name: color.name });
  }
}

function menu(color: ShownColor): DropdownMenuItem[] {
  const items: DropdownMenuItem[] = [
    {
      label: t('colors.library.edit'),
      icon: 'i-lucide-pencil',
      onSelect: () => emit('edit', color.slot),
    },
  ];
  if (!color.builtIn) {
    items.push({
      label: t('colors.library.delete'),
      icon: 'i-lucide-trash',
      color: 'error',
      onSelect: async () => {
        await colors.remove(color.slot);
        announcement.value = t('colors.library.deleted', { name: color.name });
      },
    });
  }
  return items;
}

/** The cycle in words; a name never breaks inside. */
const cycleText = computed(() =>
  colors.cycle.map((slot) => colors.color(slot)?.name.replaceAll(' ', '\u00a0') ?? '').join(' › '),
);

const groups = computed(() => [colors.builtIn, colors.custom].filter((group) => group.length));
</script>

<template>
  <div class="flex flex-col gap-3">
    <!-- The column header stands over the cycle badges, left of the menus. -->
    <div class="flex items-center justify-between gap-2">
      <h2
        data-page-heading
        tabindex="-1"
        class="text-sm font-semibold text-highlighted focus:outline-none"
      >
        {{ t('colors.library.title') }}
      </h2>
      <span aria-hidden="true" class="me-[34px] w-6 text-center text-[11px] text-muted">
        {{ t('colors.library.touch') }}
      </span>
    </div>

    <div class="flex flex-col">
      <template v-for="(group, at) in groups" :key="at">
        <USeparator v-if="at > 0" class="my-1.5" />
        <ul class="flex flex-col">
          <li v-for="color in group" :key="color.slot" class="flex h-8 items-center gap-2.5">
            <span
              aria-hidden="true"
              class="size-5 shrink-0 rounded-[20%] border border-accented"
              :style="{ background: color.hex }"
            />
            <span class="min-w-0 flex-1 truncate text-sm text-default" :title="color.name">
              {{ color.name }}
            </span>
            <!-- Filled with its rank in the cycle, dashed outside it. -->
            <UButton
              color="neutral"
              :variant="place(color.slot) ? 'solid' : 'ghost'"
              class="size-6 shrink-0 justify-center rounded-full p-0 text-[11px] font-semibold tabular-nums"
              :class="{ 'border border-dashed border-accented text-muted': !place(color.slot) }"
              :aria-pressed="place(color.slot) > 0"
              :title="
                place(color.slot) ? t('colors.cycle.removeTitle') : t('colors.cycle.addTitle')
              "
              :aria-label="
                place(color.slot)
                  ? t('colors.cycle.remove', { name: color.name, rank: rank(place(color.slot)) })
                  : t('colors.cycle.add', { name: color.name })
              "
              @click="toggle(color)"
            >
              <span v-if="place(color.slot)" aria-hidden="true">{{ place(color.slot) }}</span>
            </UButton>
            <UDropdownMenu
              :items="menu(color)"
              :modal="false"
              :content="{ align: 'end' }"
              size="sm"
            >
              <UButton
                color="neutral"
                variant="ghost"
                size="xs"
                icon="i-lucide-ellipsis-vertical"
                :aria-label="t('colors.library.more', { name: color.name })"
              />
            </UDropdownMenu>
          </li>
        </ul>
      </template>
      <UButton
        v-if="!colors.full"
        color="neutral"
        variant="link"
        class="h-8 gap-2.5 p-0 text-start text-sm font-normal"
        @click="emit('create')"
      >
        <span
          aria-hidden="true"
          class="flex size-5 shrink-0 items-center justify-center rounded-[20%] border border-dashed border-accented"
        >
          <UIcon name="i-lucide-plus" class="size-3.5" />
        </span>
        {{ t('colors.library.new') }}
      </UButton>
    </div>

    <p class="flex items-start gap-2 text-xs text-default">
      <UIcon name="i-lucide-repeat" class="mt-0.5 size-3.5 shrink-0 text-muted" />
      <span>
        <span class="sr-only">{{ t('colors.library.cycle') }}</span>
        {{ cycleText }}
      </span>
    </p>
    <p class="flex items-start gap-2 text-xs text-muted">
      <UIcon name="i-lucide-info" class="mt-0.5 size-3.5 shrink-0" />
      <span>{{ t('colors.library.appOnly') }}</span>
    </p>

    <UButton
      color="neutral"
      variant="link"
      icon="i-lucide-share-2"
      :label="t('colors.library.share')"
      trailing-icon="i-lucide-chevron-right"
      class="flex w-full gap-2 px-0 py-1 text-start text-sm font-normal text-default"
      :ui="{ leadingIcon: 'size-4 text-muted', label: 'flex-1', trailingIcon: 'size-4 text-muted' }"
      @click="emit('share')"
    />

    <p class="sr-only" aria-live="polite">{{ announcement }}</p>
  </div>
</template>
