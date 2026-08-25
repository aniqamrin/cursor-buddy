import { useState } from "react";
import { api } from "../../services/bridge";
import type { SettingsDto } from "../../types";
import { Section, Toggle } from "./MainShell";

export function MemoryTab({
  settings,
  onChanged,
}: {
  settings: SettingsDto;
  onChanged: () => void;
}) {
  const [busy, setBusy] = useState(false);

  async function update(patch: Partial<SettingsDto>) {
    setBusy(true);
    try {
      await api.saveSettings({ ...settings, ...patch });
      onChanged();
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="scroll-thin h-full overflow-y-auto px-6 py-5">
      <h1 className="mb-1 text-sm font-semibold">Memory</h1>
      <p className="mb-6 text-xs text-zinc-500">
        Buddy is deliberately forgetful by default. Nothing about you is retained between
        sessions unless you ask for it.
      </p>

      <Section title="Session memory" subtitle="Always on. Holds the current conversation and the context of your last activation so you can say “why did that happen?”. Cleared when you clear history or quit.">
        <div className="rounded-xl border border-white/[0.07] bg-white/[0.02] p-3 text-[13px] text-zinc-300">
          Session memory is active for this conversation only.
        </div>
      </Section>

      <Section title="Long-term memory" subtitle="Optional. Lets Buddy remember preferences across sessions (response style, learning level). Off by default and never built silently.">
        <Toggle
          checked={settings.memory_enabled}
          onChange={(v) => void update({ memory_enabled: v })}
          label="Enable long-term memory"
          description={
            settings.memory_enabled
              ? "On — preferences may be remembered. You can disable anytime; stored memories are deleted with it."
              : "Preference saved. Recall features arrive in a later update; nothing is remembered until then."
          }
        />
      </Section>

      <Section title="Your data" subtitle="Everything lives in %APPDATA%\\com.cursorbuddy.app\\cursor-buddy.db.">
        <div className="rounded-xl border border-white/[0.07] bg-white/[0.02] p-3 text-[12px] leading-relaxed text-zinc-400">
          Screenshots are never written to disk. Conversation history can be wiped from
          History → Clear all history at any time.
        </div>
      </Section>

      {busy && <span className="text-xs text-zinc-500">Saving…</span>}
    </div>
  );
}
