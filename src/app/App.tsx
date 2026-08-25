import { getCurrentWindow } from "@tauri-apps/api/window";
import { Bubble } from "../features/bubble/Bubble";
import { MainShell } from "../features/main/MainShell";

export default function App() {
  const label = getCurrentWindow().label;
  return label === "bubble" ? <Bubble /> : <MainShell />;
}
