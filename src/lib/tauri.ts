import { invoke, isTauri } from "@tauri-apps/api/core";

export function isDesktop(): boolean {
  return isTauri();
}

export function getVersion(): Promise<string> {
  return invoke<string>("version");
}
