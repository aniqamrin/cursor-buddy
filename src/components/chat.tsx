import { useEffect, useRef, useState } from "react";
import { SendHorizontal } from "lucide-react";
import { useBuddy, type ChatMsg } from "../stores/buddy";
import { RichText } from "./ui";

export function MessageList({ compact }: { compact?: boolean }) {
  const messages = useBuddy((s) => s.messages);
  const streamingText = useBuddy((s) => s.streamingText);
  const status = useBuddy((s) => s.status);
  const error = useBuddy((s) => s.error);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [messages.length, streamingText, status]);

  return (
    <div className="scroll-thin min-h-0 flex-1 space-y-2.5 overflow-y-auto px-3 py-2">
      <TurnList
        messages={messages}
        streamingText={streamingText}
        thinking={status === "thinking"}
        error={error}
        compact={compact}
      />
      <div ref={bottomRef} />
    </div>
  );
}

function TurnList({
  messages,
  streamingText,
  thinking,
  error,
  compact,
}: {
  messages: ChatMsg[];
  streamingText: string;
  thinking: boolean;
  error: string | null;
  compact?: boolean;
}) {
  if (messages.length === 0 && !streamingText && statusIdle(thinking)) {
    return (
      <div className={`text-zinc-500 ${compact ? "py-4 text-center text-[12px]" : "py-10 text-center"}`}>
        {compact ? "What are you working on?" : "Start a conversation with your buddy."}
      </div>
    );
  }
  return (
    <>
      {messages.map((m, i) => (
        <Bubble key={i} msg={m} compact={compact} />
      ))}
      {(thinking || streamingText) && (
        <div className="pr-2 text-[13px] text-zinc-200">
          {streamingText ? (
            <span>
              <RichText text={streamingText} />
              <span className="cb-caret ml-[1px] inline-block h-[14px] w-[7px] translate-y-[2px] bg-violet-400" />
            </span>
          ) : (
            <span className="cb-pulse text-zinc-400">Thinking…</span>
          )}
        </div>
      )}
      {error && (
        <div className="rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-2 text-[12px] text-red-300">
          {error}
        </div>
      )}
    </>
  );
}

function statusIdle(thinking: boolean) {
  return !thinking;
}

function Bubble({ msg, compact }: { msg: ChatMsg; compact?: boolean }) {
  if (msg.role === "user") {
    return (
      <div className="flex justify-end">
        <div className="max-w-[85%] rounded-xl rounded-br-sm bg-violet-600/90 px-3 py-1.5 text-[13px] text-white shadow-sm">
          {msg.content}
        </div>
      </div>
    );
  }
  return (
    <div className={`${compact ? "" : "max-w-[92%]"} pr-2 text-[13px] text-zinc-200`}>
      <RichText text={msg.content} />
    </div>
  );
}

export function Composer({
  onSubmit,
  autoFocusKey,
  hint,
}: {
  onSubmit: (text: string) => void;
  autoFocusKey?: number;
  hint?: string;
}) {
  const [value, setValue] = useState("");
  const inputRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, [autoFocusKey]);

  const submit = () => {
    const text = value.trim();
    if (!text) return;
    setValue("");
    onSubmit(text);
  };

  return (
    <div className="p-2 pt-0">
      <div className="flex items-end gap-1 rounded-xl border border-white/10 bg-white/[0.05] p-1.5 transition-colors focus-within:border-violet-400/60">
        <textarea
          ref={inputRef}
          rows={1}
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              submit();
            }
            e.stopPropagation();
          }}
          placeholder="Ask anything…"
          className="scroll-thin max-h-24 flex-1 resize-none bg-transparent px-1.5 py-1 text-[13px] text-zinc-100 outline-none placeholder:text-zinc-500"
        />
        <button
          onClick={submit}
          disabled={!value.trim()}
          className="rounded-lg bg-violet-600 p-1.5 text-white transition hover:bg-violet-500 disabled:opacity-30"
          title="Send"
        >
          <SendHorizontal className="h-4 w-4" />
        </button>
      </div>
      {hint && (
        <div className="mt-1.5 text-center text-[10px] leading-none text-zinc-500">{hint}</div>
      )}
    </div>
  );
}
