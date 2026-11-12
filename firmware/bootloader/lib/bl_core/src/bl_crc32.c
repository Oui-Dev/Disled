/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * CRC-32 of zlib. BL_CRC32_TABLE=1 uses a 16-entry table (64 bytes of
 * flash), about three times faster than the bit loop: the check of the
 * application at every reset runs on the 8 MHz internal oscillator.
 */
#include "bl_core.h"

#ifndef BL_CRC32_TABLE
#define BL_CRC32_TABLE 1
#endif

#define CRC32_POLY 0xEDB88320u

#if BL_CRC32_TABLE

/* table[i]: CRC of the 4-bit value i (four steps of the bit loop). */
static const uint32_t crc32_nibble[16] = {
    0x00000000u, 0x1DB71064u, 0x3B6E20C8u, 0x26D930ACu, 0x76DC4190u, 0x6B6B51F4u,
    0x4DB26158u, 0x5005713Cu, 0xEDB88320u, 0xF00F9344u, 0xD6D6A3E8u, 0xCB61B38Cu,
    0x9B64C2B0u, 0x86D3D2D4u, 0xA00AE278u, 0xBDBDF21Cu,
};

uint32_t bl_crc32(const uint8_t *data, uint32_t size)
{
    uint32_t crc = 0xFFFFFFFFu;
    while (size-- > 0u) {
        crc ^= *data++;
        crc = (crc >> 4) ^ crc32_nibble[crc & 0x0Fu];
        crc = (crc >> 4) ^ crc32_nibble[crc & 0x0Fu];
    }
    return ~crc;
}

#else

uint32_t bl_crc32(const uint8_t *data, uint32_t size)
{
    uint32_t crc = 0xFFFFFFFFu;
    while (size-- > 0u) {
        crc ^= *data++;
        for (uint32_t bit = 0; bit < 8u; bit++) {
            crc = (crc >> 1) ^ (CRC32_POLY & (0u - (crc & 1u)));
        }
    }
    return ~crc;
}

#endif
