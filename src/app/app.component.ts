import { Component, OnInit, signal } from "@angular/core";
import { FormsModule } from "@angular/forms";
import { addPhrase, deletePhrase, movePhrase, updatePhrase } from "./phrase-state";
import { loadConfig, saveConfig, sendPhrase } from "./api";
import type { AppConfig, Phrase } from "./types";

@Component({
  selector: "app-root",
  imports: [FormsModule],
  templateUrl: "./app.component.html",
  styleUrl: "./app.component.css",
})
export class AppComponent implements OnInit {
  readonly config = signal<AppConfig | null>(null);
  readonly managerOpen = signal(false);
  readonly status = signal("Loading...");

  newLabel = "";
  newText = "";

  async ngOnInit(): Promise<void> {
    try {
      this.config.set(await loadConfig());
      this.status.set("Ready");
    } catch (error) {
      this.status.set(this.messageFromError(error));
    }
  }

  enabledPhrases(): Phrase[] {
    return this.config()?.phrases.filter((phrase) => phrase.enabled) ?? [];
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

