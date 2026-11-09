/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Alert: a few soft breaths over the lamp color, which then ends by itself.
 * A breath moves the perceived brightness along a smoothstep, up from a dim
 * lamp and down from a bright one; led_output applies it after the fade.
 */
#ifndef DISLED_ALERT_H
#define DISLED_ALERT_H

#include <stdbool.h>
#include <stdint.h>

/* Breath amplitude: 0 is the lamp level, ALERT_AMPLITUDE_MAX the full swing. */
#define ALERT_AMPLITUDE_MAX 4096u

/* Limits of the ALERT command. */
#define ALERT_COUNT_MAX     10u
#define ALERT_PERIOD_MIN_MS 500u
#define ALERT_PERIOD_MAX_MS 5000u

/* Largest swing: half the range, so that it fits either way. */
#define ALERT_DEPTH_MAX_PERMILLE 500u

typedef struct {
    bool running;
    uint8_t count;      /* breaths */
    uint16_t period_ms; /* one breath, in and out */
    uint32_t start_ms;
} alert_t;

/* Starts, or restarts, `count` breaths (1 or more) of `period_ms` (not 0). */
void alert_start(alert_t *alert, uint8_t count, uint16_t period_ms, uint32_t now_ms);

void alert_stop(alert_t *alert);

bool alert_running(const alert_t *alert);

/* Breath amplitude at `now_ms`, 0 when stopped. After the last breath the
 * alert stops by itself: it then returns true, once. */
bool alert_update(alert_t *alert, uint32_t now_ms, uint16_t *amplitude);

/* One breath of `period_ms`, at `elapsed_ms` (modulo the period): a smoothstep
 * up to ALERT_AMPLITUDE_MAX at mid-period and back down, flat at both ends. */
uint16_t alert_breath(uint32_t elapsed_ms, uint16_t period_ms);

/* Perceived level (per mille) at a breath amplitude. The swing of
 * `depth_permille` goes up from a level with that much room above it, and
 * down otherwise. */
uint16_t alert_level_permille(uint16_t level_permille, uint16_t depth_permille, uint16_t amplitude);

#endif /* DISLED_ALERT_H */
