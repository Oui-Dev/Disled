/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * USBD interrupts, for TinyUSB. The three vectors alias one handler: GCC folds
 * identical handlers into calls to each other, wrong for interrupt functions.
 * In suspend, the wake-up restores the PLL clock before TinyUSB sees anything.
 */
#include "bsp/bsp_power.h"
#include "bsp/ch32.h"
#include "tusb.h"

void usb_irq_handler(void) BSP_ISR;
void usb_irq_handler(void)
{
    (void)bsp_power_full_clock_from_isr(); /* returns at once when fast */
    EXTI->INTFR = EXTI_Line18;             /* USB wake-up line, if it fired */
    tud_int_handler(0);
}

void USB_LP_CAN1_RX0_IRQHandler(void) __attribute__((alias("usb_irq_handler")));
void USB_HP_CAN1_TX_IRQHandler(void) __attribute__((alias("usb_irq_handler")));
void USBWakeUp_IRQHandler(void) __attribute__((alias("usb_irq_handler")));
