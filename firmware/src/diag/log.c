/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "log.h"

#if DISLED_DEBUG

#include <stdarg.h>

#include "bsp/bsp_time.h"
#include "fmt.h"
#include "usb/usb_device.h"

#define LINE_MAX 128u

void log_write(const char *format, ...)
{
    char line[LINE_MAX];
    const uint32_t now = bsp_time_ms();
    /* "I", for info: tools/touch_replay looks for "] I touch:". */
    size_t length = fmt_snprintf(line, sizeof line, "[%6lu.%03lu] I ", (unsigned long)(now / 1000u),
                                 (unsigned long)(now % 1000u));
    if (length < sizeof line) {
        va_list args;
        va_start(args, format);
        length += fmt_vsnprintf(&line[length], sizeof line - length, format, args);
        va_end(args);
    }
    /* Keep room for the line ending, even when truncated. */
    if (length > sizeof line - 3u) {
        length = sizeof line - 3u;
    }
    line[length++] = '\r';
    line[length++] = '\n';
    usb_cdc_write(line, length);
}

#endif
