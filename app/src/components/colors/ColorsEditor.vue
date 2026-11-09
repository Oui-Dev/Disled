<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The editor of a custom color: its name, seven suggestions, the hue and
  "Pale ↔ Vivid" sliders, the code, and the preview on the lamp. The draft is
  the code the user typed or picked, shown as it is; only the sliders make a
  new one, at full brightness. The page saves through `save`, which the
  popup footer calls.
-->
<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { colorsPreview } from '@/ipc/commands';
import { useColorsStore } from '@/stores/colors';
import { useLampStore } from '@/stores/lamp';
import { useSettingsStore } from '@/stores/settings';
import { hexOf, hueAndVividness, parseHex } from '@/utils/color';

const props = defineProps<{
  /** The color to change, or null for a new one. */
  slot: number | null;
  /** Saved from the "My colors" popover, a new color stays lit. */
  fromPopover: boolean;
}>();
const emit = defineEmits<{ done: [] }>();

const SUGGESTIONS = [
  { key: 'yellow', hex: '#F6D32D' },
  { key: 'cyan', hex: '#22D3EE' },
  { key: 'blue', hex: '#3B82F6' },
  { key: 'violet', hex: '#8B5CF6' },
  { key: 'magenta', hex: '#E83FE8' },
  { key: 'pink', hex: '#FB6FB7' },
  { key: 'coolWhite', hex: '#EEF5FF' },
] as const;

/** The preview follows the draft at most this often, as the brightness does. */
const PREVIEW_MS = 120;

const { t } = useI18n();
const colors = useColorsStore();
const lamp = useLampStore();
const settings = useSettingsStore();

const existing = colors.custom.find((color) => color.slot === props.slot);

/** The page opens with the focus in the name. */
const nameInput = useTemplateRef<{ inputRef: HTMLInputElement | null }>('nameInput');
onMounted(() => nameInput.value?.inputRef?.focus());
/** A new color starts on the first suggestion that no color of the user has. */
const start =
  existing?.hex
  ?? (
    SUGGESTIONS.find((suggestion) => !colors.custom.some((color) => color.hex === suggestion.hex))
    ?? SUGGESTIONS[0]
  ).hex;

const name = ref(existing?.name ?? '');
const hex = ref(start);
const hue = ref(hueAndVividness(start).hue ?? 210);
const vivid = ref(hueAndVividness(start).vivid);
/** What the code field shows: the typing while it has the focus, then the code in six capitals. */
const field = ref(start.slice(1));
const fieldValid = computed(() => parseHex(field.value) !== null);
const canSave = computed(() => name.value.trim().length > 0);
const suggestion = computed(
  () => SUGGESTIONS.find((candidate) => candidate.hex === hex.value)?.hex,
);

/** A code the user typed or picked: the sliders follow it, a grey keeps the hue. */
function take(code: string): void {
  hex.value = code;
  const shade = hueAndVividness(code);
  if (shade.hue !== null) {
    hue.value = shade.hue;
  }
  vivid.value = shade.vivid;
}

function typeCode(text: string): void {
  field.value = text;
  const code = parseHex(text);
  if (code) {
    take(code);
  }
}

function pick(code: unknown): void {
  if (typeof code === 'string') {
    take(code);
    field.value = code.slice(1);
  }
}

/** The only moment the code changes by itself: under a slider. */
function slide(kind: 'hue' | 'vivid', value: number | number[] | undefined): void {
  if (typeof value !== 'number') {
    return;
  }
  if (kind === 'hue') {
    hue.value = value;
  } else {
    vivid.value = value;
  }
  hex.value = hexOf(hue.value, vivid.value);
  field.value = hex.value.slice(1);
}

// The preview on the lamp: with the setting on and a lamp the app drives.
const previewing = computed(() => (settings.settings?.color_preview ?? true) && lamp.ready);
let lastSent = 0;
let pending: ReturnType<typeof setTimeout> | undefined;

function preview(code: string | null): void {
  clearTimeout(pending);
  pending = undefined;
  if (code === null) {
    void colorsPreview(null);
    return;
  }
  const wait = lastSent + PREVIEW_MS - Date.now();
  if (wait > 0) {
    pending = setTimeout(() => preview(code), wait);
    return;
  }
  lastSent = Date.now();
  void colorsPreview(code);
}

watch(hex, (code) => {
  if (previewing.value) {
    preview(code);
  }
});
watch(previewing, (on) => preview(on ? hex.value : null));
onMounted(() => {
  if (previewing.value) {
    preview(hex.value);
  }
});
// Cancel, Escape and a hidden popup all leave the page: the lamp goes back
// to its color. After a save, the backend has ended the preview already.
onBeforeUnmount(() => preview(null));

function setPreview(on: boolean): void {
  void settings.update({ color_preview: on });
}

async function save(): Promise<void> {
  if (!canSave.value) {
    return;
  }
  clearTimeout(pending);
  const slot = await colors.saveCustom({
    slot: props.slot,
    name: name.value,
    hex: hex.value,
    keepLit: props.fromPopover,
  });
  if (slot !== undefined) {
    emit('done');
  }
}

defineExpose({ save, canSave });
</script>

<template>
  <div class="flex flex-col gap-4">
    <h2
      data-page-heading
      tabindex="-1"
      class="text-sm font-semibold text-highlighted focus:outline-none"
    >
      {{ slot === null ? t('colors.editor.new') : t('colors.editor.edit') }}
    </h2>

    <div class="flex items-center gap-3">
      <span
        aria-hidden="true"
        class="size-8 shrink-0 rounded-[20%] border border-accented"
        :style="{ background: hex }"
      />
      <UInput
        ref="nameInput"
        v-model="name"
        maxlength="16"
        :aria-label="t('colors.editor.name')"
        :placeholder="t('colors.editor.name')"
        class="flex-1"
        @keydown.enter="save"
      />
    </div>

    <URadioGroup
      :key="suggestion ?? 'none'"
      :model-value="suggestion"
      :items="
        SUGGESTIONS.map((item) => ({
          value: item.hex,
          label: t(`colors.editor.suggestion.${item.key}`),
        }))
      "
      :legend="t('colors.editor.suggestions')"
      orientation="horizontal"
      indicator="hidden"
      :ui="{
        legend: 'sr-only',
        fieldset: 'flex-nowrap justify-between gap-x-0',
        item: 'group p-0',
        label: 'cursor-pointer',
      }"
      @update:model-value="pick($event)"
    >
      <template #label="{ item }">
        <span
          :title="item.label"
          class="block size-6 rounded-full border border-accented ring-2 ring-transparent ring-offset-2 ring-offset-bg transition-shadow group-has-data-[state=checked]:ring-inverted"
          :style="{ background: String(item.value) }"
        />
        <span class="sr-only">{{ item.label }}</span>
      </template>
    </URadioGroup>

    <USlider
      :model-value="hue"
      :min="0"
      :max="359"
      :step="1"
      :aria-label="t('colors.editor.hue')"
      :aria-valuetext="`${hue}°`"
      :style="{ '--draft': hex }"
      :ui="{ track: 'hue-track', range: 'bg-transparent', thumb: 'draft-thumb' }"
      @update:model-value="slide('hue', $event)"
    />

    <div class="flex items-center gap-2.5">
      <span class="shrink-0 text-xs text-muted">{{ t('colors.editor.pale') }}</span>
      <USlider
        :model-value="vivid"
        :min="0"
        :max="100"
        :step="1"
        :aria-label="t('colors.editor.vividLabel')"
        :aria-valuetext="`${vivid} %`"
        :style="{ '--draft': hex, '--pure': hexOf(hue, 100) }"
        :ui="{ track: 'vivid-track', range: 'bg-transparent', thumb: 'draft-thumb' }"
        class="flex-1"
        @update:model-value="slide('vivid', $event)"
      />
      <span class="shrink-0 text-xs text-muted">{{ t('colors.editor.vivid') }}</span>
    </div>

    <div class="flex items-center justify-between gap-3">
      <label for="color-code" class="text-sm text-default">{{ t('colors.editor.hex') }}</label>
      <UInput
        id="color-code"
        :model-value="field"
        maxlength="7"
        :color="fieldValid ? 'neutral' : 'error'"
        :highlight="!fieldValid"
        :aria-invalid="!fieldValid"
        class="w-32 font-mono"
        @update:model-value="typeCode(String($event))"
        @blur="field = hex.slice(1)"
      >
        <template #leading>
          <span class="text-sm text-muted">#</span>
        </template>
      </UInput>
    </div>

    <USwitch
      :model-value="settings.settings?.color_preview ?? true"
      :label="t('colors.editor.preview')"
      size="sm"
      @update:model-value="setPreview"
    />
  </div>
</template>

<style scoped>
:deep(.hue-track) {
  background: linear-gradient(to right, #f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00);
}

:deep(.vivid-track) {
  background: linear-gradient(to right, #fff, var(--pure));
}

:deep(.draft-thumb) {
  background: var(--draft);
}
</style>
