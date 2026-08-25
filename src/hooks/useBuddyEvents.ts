import { useEffect } from "react";
import { events } from "../services/bridge";
import { useBuddy } from "../stores/buddy";

/**
 * Subscribes this window to all backend event streams and routes them
 * into the shared store. Safe to mount in both windows.
 */
export function useBuddyEvents() {
  useEffect(() => {
    const unlisteners = [
      events.activate((p) => useBuddy.getState().activate(p)),
      events.token((delta) => useBuddy.getState().appendDelta(delta)),
      events.done((p) => useBuddy.getState().finishStream(p.content, p.conversation_id)),
      events.error((e) => useBuddy.getState().fail(e.message)),
      events.generationStarted(() => useBuddy.getState().generationStarted()),
      events.pauseChanged((paused) => useBuddy.getState().setPaused(paused)),
      events.permissionChanged(
        (level) => useBuddy.getState().setLevel(level as "observe" | "guide" | "assist" | "autopilot"),
      ),
      events.screenText(({ chars }) => useBuddy.getState().setScreenChars(chars)),
    ];

    return () => {
      for (const promise of unlisteners) {
        void promise.then((unlisten) => unlisten());
      }
    };
  }, []);
}
