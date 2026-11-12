/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Blocking I2C master on I2C1. Every wait has a timeout, so a stuck bus costs a
 * few milliseconds, and a failed transfer clocks SCL by hand to free a slave
 * that holds SDA low, then resets the peripheral.
 */
#ifndef DISLED_I2C_BUS_H
#define DISLED_I2C_BUS_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

void i2c_bus_init(uint32_t speed_hz);

/* Writes `length` bytes to the 7-bit address. Both transfers return false
 * on a NACK, a bus error or a timeout. */
bool i2c_bus_write(uint8_t address, const uint8_t *data, size_t length);

/* Writes one register index, then reads two bytes after a repeated start: an
 * SMBus "read word", as the VEML7700 expects. */
bool i2c_bus_read_word(uint8_t address, uint8_t reg, uint8_t out[2]);

/* Failed transfers since boot (diagnostics). */
uint16_t i2c_bus_error_count(void);

#endif /* DISLED_I2C_BUS_H */
