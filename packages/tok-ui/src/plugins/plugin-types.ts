/**
 * TypesetOK (TOK) - Official Plugin API Types & Manifest Specification
 *
 * Defines the public API surface exposed to TypesetOK plugins (JS & TS).
 * Plugins receive a sandboxed `TokPluginContext` providing high-performance,
 * capability-checked access to the document, Hebrew typography tools, UI, canvas,
 * and lifecycle events.
 */

export type PluginPermission =
  | 'document:read'      // Read document structure, story paragraphs, word count
  | 'document:write'     // Modify text, insert/delete paragraphs, apply styles
  | 'ui:commands'        // Register commands in the Command Palette
  | 'ui:notifications'   // Display toast notifications and alerts
  | 'ui:modals'          // Open custom modal dialogs
  | 'canvas:read'        // Inspect pages, spreads, zoom level, view mode
  | 'canvas:navigate'    // Scroll to pages, change zoom, toggle layout guides
  | 'storage:export'     // Trigger PDF or HTML export
  | 'network:fetch';     // Outgoing HTTP requests (strictly blocked by default)

export interface PluginManifest {
  /** Unique plugin identifier (reverse domain notation, e.g. "org.user.my-plugin") */
  id: string;
  /** Human-readable display name */
  name: string;
  /** Semantic version string (e.g. "1.0.0") */
  version: string;
  /** Short description of the plugin's functionality */
  description: string;
  /** Author name or organization */
  author?: string;
  /** Entry point file relative to the plugin directory (e.g. "index.ts" or "index.js") */
  main: string;
  /** Whether the plugin is enabled by default */
  enabled?: boolean;
  /** Declared security permissions required by the plugin */
  permissions?: PluginPermission[];
  /** Minimum compatible TypesetOK version */
  minTokVersion?: string;
  /** Official project or repository homepage */
  homepage?: string;
}

export interface PluginCommand {
  /** Unique command identifier */
  id: string;
  /** Command title displayed in the Command Palette */
  title: string;
  /** Category grouping (e.g. "טיפוגרפיה", "עריכה", "כלים") */
  category: string;
  /** Optional keyboard shortcut hint (e.g. "Ctrl+Alt+H") */
  shortcut?: string;
  /** Optional icon identifier */
  icon?: string;
  /** Action executed when the command is triggered */
  action: () => void | Promise<void>;
}

export interface Disposable {
  dispose: () => void;
}

export interface StoryParagraph {
  id: string;
  styleId: string;
  text: string;
}

export interface DocumentStats {
  wordCount: number;
  charCount: number;
  paragraphCount: number;
}

export interface HolyNameMatch {
  name: string;
  offset: number;
  len: number;
}

/**
 * High-performance Document & Story Editor API.
 * Mutations can be batched with `batch()` to avoid repeated re-render overhead.
 */
export interface TokDocumentAPI {
  /** Returns a snapshot of all paragraphs in the current story */
  getStory(): StoryParagraph[];

  /**
   * Replaces the entire story with new paragraphs in an atomic operation.
   * Requires 'document:write' permission.
   */
  loadStory(paragraphs: StoryParagraph[]): void;

  /** Returns a single paragraph by its unique ID */
  getParagraph(id: string): StoryParagraph | undefined;

  /**
   * Updates text or style for a specific paragraph.
   * Requires 'document:write' permission.
   */
  updateParagraph(id: string, text: string, styleId?: string): boolean;

  /**
   * Inserts a new paragraph at the specified zero-based index.
   * Requires 'document:write' permission.
   */
  insertParagraph(index: number, text: string, styleId?: string): string;

  /**
   * Deletes a paragraph by its unique ID.
   * Requires 'document:write' permission.
   */
  deleteParagraph(id: string): boolean;

  /** Returns currently selected text in the active editor */
  getSelectedText(): string;

  /**
   * Replaces the current selection with new text.
   * Requires 'document:write' permission.
   */
  replaceSelection(text: string): void;

  /** Returns the title of the active document */
  getDocumentTitle(): string;

  /** Computes instantaneous word, character, and paragraph statistics */
  getStats(): DocumentStats;

  /**
   * Runs multiple text modifications in a single atomic batch,
   * suspending intermediate DOM re-renders and debounced word-count updates.
   * Guarantees zero performance degradation even with thousands of operations.
   */
  batch(fn: () => void): void;

  /**
   * High-order helper: transforms the text of every paragraph with a pure function.
   * Requires 'document:write' permission.
   */
  transformText(transformFn: (text: string, para: StoryParagraph) => string): void;
}

/**
 * Pure Hebrew Typography & Sacred Prepress Utilities.
 * Executed in-process with zero DOM or IPC overhead (microsecond latency).
 */
export interface TokHebrewAPI {
  /** Converts a number to standard Hebrew Gematria (1 -> א׳, 15 -> ט״ו, 270 -> ע״ר, 5784 -> ה׳תשפ״ד) */
  toHebrewGematria(num: number): string;

  /** Strips Hebrew vowel points (Niqqud) while preserving consonants and cantillation */
  stripNiqqud(text: string): string;

  /** Strips Hebrew cantillation marks (Te'amim) while preserving vowels (Niqqud) */
  stripTaamim(text: string): string;

  /** Strips all Hebrew combining marks (both Niqqud and Te'amim) */
  stripAllMarks(text: string): string;

  /** Normalizes Hebrew combining mark order according to Israeli Standard SI 6100 (ת"י 6100) */
  normalizeHebrew(text: string): string;

  /** Detects all occurrences of holy divine names (יהוה, אלהים, אהיה, שדי, צבאות, אדני) */
  findHolyNames(text: string): HolyNameMatch[];

  /** Converts ASCII quotes (" and ') inside Hebrew text to typographic Gershayim (״) and Geresh (׳) */
  normalizeQuotes(text: string): string;

  /** Tests whether a string contains Hebrew characters */
  isHebrew(text: string): boolean;
}

/**
 * UI & User Interaction API.
 */
export interface TokUIAPI {
  /** Registers a command into the TypesetOK Command Palette (Cmd/Ctrl+K) */
  registerCommand(cmd: PluginCommand): Disposable;

  /** Displays a toast message at the bottom of the workbench */
  showToast(msg: string, isError?: boolean): void;

  /** Returns the active workspace view mode ('canvas' | 'story' | 'split') */
  getViewMode(): 'canvas' | 'story' | 'split';

  /** Switches the workspace view mode */
  setViewMode(mode: 'canvas' | 'story' | 'split'): void;

  /** Returns current theme preset name */
  getTheme(): string;

  /** Returns current UI language ('he' | 'en') */
  getLanguage(): 'he' | 'en';

  /**
   * Opens a sandboxed modal dialog with custom HTML content.
   * Requires 'ui:modals' permission. Returns a Disposable to close the modal.
   */
  openModal(title: string, render: (container: HTMLElement) => void | (() => void)): Disposable;
}

/**
 * Canvas & Page Inspection API.
 */
export interface TokCanvasAPI {
  /** Returns total count of pages in the document */
  getPageCount(): number;

  /** Returns index of the currently active page (0-based) */
  getActivePageIndex(): number;

  /** Smoothly scrolls the canvas to the specified page index */
  scrollToPage(index: number): void;

  /** Returns current canvas zoom percentage (e.g. 100 for 100%) */
  getZoom(): number;

  /** Sets canvas zoom level */
  setZoom(percent: number): void;

  /** Toggles margins guide overlay */
  toggleMarginsGuide(): void;

  /** Toggles baseline grid overlay */
  toggleBaselineGuide(): void;
}

/**
 * Lifecycle Event Listener API.
 */
export interface TokEventsAPI {
  /** Subscribes to an application event. Returns a Disposable to unsubscribe. */
  on(event: 'text-change', listener: (paraId: string, text: string) => void): Disposable;
  on(event: 'page-change', listener: (pageIndex: number) => void): Disposable;
  on(event: 'view-change', listener: (viewMode: 'canvas' | 'story' | 'split') => void): Disposable;
  on(event: string, listener: (...args: any[]) => void): Disposable;
}

/**
 * Runtime Environment & Security Status API.
 */
export interface TokEnvAPI {
  /** Current TypesetOK version string */
  readonly appVersion: string;

  /**
   * Whether Safe Mode (מצב בטוח) is active.
   * In Safe Mode, untrusted third-party plugins are disabled to prevent damage.
   */
  readonly isSafeMode: boolean;

  /**
   * Whether Safer (מערכת סייפר) or managed institutional workstation is detected.
   * When true, the application runs in hardened local-only offline mode.
   */
  readonly isSaferActive: boolean;

  /** Operating system platform ('win32' | 'darwin' | 'linux') */
  readonly platform: string;
}

/**
 * The official context object passed to a plugin's `activate(tok)` entry point.
 */
export interface TokPluginContext {
  /** The plugin's own manifest */
  readonly manifest: Readonly<PluginManifest>;

  /** Document & text operations */
  readonly document: TokDocumentAPI;

  /** Hebrew typography & sacred name tools */
  readonly hebrew: TokHebrewAPI;

  /** User interface & commands */
  readonly ui: TokUIAPI;

  /** Canvas & layout navigation */
  readonly canvas: TokCanvasAPI;

  /** Application lifecycle events */
  readonly events: TokEventsAPI;

  /** Environment & security status */
  readonly env: TokEnvAPI;

  /** Array of disposables automatically cleaned up when the plugin is deactivated */
  readonly subscriptions: Disposable[];

  /** Shorthand for tok.ui.registerCommand */
  registerCommand(cmd: PluginCommand): Disposable;

  /** Shorthand for tok.ui.showToast */
  showToast(msg: string, isError?: boolean): void;

  /** Registers a custom cleanup function called when the plugin is disabled or reloaded */
  onDeactivate(cleanupFn: () => void): void;
}
