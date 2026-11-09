/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * USB device: link state, HID report transport and, in debug builds, the CDC
 * console. Everything runs from usb_device_task() in the main loop, and nothing blocks.
 */
#ifndef DISLED_USB_DEVICE_H
#define DISLED_USB_DEVICE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "disled_types.h"
#include "protocol.h"

/* Starts the USB device. Without the crystal the USB clock would be out
 * of tolerance, so the device stays detached and the lamp runs locally. */
void usb_device_init(bool crystal_ok);

void usb_device_task(void);

/* USB wake-up interrupt (EXTI line 18), for the slow clock of suspend. */
void usb_device_wake_irq(bool enabled);

usb_link_t usb_device_link(void);

/* Queues a message report. False if the queue is full or no host is
 * listening; the caller decides whether that matters. */
bool usb_hid_send(const uint8_t payload[PROTO_REPORT_SIZE]);

/* Nothing waits to go out. The messages the lamp sends on its own wait for
 * it, so that they never fill the queue while the host reads nothing (Linux
 * reads only while an app has the lamp open), and the answers find room. */
bool usb_hid_idle(void);

/* Pops a received command report. False if none is waiting. */
bool usb_hid_receive(uint8_t payload[PROTO_REPORT_SIZE]);

#if DISLED_DEBUG
/* Console stream over CDC. Reads what arrived; writes are dropped while
 * no terminal is open or when the buffer is full. */
size_t usb_cdc_read(char *buffer, size_t max);
void usb_cdc_write(const char *data, size_t length);
#endif

#endif /* DISLED_USB_DEVICE_H */
