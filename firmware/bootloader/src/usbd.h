/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The DISLED contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Minimal driver of the USBD peripheral (the STM32 "FSDEV" design), polled:
 * enumeration, control transfers on endpoint 0 and one vendor HID interface
 * with interrupt endpoints IN and OUT of 64 bytes, which carry the update
 * protocol (bl_core.h).
 */
#ifndef BL_USBD_H
#define BL_USBD_H

#include <stdbool.h>

/* pid.codes test PID, for private testing only (LICENSING.md): the
 * bootloader needs a PID of its own before lamps ship with it. */
#define USBD_VID 0x1209u
#define USBD_PID 0x0003u
/* bcdDevice: protocol version of the bootloader, in BCD. */
#define USBD_BCD_DEVICE 0x0100u

/* Needs the 48 MHz USB clock. Connects the D+ pull-up. */
void usbd_init(void);

/* Handles the pending USB events, and the update requests. True when it
 * handled a request. */
bool usbd_poll(void);

/* The host suspended the bus after the configuration: the LED goes off. */
bool usbd_suspended(void);

/* BOOT was handled: the chip resets once the host collects the reply. */
bool usbd_booting(void);

#endif /* BL_USBD_H */
