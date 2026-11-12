/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "bsp_time.h"

#include <stddef.h>

#include "ch32.h"

/* SysTick control bits (QingKe V4 core). */
#define STK_CTLR_STE   (1u << 0) /* counter enable */
#define STK_CTLR_STIE  (1u << 1) /* interrupt enable */
#define STK_CTLR_STCLK (1u << 2) /* clock = HCLK (not HCLK/8) */
#define STK_CTLR_STRE  (1u << 3) /* auto-reload on compare match */

static volatile uint32_t s_ms;
static volatile bsp_tick_hook_t s_tick_hook;

void SysTick_Handler(void) BSP_ISR;
void SysTick_Handler(void)
{
    SysTick->SR = 0;
    s_ms++;
    const bsp_tick_hook_t hook = s_tick_hook;
    if (hook != NULL) {
        hook();
    }
}

void bsp_time_set_tick_hook(bsp_tick_hook_t hook)
{
    s_tick_hook = hook;
}

static void start_systick(void)
{
    SysTick->CTLR = 0;
    SysTick->SR = 0;
    SysTick->CNT = 0;
    SysTick->CMP = (SystemCoreClock / 1000u) - 1u;
    SysTick->CTLR = STK_CTLR_STE | STK_CTLR_STIE | STK_CTLR_STCLK | STK_CTLR_STRE;
}

void bsp_time_init(void)
{
    start_systick();
    NVIC_EnableIRQ(SysTick_IRQn);
}

void bsp_time_core_clock_changed(void)
{
    /* The restart drops a tick not counted yet and the part of the
     * millisecond under way: both count as whole ones, so that no time is
     * lost. */
    if ((SysTick->SR & 1u) != 0u) {
        s_ms++;
    }
    s_ms++;
    start_systick();
}

uint32_t bsp_time_ms(void)
{
    return s_ms;
}

/* Current SysTick count (lower word; the counter never exceeds CMP). */
static uint32_t systick_count(void)
{
    return (uint32_t)SysTick->CNT;
}

void bsp_time_delay_us(uint32_t us)
{
    /* The counter reloads every millisecond: add up the ticks between two
     * reads, so that a wait of a period or more ends too. */
    const uint32_t period = (uint32_t)SysTick->CMP + 1u;
    const uint32_t ticks = us * (SystemCoreClock / 1000000u);
    uint32_t last = systick_count();
    uint32_t elapsed = 0;

    while (elapsed < ticks) {
        const uint32_t now = systick_count();
        elapsed += (now >= last) ? (now - last) : (period - last + now);
        last = now;
    }
}
