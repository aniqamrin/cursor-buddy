import { useEffect, useState } from "react";
import { api } from "../../services/bridge";
import type { ApiKeyStatus, SettingsDto } from "../../types";
import { Section, Toggle } from "./MainShell";

export function SettingsTab({
  settings,
  onChanged,
}: {
  settings: SettingsDto;
  onChanged: () => void;
}) {
  const [draft, setDraft] = useState<SettingsDto>(settings);
  const [keyDraft, setKeyDraft] = useState("");
  const [keyStatus, setKeyStatus] = useState<ApiKeyStatus | null>(null);
  const [notice, setNotice] = useState<{ kind: "ok" | "err"; text: string } | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => setDraft(settings), [settings]);
  useEffect(() => {
    api.apiKeyStatus().then(setKeyStatus);
  }, []);

  async function saveAll() {
    setBusy(true);
    setNotice(null);
    try {
      await api.saveSettings(draft);
      setNotice({ kind: "ok", text: "Settings saved." });
      onChanged();
    } catch (e) {
      setNotice({ kind: "err", text: String(e) });
    } finally {
      setBusy(false);
    }
  }

  async function saveKey() {
    setNotice(null);
    try {
      const status = await api.setApiKey(keyDraft.trim());
      setKeyStatus(status);
      setKeyDraft("");
      // The backend may have auto-switched the model to match the provider.
      const fresh = await api.getSettings();
      setDraft(fresh);
      setNotice({ kind: "ok", text: "API key stored — model auto-configured." });
    } catch (e) {
      setNotice({ kind: "err", text: String(e) });
    }
  }

  async function removeKey() {
    const status = await api.removeApiKey();
    setKeyStatus(status);
    setNotice({ kind: "ok", text: "Stored API key removed." });
  }

  return (
    <div className="scroll-thin h-full overflow-y-auto px-6 py-5">
      <h1 className="mb-6 text-sm font-semibold">Settings</h1>

      {/* AI */}
      <Section title="AI" subtitle="Your key is stored locally in the app database and used only by the Rust core — it never enters the web UI bundle.">
        <div className="space-y-3">
          <div className="rounded-xl border border-white/[0.07] bg-white/[0.02] p-3">
            <label className="block text-[12px] font-medium text-zinc-300">OpenAI API key</label>
            <div className="mt-2 flex gap-2">
              <input
                type="password"
                value={keyDraft}
                onChange={(e) => setKeyDraft(e.target.value)}
                placeholder={
                  keyStatus?.configured ? `Stored (${keyStatus.source}): ${keyStatus.masked}` : "sk-…"
                }
                className="flex-1 rounded-lg border border-white/10 bg-black/30 px-3 py-1.5 text-[13px] outline-none transition focus:border-violet-400/60"
              />
              <button
                onClick={() => void saveKey()}
                disabled={!keyDraft.trim()}
                className="rounded-lg bg-violet-600 px-3 py-1.5 text-[12px] font-medium text-white hover:bg-violet-500 disabled:opacity-40"
              >
                Save key
              </button>
              {keyStatus?.source === "stored" && (
                <button
                  onClick={() => void removeKey()}
                  className="rounded-lg border border-white/10 px-3 py-1.5 text-[12px] text-zinc-400 hover:text-red-300"
                >
                  Remove
                </button>
              )}
            </div>
            <p className="mt-2 text-[11px] text-zinc-500">
              Alternatively set the OPENAI_API_KEY environment variable before launching.
            </p>
          </div>

          <div className="rounded-xl border border-white/[0.07] bg-white/[0.02] p-3">
            <label className="block text-[12px] font-medium text-zinc-300">Model</label>
            <input
              value={draft.model}
              onChange={(e) => setDraft({ ...draft, model: e.target.value })}
              placeholder="gemini-3.6-flash"
              className="mt-2 w-full rounded-lg border border-white/10 bg-black/30 px-3 py-1.5 text-[13px] outline-none transition focus:border-violet-400/60"
            />
            <p className="mt-2 text-[11px] text-zinc-500">
              OpenAI: gpt-4o-mini · Gemini: gemini-3.6-flash (auto-detected by prefix).
              Gemini keys come from Google AI Studio.
            </p>
          </div>
        </div>
      </Section>

      {/* General */}
      <Section title="General">
        <div className="space-y-3">
          <div className="rounded-xl border border-white/[0.07] bg-white/[0.02] p-3">
            <label className="block text-[12px] font-medium text-zinc-300">
              Global activation shortcut
            </label>
            <input
              value={draft.hotkey}
              onChange={(e) => setDraft({ ...draft, hotkey: e.target.value })}
              spellCheck={false}
              className="mt-2 w-full rounded-lg border border-white/10 bg-black/30 px-3 py-1.5 font-mono text-[13px] outline-none transition focus:border-violet-400/60"
            />
            <p className="mt-2 text-[11px] text-zinc-500">
              Format like Control+Shift+Space. Works system-wide; invalid combos are rejected on
              save.
            </p>
          </div>

          <Toggle
            checked={draft.autostart}
            onChange={(v) => setDraft({ ...draft, autostart: v })}
            label="Start with Windows"
            description="Launch Cursor Buddy automatically when you sign in."
          />
        </div>
      </Section>

      {/* Context */}
      <Section title="Context" subtitle="What Buddy may look at when it activates. Context is captured only at the moment you summon it — never continuously.">
        <div className="space-y-3">
          <Toggle
            checked={draft.activity_context_enabled}
            onChange={(v) => setDraft({ ...draft, activity_context_enabled: v })}
            label="Active application & window title"
            description="Lets Buddy know which app and window you are in."
          />
          <Toggle
            checked={draft.screen_context_enabled}
            onChange={(v) => setDraft({ ...draft, screen_context_enabled: v })}
            label="Screen capture & OCR context"
            description="Permission stored now; visual understanding arrives in an upcoming update."
          />
        </div>
      </Section>

      <div className="sticky bottom-0 flex items-center gap-3 border-t border-white/[0.06] bg-[#0b0c11]/95 py-3 backdrop-blur">
        <button
          onClick={() => void saveAll()}
          disabled={busy}
          className="rounded-lg bg-violet-600 px-4 py-1.5 text-[13px] font-medium text-white transition hover:bg-violet-500 disabled:opacity-40"
        >
          {busy ? "Saving…" : "Save settings"}
        </button>
        {notice && (
          <span className={`text-[12px] ${notice.kind === "ok" ? "text-emerald-400" : "text-red-400"}`}>
            {notice.text}
          </span>
        )}
      </div>
    </div>
  );
}
