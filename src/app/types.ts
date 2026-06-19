export type Phrase = {
  id: string;
  label: string;
  text: string;
  enabled: boolean;
};

export type AppConfig = {
  hotkey: string;
  phrases: Phrase[];
};
