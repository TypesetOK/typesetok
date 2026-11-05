/**
 * Professional SVG Vector Icons for TypesetOK (TOK) Desktop Publishing System
 * Replaces all emojis with clean, pixel-aligned vector iconography.
 */

export type IconName =
  | 'brand'
  | 'folder'
  | 'file'
  | 'search'
  | 'settings'
  | 'info'
  | 'pages'
  | 'flows'
  | 'typography'
  | 'layers'
  | 'canvas'
  | 'split'
  | 'story'
  | 'export'
  | 'globe'
  | 'check'
  | 'close'
  | 'eye'
  | 'eyeOff'
  | 'lock'
  | 'unlock'
  | 'refresh'
  | 'trash'
  | 'plugin'
  | 'accessibility'
  | 'contrast'
  | 'textScale'
  | 'motion'
  | 'keyboard'
  | 'templateTalmud'
  | 'templateBook'
  | 'templateColumns'
  | 'templateBlank'
  | 'sparkle'
  | 'alignJustify'
  | 'warning'
  | 'chevronDown'
  | 'image'
  | 'frame'
  | 'zap'
  | 'ruler'
  | 'undo'
  | 'redo'
  | 'plus'
  | 'minus'
  | 'fit'
  | 'arrowForward'
  | 'link'
  | 'grip'
  | 'appearance'
  | 'alignRight'
  | 'alignCenter'
  | 'alignLeft'
  | 'help'
  | 'upload'
  | 'chevronUp'
  | 'chevronLeft'
  | 'chevronRight'
  | 'bold'
  | 'history'
  | 'edit'
  | 'documentEmpty';

const ICONS_SVG: Record<IconName, string> = {
  undo: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M7.5 11.5 3.5 7.5l4-4"/><path d="M3.5 7.5h9a4 4 0 0 1 0 8H10"/></svg>
  `,
  redo: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="m12.5 11.5 4-4-4-4"/><path d="M16.5 7.5h-9a4 4 0 0 0 0 8H10"/></svg>
  `,
  plus: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><line x1="10" y1="4" x2="10" y2="16"/><line x1="4" y1="10" x2="16" y2="10"/></svg>
  `,
  minus: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><line x1="4" y1="10" x2="16" y2="10"/></svg>
  `,
  fit: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M3 7.5V3h4.5M17 7.5V3h-4.5M3 12.5V17h4.5M17 12.5V17h-4.5"/></svg>
  `,
  arrowForward: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><line x1="4" y1="10" x2="16" y2="10"/><path d="m11 5 5 5-5 5"/></svg>
  `,
  link: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M8.5 11.5a3.5 3.5 0 0 0 5 0l2.5-2.5a3.5 3.5 0 0 0-5-5l-.8.8"/><path d="M11.5 8.5a3.5 3.5 0 0 0-5 0L4 11a3.5 3.5 0 0 0 5 5l.8-.8"/></svg>
  `,
  grip: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><circle cx="8" cy="5" r=".6"/><circle cx="12" cy="5" r=".6"/><circle cx="8" cy="10" r=".6"/><circle cx="12" cy="10" r=".6"/><circle cx="8" cy="15" r=".6"/><circle cx="12" cy="15" r=".6"/></svg>
  `,
  appearance: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><circle cx="10" cy="10" r="6.5"/><path d="M10 3.5a6.5 6.5 0 0 0 0 13z" fill="currentColor"/></svg>
  `,
  alignRight: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><line x1="3.5" y1="5" x2="16.5" y2="5"/><line x1="8.5" y1="8.5" x2="16.5" y2="8.5"/><line x1="3.5" y1="12" x2="16.5" y2="12"/><line x1="8.5" y1="15.5" x2="16.5" y2="15.5"/></svg>
  `,
  alignCenter: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><line x1="3.5" y1="5" x2="16.5" y2="5"/><line x1="6" y1="8.5" x2="14" y2="8.5"/><line x1="3.5" y1="12" x2="16.5" y2="12"/><line x1="6" y1="15.5" x2="14" y2="15.5"/></svg>
  `,
  alignLeft: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><line x1="3.5" y1="5" x2="16.5" y2="5"/><line x1="3.5" y1="8.5" x2="11.5" y2="8.5"/><line x1="3.5" y1="12" x2="16.5" y2="12"/><line x1="3.5" y1="15.5" x2="11.5" y2="15.5"/></svg>
  `,
  help: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><circle cx="10" cy="10" r="7"/><path d="M8 8a2 2 0 1 1 2.8 1.8c-.5.2-.8.7-.8 1.3v.4"/><line x1="10" y1="14" x2="10.01" y2="14"/></svg>
  `,
  upload: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M10 13V3M6 7l4-4 4 4"/><path d="M3.5 13v2.5A1.5 1.5 0 0 0 5 17h10a1.5 1.5 0 0 0 1.5-1.5V13"/></svg>
  `,
  chevronUp: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="m5 12.5 5-5 5 5"/></svg>
  `,
  brand: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <path d="M4 16.5A2.5 2.5 0 0 1 6.5 14H16V3.5A1.5 1.5 0 0 0 14.5 2H6.5A2.5 2.5 0 0 0 4 4.5v12z"/>
      <path d="M16 14v4.5A1.5 1.5 0 0 1 14.5 20H6.5A2.5 2.5 0 0 1 4 17.5"/>
      <line x1="7.5" y1="6" x2="12.5" y2="6"/>
      <line x1="7.5" y1="9.5" x2="12.5" y2="9.5"/>
    </svg>
  `,
  folder: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <path d="M2.5 5.5A1.5 1.5 0 0 1 4 4h3.8a1.5 1.5 0 0 1 1.06.44L10.5 6H16A1.5 1.5 0 0 1 17.5 7.5v8a1.5 1.5 0 0 1-1.5 1.5H4A1.5 1.5 0 0 1 2.5 15.5v-10z"/>
    </svg>
  `,
  file: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <path d="M4.5 3A1.5 1.5 0 0 1 6 1.5h5.5l5 5V17A1.5 1.5 0 0 1 15 18.5H6A1.5 1.5 0 0 1 4.5 17V3z"/>
      <path d="M11.5 1.5v5h5"/>
    </svg>
  `,
  search: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <circle cx="8.5" cy="8.5" r="5.5"/>
      <line x1="12.5" y1="12.5" x2="17" y2="17"/>
    </svg>
  `,
  settings: `
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
  `,
  info: `
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9"/><path d="M12 11v5"/><path d="M12 7.5h.01" stroke-width="2.2"/></svg>
  `,
  pages: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <rect x="5.5" y="4.5" width="11" height="13" rx="1.5"/>
      <path d="M3.5 13.5V3A1.5 1.5 0 0 1 5 1.5h9.5"/>
    </svg>
  `,
  flows: `
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16M15 4v16"/></svg>
  `,
  typography: `
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M5 7V5h14v2M9 19h6M12 5v14"/></svg>
  `,
  layers: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <polygon points="10 2.5 17.5 6.5 10 10.5 2.5 6.5 10 2.5"/>
      <polyline points="2.5 10 10 14 17.5 10"/>
      <polyline points="2.5 13.5 10 17.5 17.5 13.5"/>
    </svg>
  `,
  canvas: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <rect x="3.5" y="3.5" width="13" height="13" rx="1.5"/>
      <line x1="3.5" y1="7" x2="16.5" y2="7"/>
    </svg>
  `,
  split: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <rect x="2.5" y="3.5" width="15" height="13" rx="1.5"/>
      <line x1="10" y1="3.5" x2="10" y2="16.5"/>
    </svg>
  `,
  story: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <line x1="3.5" y1="4.5" x2="16.5" y2="4.5"/>
      <line x1="3.5" y1="8.5" x2="16.5" y2="8.5"/>
      <line x1="3.5" y1="12.5" x2="12.5" y2="12.5"/>
      <line x1="3.5" y1="16.5" x2="14.5" y2="16.5"/>
    </svg>
  `,
  export: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <path d="M4 13v3a1.5 1.5 0 0 0 1.5 1.5h9A1.5 1.5 0 0 0 16 16v-3"/>
      <polyline points="6.5 7 10 3.5 13.5 7"/>
      <line x1="10" y1="3.5" x2="10" y2="13"/>
    </svg>
  `,
  globe: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <circle cx="10" cy="10" r="7.5"/>
      <line x1="2.5" y1="10" x2="17.5" y2="10"/>
      <ellipse cx="10" cy="10" rx="3.5" ry="7.5"/>
    </svg>
  `,
  check: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <polyline points="4.5 10.5 8.5 14.5 15.5 6.5"/>
    </svg>
  `,
  close: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
      <line x1="5" y1="5" x2="15" y2="15"/>
      <line x1="15" y1="5" x2="5" y2="15"/>
    </svg>
  `,
  eye: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <path d="M2 10s3-5.5 8-5.5 8 5.5 8 5.5-3 5.5-8 5.5S2 10 2 10z"/>
      <circle cx="10" cy="10" r="2.5"/>
    </svg>
  `,
  eyeOff: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <path d="M3.5 3.5l13 13"/>
      <path d="M9.8 4.5A8.8 8.8 0 0 1 18 10s-1.8 3.3-4.7 4.7"/>
      <path d="M6.5 6.5C3.8 8 2 10 2 10s3 5.5 8 5.5c1.4 0 2.7-.4 3.9-1.1"/>
      <path d="M8.5 8.6a2.5 2.5 0 0 0 3 3"/>
    </svg>
  `,
  lock: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <rect x="4.5" y="9" width="11" height="8.5" rx="1.5"/>
      <path d="M7 9V6a3 3 0 0 1 6 0v3"/>
    </svg>
  `,
  unlock: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <rect x="4.5" y="9" width="11" height="8.5" rx="1.5"/>
      <path d="M7 9V5.5a3 3 0 0 1 5.8-1"/>
    </svg>
  `,
  refresh: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <path d="M17.5 9A7.5 7.5 0 1 1 15 4.5l2.5.5"/>
      <polyline points="17.5 1 17.5 5 13.5 5"/>
    </svg>
  `,
  trash: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <polyline points="3.5 5.5 16.5 5.5"/>
      <path d="M6.5 5.5V4a1.5 1.5 0 0 1 1.5-1.5h4a1.5 1.5 0 0 1 1.5 1.5v1.5"/>
      <path d="M5.5 5.5v10.5A1.5 1.5 0 0 0 7 17.5h6a1.5 1.5 0 0 0 1.5-1.5V5.5"/>
    </svg>
  `,
  plugin: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <path d="M4 11h2a2 2 0 1 0 0-4H4V4h3a2 2 0 1 0 4 0h5v3a2 2 0 1 0 0 4v5h-3a2 2 0 1 0-4 0H4v-5z"/>
    </svg>
  `,
  accessibility: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <circle cx="10" cy="4" r="1.5"/>
      <path d="M4 8.5h12"/>
      <path d="M10 8.5V17"/>
      <path d="M7 17l3-4 3 4"/>
    </svg>
  `,
  contrast: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6">
      <circle cx="10" cy="10" r="7.5"/>
      <path d="M10 2.5A7.5 7.5 0 0 1 10 17.5v-15z" fill="currentColor"/>
    </svg>
  `,
  textScale: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <path d="M2.5 15.5l4-10 4 10"/>
      <line x1="4" y1="12" x2="9" y2="12"/>
      <path d="M12.5 15.5l3-7 3 7"/>
      <line x1="13.7" y1="13" x2="17.3" y2="13"/>
    </svg>
  `,
  motion: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <path d="M3 7h7a3 3 0 0 1 0 6H3"/>
      <line x1="14" y1="10" x2="17" y2="10"/>
      <polyline points="7 4 3 7 7 10"/>
    </svg>
  `,
  keyboard: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <rect x="2.5" y="5" width="15" height="10" rx="1.5"/>
      <line x1="5.5" y1="8" x2="6.5" y2="8"/>
      <line x1="9.5" y1="8" x2="10.5" y2="8"/>
      <line x1="13.5" y1="8" x2="14.5" y2="8"/>
      <line x1="6.5" y1="12" x2="13.5" y2="12"/>
    </svg>
  `,
  templateTalmud: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
      <rect x="3" y="3" width="14" height="14" rx="1.5"/>
      <rect x="6.5" y="6" width="7" height="8" rx="0.5"/>
      <line x1="4.5" y1="6" x2="5.5" y2="6"/>
      <line x1="4.5" y1="8" x2="5.5" y2="8"/>
      <line x1="14.5" y1="6" x2="15.5" y2="6"/>
      <line x1="14.5" y1="8" x2="15.5" y2="8"/>
    </svg>
  `,
  templateBook: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
      <path d="M3 4.5A2 2 0 0 1 5 2.5h5v15H5a2 2 0 0 0-2 2V4.5z"/>
      <path d="M17 4.5A2 2 0 0 0 15 2.5h-5v15h5a2 2 0 0 1 2 2V4.5z"/>
    </svg>
  `,
  templateColumns: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
      <rect x="2.5" y="3" width="4" height="14" rx="1"/>
      <rect x="8" y="3" width="4" height="14" rx="1"/>
      <rect x="13.5" y="3" width="4" height="14" rx="1"/>
    </svg>
  `,
  templateBlank: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
      <rect x="3.5" y="3.5" width="13" height="13" rx="1.5" stroke-dasharray="2 2"/>
      <line x1="10" y1="8" x2="10" y2="12"/>
      <line x1="8" y1="10" x2="12" y2="10"/>
    </svg>
  `,
  sparkle: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <path d="M10 2c0 4-3.5 8-8 8 4.5 0 8 4 8 8 0-4 3.5-8 8-8-4.5 0-8-4-8-8z"/>
      <path d="M15.5 2.5c0 1.2-1 2.5-2.5 2.5 1.5 0 2.5 1.3 2.5 2.5 0-1.2 1-2.5 2.5-2.5-1.5 0-2.5-1.3-2.5-2.5z"/>
    </svg>
  `,
  alignJustify: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round">
      <line x1="3" y1="4.5" x2="17" y2="4.5"/>
      <line x1="3" y1="8.5" x2="17" y2="8.5"/>
      <line x1="3" y1="12.5" x2="17" y2="12.5"/>
      <line x1="3" y1="16.5" x2="17" y2="16.5"/>
    </svg>
  `,
  warning: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <path d="M10 2.5L2 17h16L10 2.5z"/>
      <line x1="10" y1="8" x2="10" y2="12"/>
      <circle cx="10" cy="14.5" r="0.75" fill="currentColor"/>
    </svg>
  `,
  chevronDown: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
      <polyline points="5.5 7.5 10 12 14.5 7.5"/>
    </svg>
  `,
  chevronLeft: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
      <polyline points="12.5 5.5 8 10 12.5 14.5"/>
    </svg>
  `,
  chevronRight: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
      <polyline points="7.5 5.5 12 10 7.5 14.5"/>
    </svg>
  `,
  image: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <rect x="3" y="3" width="14" height="14" rx="1.5"/>
      <circle cx="7.5" cy="7.5" r="1.5"/>
      <polyline points="17 13.5 13 9.5 6 16.5"/>
    </svg>
  `,
  frame: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <rect x="3" y="3" width="14" height="14" rx="1"/>
      <line x1="3" y1="7.5" x2="17" y2="7.5"/>
      <line x1="7.5" y1="3" x2="7.5" y2="17"/>
    </svg>
  `,
  zap: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <polyline points="6 6 10 10 6 14"/><line x1="12" y1="14" x2="15" y2="14"/>
    </svg>
  `,
  bold: `
    <svg viewBox="0 0 20 20" fill="currentColor">
      <path d="M5.5 3.5h5.2a3.5 3.5 0 0 1 2.3 6.1A3.7 3.7 0 0 1 11.5 16.5H5.5V3.5zm2.8 2.2v3.6h2.4a1.8 1.8 0 0 0 0-3.6H8.3zm0 5.8v3.6H12a1.8 1.8 0 0 0 0-3.6H8.3z"/>
    </svg>
  `,
  history: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <circle cx="10" cy="10" r="7"/>
      <polyline points="10 6 10 10 13 12"/>
    </svg>
  `,
  documentEmpty: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <path d="M5 3a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V7.5L12.5 3H5z"/>
      <polyline points="12 3 12 8 17 8"/>
    </svg>
  `,
  ruler: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <rect x="2.5" y="6" width="15" height="8" rx="1"/>
      <line x1="5.5" y1="6" x2="5.5" y2="9"/>
      <line x1="8.5" y1="6" x2="8.5" y2="11"/>
      <line x1="11.5" y1="6" x2="11.5" y2="9"/>
      <line x1="14.5" y1="6" x2="14.5" y2="11"/>
    </svg>
  `,
  edit: `
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <path d="M12.5 3.5l4 4L6 18H2v-4L12.5 3.5z"/>
    </svg>
  `
};

/**
 * Returns an inline SVG string for the specified icon.
 */
export function renderIcon(name: IconName, size: number = 14, extraClass: string = ''): string {
  const rawSvg = ICONS_SVG[name] || ICONS_SVG.file;
  return `<span class="tok-icon-wrap ${extraClass}" style="display:inline-flex;align-items:center;justify-content:center;width:${size}px;height:${size}px;vertical-align:middle;flex-shrink:0;">
    ${rawSvg.replace('<svg ', `<svg width="${size}" height="${size}" `)}
  </span>`;
}

/**
 * Creates an HTMLElement with the rendered SVG icon.
 */
export function createIconElement(name: IconName, size: number = 14, extraClass: string = ''): HTMLElement {
  const span = document.createElement('span');
  span.className = `tok-icon-wrap ${extraClass}`;
  span.style.display = 'inline-flex';
  span.style.alignItems = 'center';
  span.style.justifyContent = 'center';
  span.style.width = `${size}px`;
  span.style.height = `${size}px`;
  span.style.verticalAlign = 'middle';
  span.style.flexShrink = '0';
  const rawSvg = ICONS_SVG[name] || ICONS_SVG.file;
  span.innerHTML = rawSvg.replace('<svg ', `<svg width="${size}" height="${size}" `);
  return span;
}
