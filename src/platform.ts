// Small differences between the Windows and Mac versions of the interface.
export const IS_MAC = typeof navigator !== "undefined" && /Mac/i.test(navigator.platform || navigator.userAgent);

export const OS = IS_MAC
  ? { name: "macOS", installed: "Installed on this Mac", licensed: "Licensed with macOS", startup: "Open at login", mod: "Cmd", undo: "⌘Z" }
  : { name: "Windows", installed: "Installed in Windows", licensed: "Licensed with Windows", startup: "Start with Windows", mod: "Ctrl", undo: "Ctrl+Z" };
