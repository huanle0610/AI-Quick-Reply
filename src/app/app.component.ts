import { Component, OnDestroy, OnInit, signal } from "@angular/core";
import { FormsModule } from "@angular/forms";
import { listen } from "@tauri-apps/api/event";
import { addPhrase, deletePhrase, movePhrase, updatePhrase } from "./phrase-state";
import { expandViewMode, initialViewMode, type ViewMode } from "./view-mode";
import { loadConfig, saveConfig, sendPhrase, setWindowMode } from "./api";
import type { AppConfig, Phrase } from "./types";

@Component({
  selector: "app-root",
  imports: [FormsModule],
  templateUrl: "./app.component.html",
  styleUrl: "./app.component.css",
})
export class AppComponent implements OnInit, OnDestroy {
  readonly config = signal<AppConfig | null>(null);
  readonly managerOpen = signal(false);
  readonly helpOpen = signal(false);
  readonly status = signal("Loading...");
  readonly viewMode = signal<ViewMode>(initialViewMode());

  newLabel = "";
  newText = "";

  private unlistenViewMode: (() => void) | null = null;

  async ngOnInit(): Promise<void> {
    try {
      this.unlistenViewMode = await listen<ViewMode>("view-mode", (event) => {
        this.viewMode.set(event.payload);
        if (event.payload === "compact") {
          this.managerOpen.set(false);
          this.helpOpen.set(false);
        }
      });
      this.config.set(await loadConfig());
      this.status.set("Ready");
    } catch (error) {
      this.status.set(this.messageFromError(error));
    }
  }

  ngOnDestroy(): void {
    this.unlistenViewMode?.();
  }

  enabledPhrases(): Phrase[] {
    return this.config()?.phrases.filter((phrase) => phrase.enabled) ?? [];
  }

  async expandToFull(): Promise<void> {
    this.viewMode.set(expandViewMode(this.viewMode()));
    try {
      await setWindowMode("full");
    } catch (error) {
      this.status.set(this.messageFromError(error));
    }
  }

  async send(phrase: Phrase): Promise<void> {
    this.status.set(`Sending ${phrase.label}...`);
    try {
      await sendPhrase(phrase.text);
      this.status.set("Pasted");
    } catch (error) {
      this.status.set(this.messageFromError(error));
    }
  }

  toggleManager(): void {
    const next = !this.managerOpen();
    this.managerOpen.set(next);
    if (next) {
      this.helpOpen.set(false);
    }
  }

  toggleHelp(): void {
    const next = !this.helpOpen();
    this.helpOpen.set(next);
    if (next) {
      this.managerOpen.set(false);
    }
  }

  async add(): Promise<void> {
    const label = this.newLabel.trim();
    const text = this.newText.trim();
    const current = this.config();
    if (!current || !label || !text) {
      return;
    }

    await this.persist(addPhrase(current, label, text));
    this.newLabel = "";
    this.newText = "";
  }

  async updateCtrlHoldSeconds(value: string | number): Promise<void> {
    const current = this.config();
    if (!current) {
      return;
    }

    const seconds = Math.max(1, Math.min(30, Number(value) || 5));
    await this.persist({ ...current, ctrlHoldSeconds: seconds });
  }

  async updateAutoStartEnabled(enabled: boolean): Promise<void> {
    const current = this.config();
    if (!current) {
      return;
    }

    await this.persist({ ...current, autoStartEnabled: enabled });
  }

  async update(id: string, patch: Partial<Omit<Phrase, "id">>): Promise<void> {
    const current = this.config();
    if (!current) {
      return;
    }
    await this.persist(updatePhrase(current, id, patch));
  }

  async remove(id: string): Promise<void> {
    const current = this.config();
    if (!current) {
      return;
    }
    await this.persist(deletePhrase(current, id));
  }

  async move(id: string, offset: -1 | 1): Promise<void> {
    const current = this.config();
    if (!current) {
      return;
    }
    await this.persist(movePhrase(current, id, offset));
  }

  private async persist(next: AppConfig): Promise<void> {
    this.config.set(next);
    await saveConfig(next);
    this.status.set("Saved");
  }

  private messageFromError(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }
}



