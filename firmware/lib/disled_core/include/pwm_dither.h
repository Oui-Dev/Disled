/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Temporal dithering of a PWM duty in sub-ticks, 1/16 of a tick: called once
 * per period, pwm_dither_next() returns the ticks of the next one, exact on
 * average over 16 periods. Header-only, for the PWM interrupt.
 */
#ifndef DISLED_PWM_DITHER_H
#define DISLED_PWM_DITHER_H

#include <stdint.h>

/* 16 periods at 20 kHz repeat at 1.25 kHz, beyond visible flicker. */
#define PWM_DITHER_SUBTICKS 16u
/* Largest period whose 100 % duty in sub-ticks fits 16 bits. */
#define PWM_DITHER_PERIOD_MAX 4095u

static inline uint16_t pwm_dither_next(uint8_t *accumulator, uint16_t subticks)
{
    const uint16_t ticks = (uint16_t)(subticks / PWM_DITHER_SUBTICKS);
    const uint8_t sum = (uint8_t)(*accumulator + (subticks % PWM_DITHER_SUBTICKS));
    if (sum >= PWM_DITHER_SUBTICKS) {
        *accumulator = (uint8_t)(sum - PWM_DITHER_SUBTICKS);
        return (uint16_t)(ticks + 1u);
    }
    *accumulator = sum;
    return ticks;
}

#endif /* DISLED_PWM_DITHER_H */
