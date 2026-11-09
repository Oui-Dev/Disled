/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "bsp_power.h"

#include "bsp_time.h"
#include "ch32.h"

/* A few milliseconds at 8 MHz: the crystal starts in about 2 ms. */
#define START_TIMEOUT_LOOPS 0x8000u

/* Core clock from the crystal and the PLL: 8 MHz x 18. */
#define PLL_HZ 144000000u

/* On the internal oscillator: from reset until bsp_init() starts the
 * crystal, and in suspend. */
static volatile bool s_slow = true;

static bool wait_for(const volatile uint32_t *reg, uint32_t mask, uint32_t value)
{
    for (uint32_t i = 0; i < START_TIMEOUT_LOOPS; i++) {
        if ((*reg & mask) == value) {
            return true;
        }
    }
    return false;
}

/* Back on the internal oscillator, which never stops, then the PLL and the
 * crystal off. The PLL settings stay in CFGR0 for the way back. */
static void stop_crystal(void)
{
    RCC->CFGR0 = (RCC->CFGR0 & ~RCC_SW) | RCC_SW_HSI;
    (void)wait_for(&RCC->CFGR0, RCC_SWS, RCC_SWS_HSI);
    RCC->CTLR &= ~(RCC_PLLON | RCC_HSEON);
}

void bsp_power_slow_clock(void)
{
    if (s_slow) {
        return;
    }
    __disable_irq();
    stop_crystal();
    /* 8 MHz / 8: the core and the buses run at 1 MHz (APB1 at 250 kHz). */
    RCC->CFGR0 = (RCC->CFGR0 & ~RCC_HPRE) | RCC_HPRE_DIV8;
    SystemCoreClock = HSI_VALUE / 8u;
    bsp_time_core_clock_changed();
    s_slow = true;
    __enable_irq();
}

bool bsp_power_full_clock_from_isr(void)
{
    if (!s_slow) {
        return (RCC->CFGR0 & RCC_SWS) == RCC_SWS_PLL;
    }
    /* Full speed internal oscillator first, so that the waits below are
     * short and the time base stays right if the crystal fails. */
    RCC->CFGR0 = (RCC->CFGR0 & ~RCC_HPRE) | RCC_HPRE_DIV1;
    SystemCoreClock = HSI_VALUE;
    bsp_time_core_clock_changed();
    /* Cleared even if the crystal fails below, so that the next interrupt
     * does not wait for it again: bsp_power_full_clock() retries it. */
    s_slow = false;

    RCC->CTLR |= RCC_HSEON;
    bool ok = wait_for(&RCC->CTLR, RCC_HSERDY, RCC_HSERDY);
    if (ok) {
        RCC->CTLR |= RCC_PLLON;
        ok = wait_for(&RCC->CTLR, RCC_PLLRDY, RCC_PLLRDY);
    }
    if (ok) {
        RCC->CFGR0 = (RCC->CFGR0 & ~RCC_SW) | RCC_SW_PLL;
        ok = wait_for(&RCC->CFGR0, RCC_SWS, RCC_SWS_PLL);
    }
    if (!ok) {
        /* The core stays at 8 MHz, as SystemCoreClock says. */
        stop_crystal();
        return false;
    }
    SystemCoreClock = PLL_HZ;
    bsp_time_core_clock_changed();
    return true;
}

bool bsp_power_full_clock(void)
{
    __disable_irq();
    /* A crystal that failed in the wake-up interrupt gets another try:
     * without the PLL, USB has no clock, so no suspend would come to retry. */
    if ((RCC->CFGR0 & RCC_SWS) != RCC_SWS_PLL) {
        s_slow = true;
    }
    const bool fast = bsp_power_full_clock_from_isr();
    __enable_irq();
    return fast;
}
