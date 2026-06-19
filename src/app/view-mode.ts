export type ViewMode = "compact" | "full";

export function initialViewMode(): ViewMode {
  return "compact";
}

export function expandViewMode(_current: ViewMode): ViewMode {
  return "full";
}

export function compactViewMode(_current: ViewMode): ViewMode {
  return "compact";
}
