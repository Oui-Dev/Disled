/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "bsp_fault.h"

#include "bsp.h"
#include "ch32.h"
#include "disled_assert.h"

/* Backup registers (16 bits each, cleared by a power cycle):
 *   DATAR1     FAULT_MAGIC while a fault is waiting to be reported
 *   DATAR2..7  last fault: mcause, mepc, mtval (kept after the report)
 *   DATAR8     BSP_UPDATE_MAGIC: update mode requested (ENTER_UPDATE, _bl
 *              builds only)
 *   DATAR9     boots since power-up
 *   DATAR10    watchdog resets (high byte), faults (low byte) */
#define FAULT_MAGIC 0xFA17u

static void unlock_backup_domain(void)
{
    RCC->APB1PCENR |= RCC_APB1Periph_PWR | RCC_APB1Periph_BKP;
    PWR->CTLR |= PWR_CTLR_DBP;
}

static uint32_t read_u32(volatile uint16_t *low, volatile uint16_t *high)
{
    return (uint32_t)*low | ((uint32_t)*high << 16);
}

static void write_u32(volatile uint16_t *low, volatile uint16_t *high, uint32_t value)
{
    *low = (uint16_t)(value & 0xFFFFu);
    *high = (uint16_t)(value >> 16);
}

static fault_record_t read_last(void)
{
    const fault_record_t record = {
        .valid = true,
        .mcause = read_u32(&BKP->DATAR2, &BKP->DATAR3),
        .mepc = read_u32(&BKP->DATAR4, &BKP->DATAR5),
        .mtval = read_u32(&BKP->DATAR6, &BKP->DATAR7),
    };
    return record;
}

static uint8_t saturating_inc(uint8_t value)
{
    return (value == UINT8_MAX) ? value : (uint8_t)(value + 1u);
}

fault_record_t bsp_fault_take(void)
{
    fault_record_t record = { 0 };

    unlock_backup_domain();
    const bool fault = BKP->DATAR1 == FAULT_MAGIC;
    if (fault) {
        record = read_last();
    }
    BKP->DATAR1 = 0;

    if (BKP->DATAR9 != UINT16_MAX) {
        BKP->DATAR9 = (uint16_t)(BKP->DATAR9 + 1u);
    }
    uint8_t watchdog_resets = (uint8_t)(BKP->DATAR10 >> 8);
    uint8_t faults = (uint8_t)(BKP->DATAR10 & 0xFFu);
    if (bsp_reset_cause() == RESET_CAUSE_WATCHDOG) {
        watchdog_resets = saturating_inc(watchdog_resets);
    }
    if (fault) {
        faults = saturating_inc(faults);
    }
    BKP->DATAR10 = (uint16_t)(((uint16_t)watchdog_resets << 8) | faults);
#if DISLED_ENTER_UPDATE
    BKP->DATAR8 = 0; /* a request no bootloader took */
#endif
    return record;
}

#if DISLED_ENTER_UPDATE
void bsp_reset_to_update(void)
{
    __disable_irq();
    unlock_backup_domain();
    BKP->DATAR8 = BSP_UPDATE_MAGIC;
    NVIC_SystemReset();
    for (;;) {
    }
}
#endif

fault_stats_t bsp_fault_stats(void)
{
    unlock_backup_domain();
    fault_stats_t stats = {
        .boots = BKP->DATAR9,
        .watchdog_resets = (uint8_t)(BKP->DATAR10 >> 8),
        .faults = (uint8_t)(BKP->DATAR10 & 0xFFu),
    };
    if (stats.faults != 0u) {
        stats.last = read_last();
    }
    return stats;
}

__attribute__((noreturn)) static void record_and_reset(uint32_t mcause, uint32_t mepc,
                                                       uint32_t mtval)
{
    unlock_backup_domain();
    write_u32(&BKP->DATAR2, &BKP->DATAR3, mcause);
    write_u32(&BKP->DATAR4, &BKP->DATAR5, mepc);
    write_u32(&BKP->DATAR6, &BKP->DATAR7, mtval);
    BKP->DATAR1 = FAULT_MAGIC;
    NVIC_SystemReset();
    for (;;) {
    }
}

/* One handler for both vectors (see usb_irq.c on folded handlers). */
void bsp_fault_handler(void) BSP_ISR;
void bsp_fault_handler(void)
{
    record_and_reset(__get_MCAUSE(), __get_MEPC(), __get_MTVAL());
}

void HardFault_Handler(void) __attribute__((alias("bsp_fault_handler")));
void NMI_Handler(void) __attribute__((alias("bsp_fault_handler")));

/* Failed DISLED_ASSERT, in release and debug builds alike: the fault is
 * reported at the next boot (INFO, DIAGNOSTICS, console). */
void disled_assert_failed(uint32_t line)
{
    __disable_irq();
    record_and_reset(BSP_FAULT_ASSERT_CAUSE | (line & 0xFFFFu),
                     (uint32_t)(uintptr_t)__builtin_return_address(0), 0);
}
