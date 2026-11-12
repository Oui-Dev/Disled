/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * 96-bit unique ID of the chip, used as the USB serial number.
 */
#ifndef DISLED_BSP_UID_H
#define DISLED_BSP_UID_H

#include <stddef.h>
#include <stdint.h>

#define BSP_UID_BYTES 12u
/* 24 hexadecimal digits, without terminator. */
#define BSP_UID_HEX_CHARS (2u * BSP_UID_BYTES)

/* Copies the unique ID, most significant byte first. */
void bsp_uid_read(uint8_t uid[BSP_UID_BYTES]);

/* Writes the unique ID as upper-case hexadecimal, without terminator. */
void bsp_uid_hex(char out[BSP_UID_HEX_CHARS]);

#endif /* DISLED_BSP_UID_H */
