// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

import type { Diagnostics } from '@/bindings/Diagnostics';

/** What needs a look in the lamp diagnostics, one flag per row of the Health tab. */
interface HealthIssues {
  watchdogResets: boolean;
  faults: boolean;
  lightSensor: boolean;
  touchSamplesLost: boolean;
  crystal: boolean;
}

export function healthIssues(d: Diagnostics): HealthIssues {
  return {
    watchdogResets: d.watchdog_resets > 0,
    faults: d.faults > 0,
    lightSensor: !d.light_sensor_present || d.i2c_errors > 0,
    touchSamplesLost: d.touch_samples_lost > 0,
    crystal: !d.crystal_running,
  };
}

/** How many rows of the Health tab need a look. */
export function issueCount(d: Diagnostics): number {
  return Object.values(healthIssues(d)).filter(Boolean).length;
}
