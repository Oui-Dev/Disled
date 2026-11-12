/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The DISLED contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Boot decision, clocks, LED and main loop of the bootloader.
 *
 * After a reset, on the internal 8 MHz oscillator, the application starts
 * at once unless it asked for update mode (DATAR8) or its image is not
 * valid. It then finds the chip as after a reset: only the PWR and BKP
 * clocks were on, and they are off again.
 *
 * Update mode runs the crystal and the PLL at 48 MHz (the USB clock; flash
 * programming needs 60 MHz or less), polls the USB device in the loop and
 * blinks the green LED. It ends with a system reset only.
 */
#include "bl_core.h"
#include "ch32.h"
#include "usbd.h"

/* SysTick counts at HCLK / 8 = 6 MHz in update mode. */
#define TICKS_PER_MS     6000u
#define BOOT_TIMEOUT     (50u * TICKS_PER_MS) /* for the host to collect the BOOT reply */
#define BLINK_SLOW_SHIFT 22u                  /* 1.4 s period: waiting for the computer */
#define BLINK_FAST_SHIFT 20u                  /* 0.35 s period: an update runs */
/* Asked for update mode with a valid application, and no request since: the
 * computer gave up, back to the application. */
#define IDLE_TIMEOUT (60000u * TICKS_PER_MS)

/* About 50 ms at 8 MHz; the crystal starts in about 2 ms. */
#define WAIT_LOOPS 0x20000u

void bl_system_reset(void)
{
    PFIC->CFGR = NVIC_KEY3 | (1u << 7); /* RESETSYS */
    for (;;) {
    }
}

/* Machine mode, interrupts off, clocks at their reset values: the
 * application's startup runs as after a reset. */
__attribute__((noreturn)) static void start_application(void)
{
    RCC->APB1PCENR = 0;
    __asm volatile("jr %0" : : "r"(BL_APP_OFFSET));
    __builtin_unreachable();
}

static bool wait_for(const volatile uint32_t *reg, uint32_t mask)
{
    for (uint32_t i = 0; i < WAIT_LOOPS; i++) {
        if ((*reg & mask) != 0u) {
            return true;
        }
    }
    return false;
}

/* PLL = 8 MHz crystal x 6 = 48 MHz: core, flash and USB (USB prescaler 1).
 * APB1, with the USB registers, at 24 MHz. */
static bool start_clocks(void)
{
    RCC->CTLR |= RCC_HSEON;
    if (!wait_for(&RCC->CTLR, RCC_HSERDY)) {
        return false;
    }
    RCC->CFGR0 = RCC_PLLSRC_HSE | RCC_PLLXTPRE_HSE | RCC_PLLMULL6 | RCC_PPRE1_DIV2;
    RCC->CTLR |= RCC_PLLON;
    if (!wait_for(&RCC->CTLR, RCC_PLLRDY)) {
        return false;
    }
    RCC->CFGR0 |= RCC_SW_PLL;
    return wait_for(&RCC->CFGR0, RCC_SWS_PLL);
}

static uint32_t ticks(void)
{
    /* Low word of the 64-bit counter. */
    return *(const volatile uint32_t *)&SysTick->CNT;
}

int main(void)
{
    RCC->APB1PCENR = RCC_PWREN | RCC_BKPEN;
    const bool requested = BKP->DATAR8 == BL_ENTER_MAGIC;
    if (requested) {
        PWR->CTLR |= PWR_CTLR_DBP;
        BKP->DATAR8 = 0; /* once: the next reset starts the application */
    }
    const uint8_t reasons = bl_boot_reasons(BL_FLASH, requested);
    if (reasons == 0u) {
        start_application();
    }

    if (!start_clocks()) {
        bl_system_reset(); /* no USB without the crystal: try again */
    }
    SysTick->CTLR = 1u; /* STE only: free-running up-counter at HCLK / 8 */
    RCC->APB2PCENR |= RCC_IOPAEN;
    BL_LED_PORT->CFGLR = (BL_LED_PORT->CFGLR & ~(0xFu << 28)) | (0x2u << 28); /* PA7 output */
    usbd_init();

    bool booting = false;
    uint32_t boot_at = 0;
    uint32_t active_at = ticks();
    for (;;) {
        IWDG->CTLR = 0xAAAA;              /* reload, should a watchdog run */
        const bool request = usbd_poll(); /* resets once the host has the BOOT reply */

        const uint32_t now = ticks();
        if (request) {
            active_at = now;
        }
        if (!booting && usbd_booting()) {
            booting = true;
            boot_at = now + BOOT_TIMEOUT;
        }
        if (booting && (int32_t)(now - boot_at) >= 0) {
            bl_system_reset();
        }
        if ((reasons & BL_REASON_APP_INVALID) == 0u && !bl_proto_updating() &&
            now - active_at > IDLE_TIMEOUT) {
            bl_system_reset();
        }

        const uint32_t shift = bl_proto_updating() ? BLINK_FAST_SHIFT : BLINK_SLOW_SHIFT;
        const bool on = !usbd_suspended() && ((now >> shift) & 1u) != 0u;
        BL_LED_PORT->BSHR = on ? BL_LED_PIN : ((uint32_t)BL_LED_PIN << 16);
    }
}
