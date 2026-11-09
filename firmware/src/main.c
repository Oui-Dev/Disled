/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The DISLED contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Disled firmware entry point. The startup code leaves the chip on its
 * internal oscillator (SystemInit); everything else starts here.
 */
#include "app/app.h"
#include "bsp/bsp.h"
#include "bsp/bsp_fault.h"

int main(void)
{
    bsp_stack_paint();
    bsp_init();
    const fault_record_t fault = bsp_fault_take();
    app_init(&fault);
    app_run();
}
