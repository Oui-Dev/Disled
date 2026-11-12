// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

import { useI18n } from 'vue-i18n';
import type { AppError } from '@/stores/errors';

/** Words an error: the cause in the user's language, the backend's detail as it came. */
export function useErrorText(): (error: AppError) => string {
  const { t } = useI18n();
  return (error) => {
    switch (error.kind) {
      case 'backend':
        return t('errors.backend', { detail: error.detail });
      case 'rejected':
        return t('errors.rejected', {
          command: `0x${error.command.toString(16).padStart(2, '0')}`,
          error: t(`enums.protocolError.${error.error}`),
        });
      case 'unreadable':
        return t('errors.unreadable', { detail: error.detail });
    }
  };
}
