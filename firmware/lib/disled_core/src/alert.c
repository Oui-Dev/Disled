/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The DISLED contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "alert.h"

#include "disled_assert.h"
#include "time_utils.h"

/* Perceived brightness range, per mille. */
#define LEVEL_MAX 1000u

/* The smoothstep runs in Q12: x * x and x2 * (3 - 2x) stay within 32 bits. */
_Static_assert(ALERT_AMPLITUDE_MAX == 4096u, "Q12 smoothstep");
_Static_assert((uint64_t)(ALERT_PERIOD_MAX_MS - 1u) * 2u * ALERT_AMPLITUDE_MAX < UINT32_MAX,
               "the triangle fits 32 bits");
_Static_assert(ALERT_PERIOD_MAX_MS < UINT32_MAX / ALERT_COUNT_MAX,
               "the length of an alert fits 32 bits");

void alert_start(alert_t *alert, uint8_t count, uint16_t period_ms, uint32_t now_ms)
{
    DISLED_ASSERT(count > 0u && period_ms > 0u);
    *alert = (alert_t){
        .running = true,
        .count = count,
        .period_ms = period_ms,
        .start_ms = now_ms,
    };
}

void alert_stop(alert_t *alert)
{
    alert->running = false;
}

bool alert_running(const alert_t *alert)
{
    return alert->running;
}

uint16_t alert_breath(uint32_t elapsed_ms, uint16_t period_ms)
{
    const uint32_t phase = elapsed_ms % period_ms;
    /* Triangle: 0 at both ends of the breath, the maximum at mid-period. */
    uint32_t x = (phase * 2u * ALERT_AMPLITUDE_MAX + period_ms / 2u) / period_ms;
    if (x > ALERT_AMPLITUDE_MAX) {
        x = 2u * ALERT_AMPLITUDE_MAX - x;
    }
    /* Smoothstep, 3x^2 - 2x^3: no edge in the light at the turns. */
    const uint32_t x2 = (x * x) / ALERT_AMPLITUDE_MAX;
    return (uint16_t)((x2 * (3u * ALERT_AMPLITUDE_MAX - 2u * x)) / ALERT_AMPLITUDE_MAX);
}

bool alert_update(alert_t *alert, uint32_t now_ms, uint16_t *amplitude)
{
    *amplitude = 0;
    if (!alert->running) {
        return false;
    }
    const uint32_t elapsed = time_elapsed_ms(now_ms, alert->start_ms);
    if (elapsed >= (uint32_t)alert->count * alert->period_ms) {
        alert->running = false;
        return true;
    }
    *amplitude = alert_breath(elapsed, alert->period_ms);
    return false;
}

uint16_t alert_level_permille(uint16_t level_permille, uint16_t depth_permille, uint16_t amplitude)
{
    const uint32_t level = (level_permille > LEVEL_MAX) ? LEVEL_MAX : level_permille;
    const uint32_t depth =
        (depth_permille > ALERT_DEPTH_MAX_PERMILLE) ? ALERT_DEPTH_MAX_PERMILLE : depth_permille;
    const uint32_t scale = (amplitude > ALERT_AMPLITUDE_MAX) ? ALERT_AMPLITUDE_MAX : amplitude;
    const uint32_t swing = (depth * scale + ALERT_AMPLITUDE_MAX / 2u) / ALERT_AMPLITUDE_MAX;
    if (level + depth <= LEVEL_MAX) {
        return (uint16_t)(level + swing);
    }
    /* Here level > LEVEL_MAX - depth >= depth >= swing. */
    return (uint16_t)(level - swing);
}
