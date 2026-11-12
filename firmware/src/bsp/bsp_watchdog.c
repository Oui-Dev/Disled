/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "bsp_watchdog.h"

#include "ch32.h"

#define LSI_HZ          40000u
#define IWDG_DIVIDER    32u
#define IWDG_RELOAD_MAX 0x0FFFu

void bsp_watchdog_start(uint16_t timeout_ms)
{
    uint32_t reload = ((uint32_t)timeout_ms * (LSI_HZ / IWDG_DIVIDER)) / 1000u;
    if (reload > IWDG_RELOAD_MAX) {
        reload = IWDG_RELOAD_MAX;
    }
    IWDG_WriteAccessCmd(IWDG_WriteAccess_Enable);
    IWDG_SetPrescaler(IWDG_Prescaler_32);
    IWDG_SetReload((uint16_t)reload);
    IWDG_ReloadCounter();
    IWDG_Enable();
}

void bsp_watchdog_feed(void)
{
    IWDG_ReloadCounter();
}
