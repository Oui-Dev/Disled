// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

import { useI18n } from 'vue-i18n';
import type { PluginView } from '@/bindings/PluginView';
import type { Reason } from '@/bindings/Reason';

/** The port of the events of the plugins (plugins/http.rs). */
const PORT = 47831;

/** How the integrations window words the plugins. */
export function usePluginTexts() {
  const { t } = useI18n();

  /** Why a plugin cannot be read, or an import was refused. */
  function reason(why: Reason): string {
    switch (why.kind) {
      case 'missing':
        return t('integrations.reason.missing');
      case 'json':
        return t('integrations.reason.json', { line: why.line });
      case 'field':
        return t('integrations.reason.field', { field: why.field });
      case 'invalid':
        return t('integrations.reason.invalid', { field: why.field });
      case 'api':
        return t('integrations.reason.api', { api: why.api });
      case 'id':
        return t('integrations.reason.id');
      case 'link':
        return t('integrations.reason.link', { path: why.path });
      case 'size':
        return t('integrations.reason.size');
      case 'path':
        return t('integrations.reason.path', { path: why.path });
    }
  }

  /** "3 s", "2 min", "1 h". */
  function ago(seconds: number): string {
    const elapsed = Math.max(0, Math.floor(seconds));
    if (elapsed < 60) {
      return t('integrations.ago.seconds', { n: elapsed });
    }
    if (elapsed < 3600) {
      return t('integrations.ago.minutes', { n: Math.floor(elapsed / 60) });
    }
    return t('integrations.ago.hours', { n: Math.floor(elapsed / 3600) });
  }

  /** The line under the name of a plugin in the list. */
  function subtitle(plugin: PluginView, driving: boolean): string {
    if (plugin.unreadable) {
      return t('integrations.unreadable', { file: 'plugin.json' });
    }
    if (!plugin.approved) {
      return t('integrations.new');
    }
    if (plugin.modified) {
      return t('integrations.status.modified');
    }
    if (!plugin.enabled) {
      return t('integrations.off');
    }
    if (plugin.program?.kind === 'failed') {
      return t('integrations.status.failed');
    }
    if (driving) {
      return t('integrations.driving');
    }
    if (plugin.capabilities.events && !plugin.last_event) {
      return t('integrations.noEvent');
    }
    return t('integrations.on');
  }

  /** The status line of the details, `now` in seconds since 1970. */
  function status(plugin: PluginView, now: number): string | null {
    if (plugin.unreadable) {
      return reason(plugin.unreadable);
    }
    if (!plugin.approved) {
      return t('integrations.newDetail');
    }
    if (plugin.modified) {
      return t('integrations.status.modified');
    }
    if (!plugin.enabled) {
      return null;
    }
    if (plugin.port_taken) {
      return t('integrations.status.portTaken', { port: PORT });
    }
    switch (plugin.program?.kind) {
      case 'failed':
        return t('integrations.status.failed');
      case 'not_found':
        return t('integrations.status.notFound', { program: plugin.program.program });
      case 'stopped': {
        const stopped = t('integrations.status.stopped');
        return plugin.program.error ? `${stopped} ${plugin.program.error}` : stopped;
      }
    }
    const event = plugin.last_event;
    if (event) {
      const since = ago(now - event.at);
      return event.state
        ? t('integrations.lastEvent', { ago: since, state: event.state })
        : t('integrations.lastEventNoState', { ago: since });
    }
    if (plugin.capabilities.events) {
      return t('integrations.noEventYet');
    }
    return plugin.program?.kind === 'running' ? t('integrations.status.running') : null;
  }

  return { reason, ago, subtitle, status };
}
