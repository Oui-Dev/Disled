/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "bsp_gpio.h"

void bsp_gpio_config(GPIO_TypeDef *port, uint16_t pins, GPIOMode_TypeDef mode,
                     GPIOSpeed_TypeDef speed)
{
    /* CNF and MODE, 4 bits a pin: the low bits of the SDK mode, plus the
     * speed for an output mode (bit 4). */
    const uint32_t config =
        ((uint32_t)mode & 0x0Fu) | ((((uint32_t)mode & 0x10u) != 0u) ? (uint32_t)speed : 0u);
    for (uint32_t pin = 0; pin < 16u; pin++) {
        if ((pins & (1u << pin)) == 0u) {
            continue;
        }
        volatile uint32_t *reg = (pin < 8u) ? &port->CFGLR : &port->CFGHR;
        const uint32_t shift = (pin % 8u) * 4u;
        *reg = (*reg & ~(0xFu << shift)) | (config << shift);
    }
}
