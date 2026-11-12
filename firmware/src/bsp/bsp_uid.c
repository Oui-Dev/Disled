/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "bsp_uid.h"

/* Electronic signature (ESIG) words, CH32V20x reference manual. */
#define ESIG_UNIID1 (*(const volatile uint32_t *)0x1FFFF7E8u)
#define ESIG_UNIID2 (*(const volatile uint32_t *)0x1FFFF7ECu)
#define ESIG_UNIID3 (*(const volatile uint32_t *)0x1FFFF7F0u)

void bsp_uid_read(uint8_t uid[BSP_UID_BYTES])
{
    const uint32_t words[3] = { ESIG_UNIID1, ESIG_UNIID2, ESIG_UNIID3 };

    for (size_t w = 0; w < 3u; w++) {
        for (size_t b = 0; b < 4u; b++) {
            uid[(w * 4u) + b] = (uint8_t)(words[w] >> (24u - (8u * b)));
        }
    }
}

void bsp_uid_hex(char out[BSP_UID_HEX_CHARS])
{
    static const char digits[] = "0123456789ABCDEF";
    uint8_t uid[BSP_UID_BYTES];

    bsp_uid_read(uid);
    for (size_t i = 0; i < BSP_UID_BYTES; i++) {
        out[2u * i] = digits[uid[i] >> 4];
        out[(2u * i) + 1u] = digits[uid[i] & 0x0Fu];
    }
}
