/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Pin map of the Disled board, pcb 1.0.0, board revision 4 in INFO
 * (hardware/README.md). Nothing else knows where things are wired.
 */
#ifndef DISLED_BOARD_H
#define DISLED_BOARD_H

#include "ch32.h"

#define BOARD_REVISION 4u

/* RGB LED: low-side MOSFETs, active high, 10 kOhm gate pull-downs.
 * TIM3 channels 1 to 3 drive red, green and blue (default pin mapping). */
#define BOARD_LED_TIMER      TIM3
#define BOARD_LED_RED_PORT   GPIOA
#define BOARD_LED_RED_PIN    GPIO_Pin_6 /* TIM3_CH1 */
#define BOARD_LED_GREEN_PORT GPIOA
#define BOARD_LED_GREEN_PIN  GPIO_Pin_7 /* TIM3_CH2 */
#define BOARD_LED_BLUE_PORT  GPIOB
#define BOARD_LED_BLUE_PIN   GPIO_Pin_0 /* TIM3_CH3 */

/* Touch electrode: CN1.2, through R6 (2.2 kOhm), to PB1, ADC channel 9. */
#define BOARD_TOUCH_PORT        GPIOB
#define BOARD_TOUCH_PIN         GPIO_Pin_1
#define BOARD_TOUCH_ADC_CHANNEL ADC_Channel_9

/* Ambient light sensor VEML7700 on I2C1, default pin mapping. */
#define BOARD_I2C           I2C1
#define BOARD_I2C_PORT      GPIOB
#define BOARD_I2C_SCL_PIN   GPIO_Pin_6
#define BOARD_I2C_SDA_PIN   GPIO_Pin_7
#define BOARD_VEML7700_ADDR 0x10u

/*
 * Unused pins, set to analog input (lowest leakage, no floating inputs).
 * Deliberately absent (constraint N3 in hardware/README.md):
 *  - PA10 is bonded to PA11 (USB D-) on this package: it must stay a
 *    floating input, its reset state. Never drive it or pull it.
 *  - PB8 is bonded to BOOT0: never drive it.
 *  - PA13/PA14 are the SDI debug pads, PD0/PD1 the crystal.
 */
#define BOARD_UNUSED_PINS_A                                                                     \
    (GPIO_Pin_0 | GPIO_Pin_1 | GPIO_Pin_2 | GPIO_Pin_3 | GPIO_Pin_4 | GPIO_Pin_5 | GPIO_Pin_9 | \
     GPIO_Pin_15)
#define BOARD_UNUSED_PINS_B (GPIO_Pin_3 | GPIO_Pin_4 | GPIO_Pin_5)

_Static_assert((BOARD_UNUSED_PINS_A & GPIO_Pin_10) == 0, "PA10 shares a pad with USB D-");
_Static_assert((BOARD_UNUSED_PINS_B & GPIO_Pin_8) == 0, "PB8 shares a pad with BOOT0");

#endif /* DISLED_BOARD_H */
