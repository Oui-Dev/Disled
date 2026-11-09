/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Logging to the CDC console, debug builds only: in release builds the macro
 * compiles to nothing, its arguments still type-checked. LOG_INFO("touch: color
 * %u", 1) prints "[  12.345] I touch: color 1".
 */
#ifndef DISLED_LOG_H
#define DISLED_LOG_H

#if DISLED_DEBUG

__attribute__((format(printf, 1, 2))) void log_write(const char *format, ...);

#define LOG_INFO(...) log_write(__VA_ARGS__)

#else

__attribute__((format(printf, 1, 2))) static inline void log_discard(const char *format, ...)
{
    (void)format;
}

#define LOG_INFO(...) log_discard(__VA_ARGS__)

#endif

#endif /* DISLED_LOG_H */
