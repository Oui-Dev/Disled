/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Pin modes, written directly: the SDK's GPIO_Init() is large for the few
 * fixed settings of this board.
 */
#ifndef DISLED_BSP_GPIO_H
#define DISLED_BSP_GPIO_H

#include <stdint.h>

#include "ch32.h"

/* Sets the mode of the `pins` of `port`, and the speed of outputs, as
 * GPIO_Init() does. Not for the inputs with a pull-up or a pull-down, which
 * also need the output register. */
void bsp_gpio_config(GPIO_TypeDef *port, uint16_t pins, GPIOMode_TypeDef mode,
                     GPIOSpeed_TypeDef speed);

#endif /* DISLED_BSP_GPIO_H */
