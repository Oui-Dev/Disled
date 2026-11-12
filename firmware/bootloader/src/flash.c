/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The DISLED contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Fast page erase and program of the CH32V20x, 256 bytes at a time, as the
 * SDK's FLASH_ErasePage_Fast() and FLASH_ProgramPage_Fast() do. The flash
 * controller needs HCLK <= 60 MHz: the bootloader runs at 48 MHz. The
 * datasheet gives 16 ms to erase a page and 2 ms to program it; meanwhile
 * the host waits on a NAK.
 */
#include <stddef.h>

#include "bl_core.h"
#include "ch32.h"

#define FLASH_KEY1 0x45670123u
#define FLASH_KEY2 0xCDEF89ABu

#define CR_STRT    0x00000040u
#define CR_LOCK    0x00000080u
#define CR_FLOCK   0x00008000u /* fast mode lock */
#define CR_PAGE_PG 0x00010000u
#define CR_PAGE_ER 0x00020000u
#define CR_PG_STRT 0x00200000u
#define SR_BSY     0x00000001u
#define SR_WR_BSY  0x00000002u

static void wait_while(uint32_t busy)
{
    while ((FLASH->STATR & busy) != 0u) {
    }
}

bool bl_port_write_page(uint32_t offset, const uint32_t *data)
{
    volatile uint32_t *const page = (volatile uint32_t *)(FLASH_BASE + offset);

    /* Unlocked for this page only. */
    FLASH->KEYR = FLASH_KEY1;
    FLASH->KEYR = FLASH_KEY2;
    FLASH->MODEKEYR = FLASH_KEY1;
    FLASH->MODEKEYR = FLASH_KEY2;

    FLASH->CTLR |= CR_PAGE_ER;
    FLASH->ADDR = (uint32_t)page;
    FLASH->CTLR |= CR_STRT;
    wait_while(SR_BSY);
    FLASH->CTLR &= ~CR_PAGE_ER;

    if (data != NULL) {
        FLASH->CTLR |= CR_PAGE_PG;
        wait_while(SR_BSY | SR_WR_BSY);
        for (uint32_t i = 0; i < BL_PAGE_SIZE / 4u; i++) {
            page[i] = data[i]; /* into the page buffer of the controller */
            wait_while(SR_WR_BSY);
        }
        FLASH->CTLR |= CR_PG_STRT;
        wait_while(SR_BSY);
        FLASH->CTLR &= ~CR_PAGE_PG;
    }
    FLASH->CTLR |= CR_FLOCK | CR_LOCK;

    /* Read back. A write-protected sector ignores the erase and the program,
     * which only this check reveals. The caller checks an erase. */
    if (data != NULL) {
        for (uint32_t i = 0; i < BL_PAGE_SIZE / 4u; i++) {
            if (page[i] != data[i]) {
                return false;
            }
        }
    }
    return true;
}
