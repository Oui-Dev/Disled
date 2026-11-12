// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

import type { Telemetry } from '@/bindings/Telemetry';

/** A line of a telemetry chart. */
export interface ChartSeries {
  key: string;
  label: string;
  /** A theme color or any CSS color. */
  color: string;
  value: (sample: Telemetry) => number;
}

export type ChartRow = Record<string, number | string>;

/**
 * The lamp's uptime as h:mm:ss.mmm, the x value of the charts. Each sample
 * gets its own, since the chart merges the samples of a same value.
 */
function formatUptime(ms: number): string {
  const seconds = Math.floor(ms / 1000) % 60;
  const minutes = Math.floor(ms / 60_000) % 60;
  const hours = Math.floor(ms / 3_600_000);
  const pad = (value: number, digits = 2) => String(value).padStart(digits, '0');
  return `${hours}:${pad(minutes)}:${pad(seconds)}.${pad(ms % 1000, 3)}`;
}

/** The uptime of each sample, formatted once for every chart and refresh. */
const uptimes = new WeakMap<Telemetry, string>();

function uptimeOf(sample: Telemetry): string {
  let uptime = uptimes.get(sample);
  if (uptime === undefined) {
    uptime = formatUptime(sample.uptime_ms);
    uptimes.set(sample, uptime);
  }
  return uptime;
}

/** One row per sample: its uptime, then the value of each series. */
export function chartRows(
  samples: readonly Telemetry[],
  series: readonly ChartSeries[],
): ChartRow[] {
  return samples.map((sample) => {
    const row: ChartRow = { uptime: uptimeOf(sample) };
    for (const { key, value } of series) {
      row[key] = value(sample);
    }
    return row;
  });
}
