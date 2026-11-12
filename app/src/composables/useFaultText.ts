// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// Firmware faults as the lamp reports them: RISC-V trap registers, or a
// failed assertion that the firmware encodes in mcause.

import { useI18n } from 'vue-i18n';
import type { Fault } from '@/bindings/Fault';
import type { FaultDetail } from '@/bindings/FaultDetail';

/** mcause 0x0A55nnnn marks a failed assertion of the firmware, nnnn being its source line. */
const ASSERTION = 0x0a55;

/** A 32-bit register value, as the RISC-V tools print it. */
function hex32(value: number): string {
  return `0x${value.toString(16).padStart(8, '0')}`;
}

/**
 * Words a firmware fault, from INFO or from DIAGNOSTICS with mtval too: a
 * failed assertion, or the trap registers.
 */
export function useFaultText(): (fault: Fault & Partial<FaultDetail>) => string {
  const { t } = useI18n();
  return ({ mcause, mepc, mtval }) => {
    if (mcause >>> 16 === ASSERTION) {
      return t('diagnostics.assertion', { line: mcause & 0xffff, caller: hex32(mepc) });
    }
    const registers = { mcause: hex32(mcause), mepc: hex32(mepc) };
    return mtval === undefined
      ? t('diagnostics.trap', registers)
      : t('diagnostics.trapValue', { ...registers, mtval: hex32(mtval) });
  };
}
