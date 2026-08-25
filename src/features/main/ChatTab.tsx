import { useCallback } from "react";
import { api } from "../../services/bridge";
import { useBuddy } from "../../stores/buddy";
import { Composer, MessageList } from "../../components/chat";

export function ChatTab() {
  const paused = useBuddy((s) => s.paused);

  const submit = useCallback(
    (text: string) => {
      if (paused) return;
      useBuddy.getState().pushLocalUserMessage(text);
      api.send(text).catch((err) => useBuddy.getState().fail(String(err)));
    },
    [paused],
  );

  return (
    <div className="flex h-full flex-col">
      <header className="flex h-12 shrink-0 items-center justify-between border-b border-white/[0.06] px-5">
        <h1 className="text-sm font-semibold">Chat</h1>
        <span className="text-[11px] text-zinc-500">
          Context is captured when you press the shortcut
        </span>
      </header>
      <MessageList />
      <Composer onSubmit={submit} hint="Enter ↵ send · Shift+Enter newline" />
    </div>
  );
}
