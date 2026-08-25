import { useEffect, useState } from "react";
import { Trash2 } from "lucide-react";
import { api } from "../../services/bridge";
import { useBuddy } from "../../stores/buddy";
import type { ConversationSummary, MessageRow } from "../../types";

export function HistoryTab() {
  const [convos, setConvos] = useState<ConversationSummary[]>([]);
  const [selected, setSelected] = useState<MessageRow[] | null>(null);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [confirmingClear, setConfirmingClear] = useState(false);

  useEffect(() => {
    void reload();
  }, []);

  async function reload() {
    setConvos(await api.listConversations());
  }

  async function open(id: number) {
    const rows = await api.getMessages(id);
    setSelected(rows);
    setSelectedId(id);
  }

  async function clearAll() {
    await api.clearHistory();
    setConfirmingClear(false);
    setSelected(null);
    setSelectedId(null);
    useBuddy.getState().seedMessages([]);
    await reload();
  }

  return (
    <div className="scroll-thin h-full overflow-y-auto px-6 py-5">
      <header className="mb-4 flex items-center justify-between">
        <div>
          <h1 className="text-sm font-semibold">History</h1>
          <p className="text-xs text-zinc-500">
            Conversations are stored locally in SQLite. Nothing leaves your machine except the
            messages you send to the model.
          </p>
        </div>
        {convos.length > 0 &&
          (confirmingClear ? (
            <span className="flex items-center gap-2 text-xs">
              <span className="text-zinc-400">Delete all conversations?</span>
              <button
                onClick={() => void clearAll()}
                className="rounded-md bg-red-600/80 px-2.5 py-1 font-medium text-white hover:bg-red-600"
              >
                Delete everything
              </button>
              <button onClick={() => setConfirmingClear(false)} className="text-zinc-500 hover:text-zinc-300">
                Cancel
              </button>
            </span>
          ) : (
            <button
              onClick={() => setConfirmingClear(true)}
              className="flex items-center gap-1.5 rounded-lg border border-white/10 px-3 py-1.5 text-[12px] text-zinc-400 transition hover:border-red-500/40 hover:text-red-300"
            >
              <Trash2 className="h-3.5 w-3.5" /> Clear all history
            </button>
          ))}
      </header>

      {convos.length === 0 ? (
        <p className="mt-10 text-center text-sm text-zinc-500">
          No conversations yet.
        </p>
      ) : (
        <div className="grid gap-2 md:grid-cols-2 lg:grid-cols-3">
          {convos.map((c) => (
            <button
              key={c.id}
              onClick={() => void open(c.id)}
              className={`rounded-xl border p-3 text-left transition ${
                selectedId === c.id
                  ? "border-violet-400/50 bg-violet-500/10"
                  : "border-white/[0.07] bg-white/[0.02] hover:border-white/20"
              }`}
            >
              <p className="truncate text-[13px] text-zinc-200">{c.title}</p>
              <p className="mt-1 text-[11px] text-zinc-500">
                {c.message_count} messages · {new Date(c.created_at).toLocaleString()}
              </p>
            </button>
          ))}
        </div>
      )}

      {selected && selectedId !== null && (
        <section className="mt-6 rounded-xl border border-white/[0.07] bg-white/[0.02] p-4">
          <h2 className="mb-3 text-xs font-semibold uppercase tracking-wider text-zinc-500">
            Conversation #{selectedId}
          </h2>
          <div className="space-y-3">
            {selected.map((m) => (
              <div key={m.id} className={m.role === "user" ? "text-right" : ""}>
                <span
                  className={`inline-block max-w-[85%] rounded-xl px-3 py-1.5 text-left text-[12px] ${
                    m.role === "user"
                      ? "bg-violet-600/70 text-white"
                      : "bg-white/[0.05] text-zinc-200"
                  }`}
                >
                  {m.content}
                </span>
              </div>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}
