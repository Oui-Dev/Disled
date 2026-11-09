/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Small snprintf for logs and the debug console: predictable size, no heap,
 * no floating point. Supports %d %u %s %%, the '0' and '-' flags, a width,
 * and the 'l' length modifier.
 */
#ifndef DISLED_FMT_H
#define DISLED_FMT_H

#include <stdarg.h>
#include <stddef.h>

/* Like vsnprintf: always terminates the output when size > 0 and returns
 * the length the complete output would have. */
size_t fmt_vsnprintf(char *out, size_t size, const char *format, va_list args);

__attribute__((format(printf, 3, 4))) size_t fmt_snprintf(char *out, size_t size,
                                                          const char *format, ...);

#endif /* DISLED_FMT_H */
