// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// Telemetry recordings as CSV, one row per sample, for the tuning campaigns
// of docs/bringup.md. Tools read the columns, so new ones go at the end.

import type { LampState } from '@/bindings/LampState';
import type { Telemetry } from '@/bindings/Telemetry';

/** The cell of a column for a sample, taken in a lamp state. */
type Cell = (t: Telemetry, state: LampState | null) => string | number;

/** The columns by name, in order. The cells of an unknown lamp state stay empty. */
const COLUMNS: [name: string, cell: Cell][] = [
  ['uptime_ms', (t) => t.uptime_ms],
  ['touch_raw', (t) => t.touch_raw],
  ['touch_value', (t) => t.touch_value],
  ['touch_baseline', (t) => t.touch_baseline],
  ['touch_delta_permille', (t) => t.touch_delta_permille],
  ['touch_state', (t) => t.touch_state],
  ['lux', (t) => (t.lux_centi / 100).toFixed(2)],
  ['als_raw', (t) => t.als_raw],
  ['duty_r', (t) => t.duty[0]],
  ['duty_g', (t) => t.duty[1]],
  ['duty_b', (t) => t.duty[2]],
  ['led_ma', (t) => (t.led_current_ma_x10 / 10).toFixed(1)],
  ['cap_ma', (t) => t.cap_ma],
  ['link', (t) => t.link],
  ['level_permille', (t) => t.level_permille],
  ['lamp_on', (_, state) => (state ? Number(state.on) : '')],
  ['color', (_, state) => state?.color ?? ''],
  ['brightness_mode', (_, state) => state?.brightness_mode ?? ''],
  ['applied_level', (_, state) => state?.applied_level ?? ''],
];

const HEADER = COLUMNS.map(([name]) => name).join(',');

/** One sample, with the lamp state it was taken in. */
export function telemetryRow(t: Telemetry, state: LampState | null): string {
  return COLUMNS.map(([, cell]) => cell(t, state)).join(',');
}

/** The file: the header, the rows, and a newline at the end. */
export function csvFile(rows: readonly string[]): string {
  return [HEADER, ...rows, ''].join('\n');
}
