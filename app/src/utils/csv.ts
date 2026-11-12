// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// Telemetry recordings as CSV, one row per sample, for the tuning campaigns
// of docs/bringup.md. Tools read the columns, so new ones go at the end.

import type { LampState } from '@/bindings/LampState';
import type { Telemetry } from '@/bindings/Telemetry';

const HEADER = [
  'uptime_ms',
  'touch_raw',
  'touch_value',
  'touch_baseline',
  'touch_delta_permille',
  'touch_state',
  'lux',
  'als_raw',
  'duty_r',
  'duty_g',
  'duty_b',
  'led_ma',
  'cap_ma',
  'link',
  'level_permille',
  'lamp_on',
  'color',
  'brightness_mode',
  'applied_level',
].join(',');

/** One sample, with the lamp state it was taken in, empty when unknown. */
export function telemetryRow(t: Telemetry, state: LampState | null): string {
  return [
    t.uptime_ms,
    t.touch_raw,
    t.touch_value,
    t.touch_baseline,
    t.touch_delta_permille,
    t.touch_state,
    (t.lux_centi / 100).toFixed(2),
    t.als_raw,
    ...t.duty,
    (t.led_current_ma_x10 / 10).toFixed(1),
    t.cap_ma,
    t.link,
    t.level_permille,
    state ? Number(state.on) : '',
    state?.color ?? '',
    state?.brightness_mode ?? '',
    state?.applied_level ?? '',
  ].join(',');
}

/** The file: the header, the rows, and a newline at the end. */
export function csvFile(rows: readonly string[]): string {
  return [HEADER, ...rows, ''].join('\n');
}
