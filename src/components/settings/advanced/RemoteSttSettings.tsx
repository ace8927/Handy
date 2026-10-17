import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type SttMode } from "@/bindings";
import { useSettings } from "../../../hooks/useSettings";
import { SettingContainer } from "../../ui/SettingContainer";

/** The only added UI: a Local/Remote selector and remote connection settings. */
export const RemoteSttSettings: React.FC = () => {
  const { t } = useTranslation();
  const { settings, refreshSettings } = useSettings();
  const [mode, setMode] = useState<SttMode>("local");
  const [url, setUrl] = useState("");
  const [model, setModel] = useState("whisper-1");
  const [apiKey, setApiKey] = useState("");
  const [saving, setSaving] = useState(false);
  const [feedback, setFeedback] = useState("");

  useEffect(() => {
    if (!settings) return;
    setMode(settings.stt_mode ?? "local");
    setUrl(settings.remote_stt_url ?? "");
    setModel(settings.remote_stt_model ?? "whisper-1");
    setApiKey(settings.remote_stt_api_keys?.server ?? "");
  }, [settings]);

  const save = async () => {
    setSaving(true);
    setFeedback("");
    try {
      const result = await commands.configureRemoteStt(
        mode,
        url.trim(),
        model.trim(),
        apiKey,
      );
      if (result.status === "error") {
        throw new Error(result.error);
      }
      await refreshSettings();
      setFeedback(t("settings.remoteStt.saved", { defaultValue: "Saved" }));
    } catch (error) {
      setFeedback(error instanceof Error ? error.message : String(error));
    } finally {
      setSaving(false);
    }
  };

  return (
    <SettingContainer
      title={t("settings.remoteStt.title", { defaultValue: "STT engine" })}
      description={t("settings.remoteStt.description", {
        defaultValue: "Use the built-in model or an OpenAI-compatible STT server.",
      })}
      descriptionMode="inline"
      grouped={true}
      layout="stacked"
    >
      <div className="space-y-2 text-sm">
        <select
          aria-label={t("settings.remoteStt.title", { defaultValue: "STT engine" })}
          value={mode}
          onChange={(e) => { setMode(e.target.value as SttMode); setFeedback(""); }}
          className="w-full rounded-md border border-mid-gray/30 bg-background px-3 py-2"
        >
          <option value="local">{t("settings.remoteStt.local", { defaultValue: "Local" })}</option>
          <option value="remote">{t("settings.remoteStt.remote", { defaultValue: "Remote STT" })}</option>
        </select>
        {mode === "remote" && (
          <div className="space-y-2">
            <label className="block">
              <span className="block text-xs mb-1">{t("settings.remoteStt.url", { defaultValue: "Server URL" })}</span>
              <input
                type="url"
                spellCheck={false}
                autoComplete="off"
                value={url}
                onChange={(e) => setUrl(e.target.value)}
                placeholder="http://192.168.1.20:8000/v1"
                className="w-full rounded-md border border-mid-gray/30 bg-background px-3 py-2"
              />
            </label>
            <label className="block">
              <span className="block text-xs mb-1">{t("settings.remoteStt.model", { defaultValue: "Model" })}</span>
              <input
                type="text"
                spellCheck={false}
                value={model}
                onChange={(e) => setModel(e.target.value)}
                placeholder="whisper-1"
                className="w-full rounded-md border border-mid-gray/30 bg-background px-3 py-2"
              />
            </label>
            <label className="block">
              <span className="block text-xs mb-1">{t("settings.remoteStt.apiKey", { defaultValue: "API Key (optional)" })}</span>
              <input
                type="password"
                autoComplete="off"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                className="w-full rounded-md border border-mid-gray/30 bg-background px-3 py-2"
              />
            </label>
          </div>
        )}
        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={() => { void save(); }}
            disabled={saving}
            className="rounded-md border border-mid-gray/30 bg-background px-4 py-2 hover:bg-mid-gray/10 disabled:opacity-50"
          >
            {t("settings.remoteStt.save", { defaultValue: "Save STT settings" })}
          </button>
          {feedback && <span role="status" className="text-xs text-mid-gray break-all">{feedback}</span>}
        </div>
      </div>
    </SettingContainer>
  );
};
