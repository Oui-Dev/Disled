/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Vishay VEML7700 ambient light sensor at I2C address 0x10, at one range:
 * gain 2 and 400 ms, 0.0084 lx a count (datasheet rev. 1.8) up to 550 lx.
 * The brightness curve ends at 100 lx, where gain 2 is still linear. The
 * sensor integrates continuously, so one read per integration time gives a
 * fresh count.
 */
#ifndef DISLED_VEML7700_H
#define DISLED_VEML7700_H

#include <stdbool.h>
#include <stdint.h>

/* Set in ALS_CONF_0 by veml7700.c, which checks that the two agree. */
#define VEML7700_INTEGRATION_MS 400u

/* Checks the device ID and powers the sensor on. False if it does not
 * answer or is not a VEML7700. */
bool veml7700_init(void);

/* Reads the ALS count. False on a bus error. */
bool veml7700_read(uint16_t *count);

/* Puts the sensor to sleep (about 0.5 uA); veml7700_init() wakes it. */
bool veml7700_shutdown(void);

/* Light of a count, in lux x 1000. */
static inline uint32_t veml7700_lux_milli(uint16_t count)
{
    return ((uint32_t)count * 84u + 5u) / 10u;
}

#endif /* DISLED_VEML7700_H */
