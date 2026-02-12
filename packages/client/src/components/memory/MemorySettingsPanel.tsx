import { useEffect, useState } from 'react';
import type { MemorySettings } from '../../hooks/useMemoryApi';
import {
  getConsoleLanguageCopy,
  type ConsoleLanguage,
} from '../../i18n/consoleLanguage';

interface MemorySettingsPanelProps {
  settings: MemorySettings | null;
  isLoading: boolean;
  onSave: (patch: Partial<MemorySettings>) => Promise<void>;
  language?: ConsoleLanguage;
}

const asNumber = (value: string, fallback: number): number => {
  const parsed = Number.parseInt(value, 10);
  if (!Number.isFinite(parsed)) {
    return fallback;
  }
  return parsed;
};

export function MemorySettingsPanel({
  settings,
  isLoading,
  onSave,
  language = 'en',
}: MemorySettingsPanelProps) {
  const copy = getConsoleLanguageCopy(language).memory.settingsPanel;
  const [draft, setDraft] = useState<MemorySettings | null>(settings);

  useEffect(() => {
    setDraft(settings);
  }, [settings]);

  if (!draft) {
    return (
      <div className="info-block">
        <p className="section-note">{copy.noSettings}</p>
      </div>
    );
  }

  const submit = async () => {
    await onSave({
      enabled: draft.enabled,
      gatewayStoreEnabled: draft.gatewayStoreEnabled,
      rustStoreEnabled: draft.rustStoreEnabled,
      autoWrite: draft.autoWrite,
      promptInjection: draft.promptInjection,
      tokenBudget: draft.tokenBudget,
      retrievalTopK: draft.retrievalTopK,
      llmExtractEnabled: draft.llmExtractEnabled,
    });
  };

  return (
    <div className="memory-settings">
      <div className="memory-settings__grid">
        <label className="memory-toggle">
          <input
            type="checkbox"
            checked={draft.enabled}
            onChange={(event) => setDraft((prev) => (prev ? { ...prev, enabled: event.target.checked } : prev))}
          />
          <span>{copy.enabled}</span>
        </label>
        <label className="memory-toggle">
          <input
            type="checkbox"
            checked={draft.gatewayStoreEnabled}
            onChange={(event) =>
              setDraft((prev) => (prev ? { ...prev, gatewayStoreEnabled: event.target.checked } : prev))
            }
          />
          <span>{copy.gatewayStore}</span>
        </label>
        <label className="memory-toggle">
          <input
            type="checkbox"
            checked={draft.rustStoreEnabled}
            onChange={(event) =>
              setDraft((prev) => (prev ? { ...prev, rustStoreEnabled: event.target.checked } : prev))
            }
          />
          <span>{copy.rustStore}</span>
        </label>
        <label className="memory-toggle">
          <input
            type="checkbox"
            checked={draft.autoWrite}
            onChange={(event) => setDraft((prev) => (prev ? { ...prev, autoWrite: event.target.checked } : prev))}
          />
          <span>{copy.autoWrite}</span>
        </label>
        <label className="memory-toggle">
          <input
            type="checkbox"
            checked={draft.promptInjection}
            onChange={(event) =>
              setDraft((prev) => (prev ? { ...prev, promptInjection: event.target.checked } : prev))
            }
          />
          <span>{copy.promptInjection}</span>
        </label>
        <label className="memory-toggle">
          <input
            type="checkbox"
            checked={draft.llmExtractEnabled}
            onChange={(event) =>
              setDraft((prev) => (prev ? { ...prev, llmExtractEnabled: event.target.checked } : prev))
            }
          />
          <span>{copy.llmExtractFallback}</span>
        </label>
      </div>

      <div className="memory-settings__numbers">
        <label className="field">
          <span className="field-label">{copy.tokenBudget}</span>
          <input
            className="glass-input"
            type="number"
            min={200}
            max={6000}
            value={draft.tokenBudget}
            onChange={(event) =>
              setDraft((prev) =>
                prev ? { ...prev, tokenBudget: asNumber(event.target.value, prev.tokenBudget) } : prev
              )
            }
          />
        </label>
        <label className="field">
          <span className="field-label">{copy.retrievalTopK}</span>
          <input
            className="glass-input"
            type="number"
            min={1}
            max={50}
            value={draft.retrievalTopK}
            onChange={(event) =>
              setDraft((prev) =>
                prev ? { ...prev, retrievalTopK: asNumber(event.target.value, prev.retrievalTopK) } : prev
              )
            }
          />
        </label>
      </div>

      <div className="memory-settings__actions">
        <button type="button" className="tech-btn tech-btn-primary" onClick={submit} disabled={isLoading}>
          {copy.saveSettings}
        </button>
      </div>
    </div>
  );
}
