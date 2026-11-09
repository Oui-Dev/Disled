/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "veml7700.h"

#include "bsp/board.h"
#include "i2c_bus.h"

#define REG_ALS_CONF 0x00u
#define REG_ALS      0x04u
#define REG_ID       0x07u

#define DEVICE_ID_LSB 0x81u

/* ALS_CONF_0: ALS_GAIN x2 (bits 12:11), ALS_IT 400 ms (bits 9:6),
 * persistence 1, interrupt off. ALS_SD, bit 0, shuts the sensor down. */
#define CONF_GAIN_X2  (0x1u << 11)
#define CONF_IT_400MS (0x2u << 6)
#define CONF_ON       (CONF_GAIN_X2 | CONF_IT_400MS)
#define CONF_SHUTDOWN (CONF_ON | 0x1u)

/* veml7700_lux_milli() holds for this gain and integration time only. */
_Static_assert(VEML7700_INTEGRATION_MS == 400u, "CONF_ON sets ALS_IT to 400 ms");

static bool write_register(uint8_t reg, uint16_t value)
{
    const uint8_t frame[3] = { reg, (uint8_t)(value & 0xFFu), (uint8_t)(value >> 8) };
    return i2c_bus_write(BOARD_VEML7700_ADDR, frame, sizeof frame);
}

static bool read_register(uint8_t reg, uint16_t *value)
{
    uint8_t bytes[2];
    if (!i2c_bus_read_word(BOARD_VEML7700_ADDR, reg, bytes)) {
        return false;
    }
    *value = (uint16_t)(bytes[0] | ((uint16_t)bytes[1] << 8));
    return true;
}

bool veml7700_init(void)
{
    uint16_t id;
    if (!read_register(REG_ID, &id) || (id & 0xFFu) != DEVICE_ID_LSB) {
        return false;
    }
    return write_register(REG_ALS_CONF, CONF_ON);
}

bool veml7700_shutdown(void)
{
    return write_register(REG_ALS_CONF, CONF_SHUTDOWN);
}

bool veml7700_read(uint16_t *count)
{
    return read_register(REG_ALS, count);
}
