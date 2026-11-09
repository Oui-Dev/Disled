/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * The colors a short press steps through, in an order set by the host. The
 * default is the cycle of version 1: green, orange, red.
 */
#ifndef DISLED_TOUCH_CYCLE_H
#define DISLED_TOUCH_CYCLE_H

#include <stdbool.h>
#include <stdint.h>

#include "disled_types.h"

/* Every color the app offers: the 4 built-in ones and up to 5 custom ones. */
#define TOUCH_CYCLE_MAX 9u

typedef struct {
    uint8_t count;                  /* 1 to TOUCH_CYCLE_MAX */
    uint8_t slots[TOUCH_CYCLE_MAX]; /* color slots in order, zero past `count` */
} touch_cycle_t;

/* Green, orange, red. */
void touch_cycle_init(touch_cycle_t *cycle);

/* 1 to TOUCH_CYCLE_MAX colors, each one defined (bit per slot in
 * `defined`), none twice. */
bool touch_cycle_is_valid(const uint8_t *slots, uint32_t count, uint32_t defined);

/* Replaces the cycle with a valid one. Otherwise returns false and changes
 * nothing. */
bool touch_cycle_set(touch_cycle_t *cycle, const uint8_t *slots, uint32_t count, uint32_t defined);

/* The color after `color`: the next one in the cycle, the first one after the
 * last, and the first one from a color outside the cycle. */
lamp_color_t touch_cycle_next(const touch_cycle_t *cycle, lamp_color_t color);

/* Drops a color that no longer exists. An empty cycle becomes the default
 * one. */
void touch_cycle_remove(touch_cycle_t *cycle, uint32_t slot);

#endif /* DISLED_TOUCH_CYCLE_H */
