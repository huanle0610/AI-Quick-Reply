import { invoke } from "@tauri-apps/api/core";
import type { AppConfig } from "./types";

export function loadConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("load_config");
}

export function saveConfig(config: AppConfig): Promise<void> {
  return invoke<void>("save_config", { config });
}

export function sendPhrase(text: string): Promise<void> {
  return invoke<void>("send_phrase", { text });
}
