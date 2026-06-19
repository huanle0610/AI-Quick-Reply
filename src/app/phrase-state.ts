import type { AppConfig, Phrase } from "./types";

const slugify = (value: string): string => {
  const slug = value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");

  return slug || `phrase-${Date.now()}`;
};

const uniqueId = (config: AppConfig, label: string): string => {
  const base = slugify(label);
  const existing = new Set(config.phrases.map((phrase) => phrase.id));

  if (!existing.has(base)) {
    return base;
  }

  let index = 2;
  while (existing.has(`${base}-${index}`)) {
    index += 1;
  }
  return `${base}-${index}`;
};

export function addPhrase(config: AppConfig, label: string, text: string): AppConfig {
  const phrase: Phrase = {
    id: uniqueId(config, label),
    label: label.trim(),
    text,
    enabled: true,
  };

  return { ...config, phrases: [...config.phrases, phrase] };
}

export function updatePhrase(config: AppConfig, id: string, patch: Partial<Omit<Phrase, "id">>): AppConfig {
  return {
    ...config,
    phrases: config.phrases.map((phrase) => (phrase.id === id ? { ...phrase, ...patch } : phrase)),
  };
}

export function deletePhrase(config: AppConfig, id: string): AppConfig {
  return { ...config, phrases: config.phrases.filter((phrase) => phrase.id !== id) };
}

export function movePhrase(config: AppConfig, id: string, offset: -1 | 1): AppConfig {
  const index = config.phrases.findIndex((phrase) => phrase.id === id);
  if (index < 0) {
    return config;
  }

  const nextIndex = Math.max(0, Math.min(config.phrases.length - 1, index + offset));
  if (nextIndex === index) {
    return config;
  }

  const phrases = [...config.phrases];
  const [phrase] = phrases.splice(index, 1);
  phrases.splice(nextIndex, 0, phrase);
  return { ...config, phrases };
}

