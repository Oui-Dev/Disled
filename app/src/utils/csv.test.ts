// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

import { describe, expect, it } from 'vitest';
import type { LampState } from '@/bindings/LampState';
import type { Telemetry } from '@/bindings/Telemetry';
import { csvFile, telemetryRow } from './csv';

const sample: Telemetry = {
  uptime_ms: 123456,
  touch_raw: 1500,
  touch_value: 1498,
  touch_baseline: 1510,
  touch_delta_permille: -8,
  touch_state: 'idle',
  lux_centi: 12345,
  als_raw: 321,
  duty: [10, 20, 30],
  led_current_ma_x10: 1234,
  cap_ma: 300,
  link: 'configured',
  level_permille: 456,
};

const state: LampState = {
  state_seq: 1,
  on: true,
  color: 2,
  brightness_mode: 'auto',
  manual_level: 50,
  applied_level: 37,
  link: 'configured',
  source: 'touch',
  touch_mode: 'normal',
  rgb: { r: 1, g: 2, b: 3 },
};

describe('the telemetry recording', () => {
  // The tools of the tuning campaigns read these columns by name.
  it('keeps its columns and their formats', () => {
    expect(csvFile([telemetryRow(sample, state)])).toBe(
      'uptime_ms,touch_raw,touch_value,touch_baseline,touch_delta_permille,touch_state,lux,'
        + 'als_raw,duty_r,duty_g,duty_b,led_ma,cap_ma,link,level_permille,lamp_on,color,'
        + 'brightness_mode,applied_level\n'
        + '123456,1500,1498,1510,-8,idle,123.45,321,10,20,30,123.4,300,configured,456,1,2,auto,37\n',
    );
  });

  it('leaves the cells of the lamp state empty while it is unknown', () => {
    expect(telemetryRow(sample, null)).toBe(
      '123456,1500,1498,1510,-8,idle,123.45,321,10,20,30,123.4,300,configured,456,,,,',
    );
  });
});
