/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "bsp.h"

#include "board.h"
#include "bsp_gpio.h"
#include "bsp_power.h"
#include "bsp_time.h"
#include "ch32.h"

static reset_cause_t s_reset_cause = RESET_CAUSE_UNKNOWN;
static bool s_crystal_ok;

static void capture_reset_cause(void)
{
    /* Highest priority first: a watchdog reset also sets the pin flag. The
     * window watchdog and the low-power modes are never used. */
    const uint32_t flags = RCC->RSTSCKR;
    if ((flags & RCC_IWDGRSTF) != 0u) {
        s_reset_cause = RESET_CAUSE_WATCHDOG;
    } else if ((flags & RCC_SFTRSTF) != 0u) {
        s_reset_cause = RESET_CAUSE_SOFTWARE;
    } else if ((flags & RCC_PORRSTF) != 0u) {
        s_reset_cause = RESET_CAUSE_POWER_ON;
    } else if ((flags & RCC_PINRSTF) != 0u) {
        s_reset_cause = RESET_CAUSE_PIN;
    }
    RCC->RSTSCKR |= RCC_RMVF;
}

static void init_clocks(void)
{
    /* SystemInit() left the chip on the internal oscillator, with these
     * fields cleared. PLL = 8 MHz crystal x 18 = 144 MHz = HCLK.
     * I2C needs PCLK1 <= 60 MHz: the SDK writes PCLK1 in MHz into a 6-bit
     * field and only clamps it to 60 afterwards, so 72 MHz silently becomes
     * 8 MHz and breaks every byte transfer. 144 / 4 = 36 MHz; the APB1
     * timers then run at 72 MHz.
     * The ADC clock, PCLK2 / 8, must stay within 14 MHz: 72 / 8 = 9 MHz.
     * USB: 144 / 3 = 48 MHz. */
    RCC->CFGR0 |= RCC_PLLSRC_HSE | RCC_PLLXTPRE_HSE | RCC_PLLMULL18 | RCC_PPRE1_DIV4 |
                  RCC_PPRE2_DIV2 | RCC_ADCPRE_DIV8 | ((uint32_t)RCC_USBCLKSource_PLLCLK_Div3 << 22);
    /* If the crystal fails, the chip stays on the internal oscillator. */
    s_crystal_ok = bsp_power_full_clock();

    RCC_APB2PeriphClockCmd(RCC_APB2Periph_GPIOA | RCC_APB2Periph_GPIOB | RCC_APB2Periph_AFIO,
                           ENABLE);
}

static void init_unused_pins(void)
{
    bsp_gpio_config(GPIOA, BOARD_UNUSED_PINS_A, GPIO_Mode_AIN, GPIO_Speed_2MHz);
    bsp_gpio_config(GPIOB, BOARD_UNUSED_PINS_B, GPIO_Mode_AIN, GPIO_Speed_2MHz);
}

void bsp_init(void)
{
    capture_reset_cause();
    /* The time base first: it follows the start of the crystal as it
     * follows the end of a USB suspend. */
    bsp_time_init();
    init_clocks();
    init_unused_pins();
}

reset_cause_t bsp_reset_cause(void)
{
    return s_reset_cause;
}

bool bsp_crystal_ok(void)
{
    return s_crystal_ok;
}

uint32_t bsp_apb1_timer_clock_hz(void)
{
    /* APB1 runs at HCLK / 4 (init_clocks), and its timers at twice that. */
    return SystemCoreClock / 2u;
}

/* Stack bounds from the linker script (2 KB at the top of RAM). */
extern uint32_t _susrstack[];
extern uint32_t _eusrstack[];

#define STACK_PATTERN 0xA5A5A5A5u

void bsp_stack_paint(void)
{
    /* Stop well below the current frame. */
    const uintptr_t limit = (uintptr_t)__builtin_frame_address(0) - 64u;
    for (uint32_t *word = _susrstack; (uintptr_t)word < limit; word++) {
        *word = STACK_PATTERN;
    }
}

uint16_t bsp_stack_free_min(void)
{
    /* Addresses as integers: the two linker symbols are different objects
     * for the C language. */
    const uintptr_t bottom = (uintptr_t)_susrstack;
    const uintptr_t top = (uintptr_t)_eusrstack;
    uintptr_t address = bottom;
    while (address < top && *(const uint32_t *)address == STACK_PATTERN) {
        address += sizeof(uint32_t);
    }
    return (uint16_t)(address - bottom);
}

void bsp_system_reset(void)
{
    NVIC_SystemReset();
    for (;;) {
    }
}
