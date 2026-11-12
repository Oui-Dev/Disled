/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Single entry point to the WCH SDK header, whose warnings are silenced
 * here only. The bootloader uses the register definitions, no SDK function.
 */
#ifndef BL_CH32_H
#define BL_CH32_H

#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wstrict-prototypes"
#pragma GCC diagnostic ignored "-Wmissing-prototypes"
#pragma GCC diagnostic ignored "-Wshadow"
#include "ch32v20x.h"
#pragma GCC diagnostic pop

/* The flash as the CPU reads it, and the address the flash controller
 * programs: the 0x08000000 alias of the code at address 0. */
#define BL_FLASH ((const uint8_t *)FLASH_BASE)

/* Status LED: green only (PA7, gate of Q3, active high). At 100 % it draws
 * at most 63 mA (hardware/README.md, N2), within the 100 mA of an
 * unconfigured device with the MCU. Never configure PA10 (bonded to USB D-)
 * nor PB8 (bonded to BOOT0): constraint N3. */
#define BL_LED_PORT GPIOA
#define BL_LED_PIN  GPIO_Pin_7

__attribute__((noreturn)) void bl_system_reset(void);

#endif /* BL_CH32_H */
