export type Language = 'he' | 'en';

export interface Translations {
  [key: string]: {
    he: string;
    en: string;
  };
}

export const strings: Translations = {
  // Brand & Welcome
  appName: { he: 'TypesetOK', en: 'TypesetOK' },
  appTagline: { he: 'מערכת עימוד והוצאה לאור מקצועית לטקסט עברי', en: 'Professional Hebrew Desktop Publishing System' },
  welcomeTitle: { he: 'מה נעמד היום?', en: 'What shall we typeset today?' },
  welcomeSubtitle: { he: 'בחרו תבנית להתחלה מהירה, או הקימו מסמך ריק.', en: 'Choose a template to get started or create a blank document.' },
  newProject: { he: 'הקמת מסמך חדש', en: 'Create New Document' },
  openProject: { he: 'פתיחת קובץ…', en: 'Open file…' },
  demoProject: { he: 'פתיחת הדוגמה', en: 'Open the sample' },
  recentProjects: { he: 'פרויקטים אחרונים', en: 'Recent projects' },
  showOnStartup: { he: 'הצגת המסך הזה בכל פתיחה', en: 'Show this screen on startup' },
  noRecentProjects: { he: 'אין היסטוריית פרויקטים', en: 'No project history' },
  noRecentProjectsSub: { he: 'קבצים שתפתחו או תשמרו יוצגו כאן לגישה מהירה', en: 'Projects you open or save will appear here' },
  returnToDocument: { he: 'חזרה למסמך הפעיל', en: 'Return to Active Document' },
  appMainText: { he: 'טקסט ראשי', en: 'Main text' },
  continueToWorkspace: { he: 'המשך לסביבת העבודה', en: 'Continue to Workspace' },
  welcomePagesCount: { he: '{n} עמודים', en: '{n} pages' },

  // Templates
  templateGemara: { he: 'דף גמרא', en: 'Talmud page' },
  templateGemaraDesc: { he: 'צורת הדף המסורתית: גמרא במרכז, רש״י ותוספות מסביב.', en: 'The traditional layout: Gemara in the center, Rashi and Tosafot around it.' },
  templateProse: { he: 'ספר רציף', en: 'Continuous book' },
  templateProseDesc: { he: 'טור אחד של טקסט, לספרי עיון, הגות וסיפורת.', en: 'One column of text for study books, essays and prose.' },
  templateBulletin: { he: 'עלון וקונטרס', en: 'Bulletin & booklet' },
  templateBulletinDesc: { he: 'שני טורים עם כותרת, לעלוני שבת וחוברות.', en: 'Two columns with a headline, for weekly bulletins and booklets.' },
  templateBlank: { he: 'מסמך ריק', en: 'Blank document' },
  templateBlankDesc: { he: 'בוחרים גודל דף ושוליים בעצמכם', en: 'Choose page size and margins yourself' },

  // Top Bar & Navigation
  topBarProjects: { he: 'פרויקטים', en: 'Projects' },
  topBarMenu: { he: 'תפריט', en: 'Menu' },
  topBarFileAndMenu: { he: 'קובץ', en: 'File' },
  topBarSearchPlaceholder: { he: 'חיפוש פקודה…', en: 'Search commands…' },
  topBarViewCanvas: { he: 'תצוגת עמודים', en: 'Pages View' },
  topBarViewSplit: { he: 'תצוגה משולבת', en: 'Split View' },
  topBarViewStory: { he: 'עורך טקסט', en: 'Text Editor' },
  topBarExportPdf: { he: 'ייצוא PDF', en: 'Export PDF' },
  topBarSaved: { he: 'נשמר', en: 'Saved' },
  topBarSwitchLanguage: { he: 'החלפת שפה', en: 'Switch language' },
  topBarViewMode: { he: 'תצוגה', en: 'View' },

  // Quick Menu
  menuNewDocument: { he: 'הקמת מסמך חדש...', en: 'New Document...' },
  menuOpenDocument: { he: 'פתיחת מסמך…', en: 'Open Document (.tok)...' },
  menuSave: { he: 'שמירת מסמך', en: 'Save Document' },
  menuSaveAs: { he: 'שמירה בשם...', en: 'Save As...' },
  menuNormalizeNiqqud: { he: 'נרמול ניקוד וטעמים (ת״י 6100)', en: 'Normalize Niqqud & Cantillation (SI 6100)' },
  menuShieldDivineNames: { he: 'מגן שמות קדושים (איסור שבירה)', en: 'Divine Names Shield (No Break)' },
  menuRecalcGematria: { he: 'סנכרון מספור עמודים עברי', en: 'Sync Hebrew Page Numbering' },
  menuExportPdf: { he: 'ייצוא לדפוס…', en: 'Export for print…' },

  // Command Palette
  paletteSearchPlaceholder: { he: 'הקלד לחיפוש פקודות, תזרימים, עמודים או פעולות עימוד...', en: 'Type to search commands, flows, pages or layout actions...' },
  paletteHintNavigate: { he: 'ניווט עם ↑ ↓', en: '↑ ↓ to navigate' },
  paletteHintRun: { he: '↵ להפעלה', en: '↵ to run' },
  paletteHintClose: { he: 'Esc ליציאה', en: 'Esc to close' },
  paletteNoResults: { he: 'לא נמצאו פקודות תואמות', en: 'No matching commands' },

  // Status Bar
  statusTextLength: { he: 'אורך טקסט', en: 'Text length' },
  statusWords: { he: 'מילים', en: 'words' },
  statusActiveFlow: { he: 'תזרים פעיל', en: 'Active flow' },
  statusJustificationRules: { he: 'שבירת שורות אופטימלית + אהלת״ם', en: 'Optimal line breaking + Ahalterm' },
  statusPreflightClean: { he: 'מוכן להדפסה', en: 'Ready for print' },
  statusPreflightWarnings: { he: 'בדיקת דפוס: אזהרות', en: 'Print check: warnings' },
  statusPreflightErrors: { he: 'בדיקת דפוס: שגיאות', en: 'Print check: errors' },
  statusZoom: { he: 'זום', en: 'Zoom' },
  statusGoToPage: { he: 'מעבר לעמוד או לפקודה (Ctrl+K)', en: 'Go to page or command (Ctrl+K)' },

  // Sidebar Tabs
  sidebarPages: { he: 'עמודים', en: 'Pages' },
  sidebarFlows: { he: 'תזרימים', en: 'Flows' },
  sidebarStyles: { he: 'סגנונות', en: 'Styles' },
  sidebarLayers: { he: 'שכבות', en: 'Layers' },
  sidebarSettings: { he: 'הגדרות', en: 'Settings' },
  sidebarAbout: { he: 'אודות', en: 'About' },
  sidebarAddPage: { he: 'הוספת עמוד', en: 'Add Page' },
  structurePagesCount: { he: 'עמודים', en: 'Pages' },
  structurePageLabel: { he: 'דף {g}', en: 'Page {g}' },
  structurePageNumber: { he: "עמ' {n}", en: 'p. {n}' },
  structureSpreadRight: { he: 'כפולה ימנית', en: 'right of spread' },
  structureSpreadLeft: { he: 'כפולה שמאלית', en: 'left of spread' },
  structureFlowsTitle: { he: 'תזרימים', en: 'Flows' },
  structureFlowsDesc: { he: 'כל תזרים הוא טקסט רציף אחד. המנוע מפזר אותו על פני העמודים לפי צורת הדף.', en: 'Each flow is one continuous text. The engine pours it across the pages following the page layout.' },
  structureFlowPosition: { he: 'מיקום', en: 'Position' },
  structureStylesTitle: { he: 'סגנונות', en: 'Styles' },
  structureLayersTitle: { he: 'שכבות עבודה', en: 'Layers' },
  structureHideLayer: { he: 'הסתר שכבה', en: 'Hide layer' },
  structureShowLayer: { he: 'הצג שכבה', en: 'Show layer' },
  structureLockLayer: { he: 'נעל שכבה', en: 'Lock layer' },
  structureUnlockLayer: { he: 'שחרר נעילת שכבה', en: 'Unlock layer' },

  // Settings Tabs
  settingsTitle: { he: 'הגדרות', en: 'Settings' },
  settingsTabAppearance: { he: 'מראה', en: 'Appearance' },
  settingsTabAccessibility: { he: 'נגישות', en: 'Accessibility' },
  settingsTabLanguage: { he: 'שפה', en: 'Language' },
  settingsTabLogs: { he: 'יומן ותחזוקה', en: 'Logs & maintenance' },
  settingsTabUpdates: { he: 'עדכונים', en: 'Updates' },
  settingsTabPlugins: { he: 'תוספים', en: 'Plugins' },

  // Appearance Settings
  appearanceAccentColor: { he: 'צבע הדגשה', en: 'Accent color' },
  appearanceCanvasTone: { he: 'גוון שולחן העבודה', en: 'Desk color' },
  appearanceDensity: { he: 'צפיפות הממשק', en: 'Interface density' },
  densityComfortable: { he: 'רגילה', en: 'Normal' },
  densityCompact: { he: 'דחוסה', en: 'Compact' },

  // Accessibility Settings
  accessHighContrast: { he: 'ניגודיות גבוהה', en: 'High contrast' },
  accessHighContrastDesc: { he: 'שיפור חדות הקריאה באמצעות רקע כהה מוחלט והבלטת גבולות וטקסטים', en: 'Enhance readability with pure black backgrounds and high-contrast borders' },
  accessFontScale: { he: 'גודל הטקסט בממשק', en: 'Interface text size' },
  accessFontScaleDesc: { he: 'הגדלת הגופנים בכל רחבי המערכת להתאמה אופטימלית', en: 'Adjust overall interface font scale for comfortable viewing' },
  accessReducedMotion: { he: 'הפחתת אנימציות ותנועה', en: 'Reduce Motion & Animations' },
  accessReducedMotionDesc: { he: 'ביטול מעברים והנפשות לטובת יציבות חזותית מוחלטת', en: 'Disable interface transitions and pulses for visual stability' },
  accessEnhancedFocus: { he: 'הדגשת פוקוס בניווט מקלדת', en: 'Enhanced Keyboard Focus Ring' },
  accessEnhancedFocusDesc: { he: 'מסגרת בולטת במיוחד סביב רכיב נבחר בעת מעבר עם מקש Tab', en: 'Prominent focus indicators when navigating the interface via keyboard' },
  accessDyslexicFont: { he: 'גופן ממשק קריא במיוחד', en: 'Extra-legible interface font' },
  accessDyslexicFontDesc: { he: 'החלת גופן קריא בעל הבחנה גבוהה בין אותיות דומות', en: 'Use high-legibility sans-serif font across all interface panels' },

  // Language Settings
  languageSelect: { he: 'שפת הממשק', en: 'Interface language' },
  languageHebrew: { he: 'עברית (מימין לשמאל)', en: 'Hebrew (right to left)' },
  languageEnglish: { he: 'English (left to right)', en: 'English (left to right)' },
  languageActiveBadge: { he: 'פעיל', en: 'Active' },

  // Logs Settings
  logsRetentionLabel: { he: 'שמירת יומנים (מחיקה אוטומטית)', en: 'Keep logs for (auto-cleanup)' },
  logsRetentionDays: { he: 'ימים', en: 'days' },
  logsOpenFolder: { he: 'פתיחת תיקיית היומנים', en: 'Open logs folder' },
  logsCleanNow: { he: 'מחיקת יומנים ישנים', en: 'Delete old logs' },
  logsRecentTitle: { he: 'רשומות אחרונות', en: 'Recent entries' },

  // Updates Settings
  updatesStatusChecking: { he: 'בודק זמינות עדכונים מול השרת...', en: 'Checking for updates on GitHub...' },
  updatesStatusLatest: { he: 'הגרסה המותקנת היא העדכנית ביותר', en: 'You are using the latest version' },
  updatesStatusAvailable: { he: 'גרסה חדשה זמינה להורדה והתקנה', en: 'A newer version is available for download' },
  updatesCheckNow: { he: 'בדיקת עדכונים', en: 'Check for updates' },
  updatesAutoCheck: { he: 'בדיקת עדכונים אוטומטית בעת פתיחת התוכנה', en: 'Automatically check for updates on startup' },
  updatesDownload: { he: 'הורדת חבילת העדכון', en: 'Download Update Package' },
  updatesCheckFailed: { he: 'בדיקת העדכונים נכשלה', en: 'Update check failed' },
  actionFailed: { he: 'הפעולה נכשלה', en: 'Action failed' },
  stateOn: { he: 'הופעל', en: 'enabled' },
  stateOff: { he: 'הושבת', en: 'disabled' },
  desktopOnlyFeature: { he: 'פעולה זו זמינה רק בגרסת שולחן העבודה', en: 'This action is only available in the desktop app' },
  logsRetentionUpdated: { he: 'מדיניות מחיקת יומנים עודכנה ל-{n} ימים', en: 'Log retention set to {n} days' },
  logsCleanedCount: { he: 'ניקוי הושלם: נמחקו {n} קובצי יומן ישנים', en: 'Cleanup done: {n} old log files deleted' },
  logsCleaned: { he: 'ניקוי יומנים הושלם בהצלחה', en: 'Log cleanup completed' },
  logsLoading: { he: 'טוען רשומות מיומן המערכת...', en: 'Loading log entries...' },
  logsEmpty: { he: '[אין רשומות יומן להצגה]', en: '[No log entries]' },
  logsLoadFailed: { he: 'שגיאה בטעינת יומן', en: 'Failed to load logs' },
  pluginsReloaded: { he: 'התוספים נטענו מחדש בהצלחה', en: 'Plugins reloaded' },
  pluginsLoadError: { he: 'שגיאת טעינה', en: 'Load error' },
  updatesCurrentChannel: { he: 'ערוץ שחרור רשמי יציב', en: 'Official stable channel' },
  updatesHint: { he: 'לחצו על "בדיקת עדכונים" כדי לבדוק אם יש גרסה חדשה ב-GitHub.', en: 'Click "Check for updates" to look for a new release on GitHub.' },

  // Plugins Settings
  pluginsInstalled: { he: 'תוספים מותקנים', en: 'Installed plugins' },
  pluginsOpenFolder: { he: 'פתיחת תיקיית התוספים', en: 'Open Plugins Directory' },
  pluginsReload: { he: 'טעינה מחדש', en: 'Reload all' },
  pluginsEnabled: { he: 'פעיל', en: 'Enabled' },
  pluginsDisabled: { he: 'מושבת', en: 'Disabled' },
  pluginsNoPlugins: { he: 'לא נמצאו תוספים בתיקיית ההרחבות', en: 'No plugin extensions found' },
  pluginsSafeMode: { he: 'מצב בטוח (Safe Mode / סייפר)', en: 'Safe Mode (Safer)' },
  pluginsSafeModeDesc: { he: 'השבתת כל התוספים החיצוניים למניעת נזק, קריסות או פעילות זדונית', en: 'Disable all third-party plugins to protect against crashes, malicious code, or instability' },
  pluginsSaferActive: { he: 'סביבת סייפר (Safer) פעילה', en: 'Safer environment active' },
  pluginsSaferActiveDesc: { he: 'זוהתה עמדת מחשב תורנית מנוהלת (סייפר). התוכנה פועלת באופן מקומי ומאובטח (Offline).', en: 'Managed workstation detected (Safer). TypesetOK is running in secure offline mode.' },
  pluginsPermissions: { he: 'הרשאות', en: 'Permissions' },
  pluginsPermDocRead: { he: 'קריאת מסמך', en: 'Read document' },
  pluginsPermDocWrite: { he: 'עריכת טקסט', en: 'Edit text' },
  pluginsPermCommands: { he: 'פקודות', en: 'Commands' },
  pluginsPermNotifications: { he: 'הודעות', en: 'Notifications' },
  pluginsPermModals: { he: 'חלונות', en: 'Modals' },
  pluginsPermCanvas: { he: 'קנבס וניווט', en: 'Canvas & pages' },
  pluginsPermNetwork: { he: 'רשת חיצונית', en: 'External network' },
  pluginsBlockedBySafeMode: { he: 'מושבת במצב בטוח', en: 'Blocked by Safe Mode' },
  pluginsDisableAll: { he: 'השבתת כל התוספים (חירום)', en: 'Disable all plugins (Emergency)' },
  pluginsAllDisabled: { he: 'כל התוספים הושבתו בהצלחה', en: 'All plugins disabled' },

  // Action HUD
  hudFont: { he: 'גופן', en: 'Font' },
  hudSmaller: { he: 'הקטן גופן', en: 'Decrease size' },
  hudLarger: { he: 'הגדל גופן', en: 'Increase size' },
  hudBold: { he: 'מודגש', en: 'Bold' },
  inspWeight: { he: 'משקל', en: 'Weight' },
  inspWeightRegular: { he: 'רגיל (400)', en: 'Regular (400)' },
  inspWeightBold: { he: 'מודגש (700)', en: 'Bold (700)' },
  hudAlignRight: { he: 'ימין', en: 'Right' },
  hudAlignCenter: { he: 'מרכז', en: 'Center' },
  hudAlignJustify: { he: 'בלוק', en: 'Justify' },
  hudAlignTitle: { he: 'שנה יישור (ימין / מרכז / בלוק)', en: 'Change alignment (right / center / justify)' },
  hudQuickStyle: { he: 'בורר סגנון מהיר', en: 'Quick style' },

  // Contextual Inspector
  inspAriaLabel: { he: 'מפקח מאפיינים', en: 'Properties inspector' },
  inspDocTitle: { he: 'מסמך ועמוד', en: 'Document & page' },
  inspDocSubtitle: { he: 'מסמך תורני', en: 'Torah document' },
  inspPageSize: { he: 'גודל עמוד', en: 'Page size' },
  inspPresetSefer: { he: 'ספר קודש סטנדרטי (17×24 ס"מ)', en: 'Standard sefer (17×24 cm)' },
  inspPresetCrown: { he: 'פורמט קראון (16.5×23.5 ס"מ)', en: 'Crown (16.5×23.5 cm)' },
  inspPresetA4: { he: 'פורמט A4 (21×29.7 ס"מ)', en: 'A4 (21×29.7 cm)' },
  inspPresetB5: { he: 'פורמט B5 (17.6×25 ס"מ)', en: 'B5 (17.6×25 cm)' },
  inspWidth: { he: 'רוחב', en: 'Width' },
  inspHeight: { he: 'גובה', en: 'Height' },
  unitMm: { he: 'מ"מ', en: 'mm' },
  inspGradedMargins: { he: 'שוליים', en: 'Margins' },
  inspTop: { he: 'עליון', en: 'Top' },
  inspBottom: { he: 'תחתון', en: 'Bottom' },
  inspInside: { he: 'פנימי/שדרה', en: 'Inside/Spine' },
  inspOutside: { he: 'חיצוני', en: 'Outside' },
  inspBaselineGrid: { he: 'רשת שורות בסיס', en: 'Baseline grid' },
  inspLineStep: { he: 'צעד שורה', en: 'Line step' },
  inspTopOffset: { he: 'היסט עליון', en: 'Top offset' },
  inspPreflight: { he: 'בדיקת דפוס', en: 'Print check' },
  inspProdStatus: { he: 'סטטוס ייצור', en: 'Production status' },
  inspReadyForPrint: { he: 'מוכן להדפסה', en: 'Ready for print' },
  inspColorProfile: { he: 'פרופיל צבע', en: 'Color profile' },
  inspOverset: { he: 'טקסט גולש', en: 'Overset text' },
  inspNoIssues: { he: '0 חריגות', en: '0 issues' },
  inspImageRes: { he: 'רזולוציית תמונות', en: 'Image resolution' },
  inspImageResOk: { he: '300+ DPI', en: '300+ DPI' },
  inspTextFrame: { he: 'תיבת טקסט', en: 'Text Frame' },
  inspGeometry: { he: 'מיקום וממדים', en: 'Position & size' },
  inspPosX: { he: 'מיקום X', en: 'X' },
  inspPosY: { he: 'מיקום Y', en: 'Y' },
  inspWidthW: { he: 'רוחב W', en: 'Width W' },
  inspHeightH: { he: 'גובה H', en: 'Height H' },
  inspFlowThreading: { he: 'תזרים ושרשור', en: 'Flow & threading' },
  inspFlowGemara: { he: 'גמרא (טקסט מרכזי)', en: 'Gemara (main text)' },
  inspFlowRashi: { he: 'רש"י (פירוש פנימי)', en: 'Rashi (inner commentary)' },
  inspFlowTosafot: { he: 'תוספות (פירוש חיצוני)', en: 'Tosafot (outer commentary)' },
  inspFlowNotes: { he: 'הערות שוליים וציונים', en: 'Footnotes & references' },
  inspNone: { he: 'ללא', en: 'none' },
  inspInsets: { he: 'שוליים פנימיים ויישור אנכי', en: 'Insets & vertical alignment' },
  inspRight: { he: 'ימין', en: 'Right' },
  inspLeft: { he: 'שמאל', en: 'Left' },
  inspVAlign: { he: 'יישור אנכי', en: 'Vertical alignment' },
  inspCenter: { he: 'מרכז', en: 'Center' },
  inspJustify: { he: 'מלא', en: 'Justify' },
  inspTypography: { he: 'טיפוגרפיה', en: 'Typography' },
  inspStyleToken: { he: 'סגנון פסקה', en: 'Paragraph style' },
  inspLocalOverride: { he: 'שינוי מקומי', en: 'Local override' },
  inspFontSpacing: { he: 'גופן', en: 'Font' },
  inspFontSize: { he: 'גודל גופן', en: 'Font size' },
  inspLeading: { he: 'רווח שורות', en: 'Leading' },
  inspJustify3: { he: 'יישור עברי', en: 'Hebrew justification' },
  inspTier1: { he: 'רווחי מילים (80%–130%)', en: 'Word spacing (80%–130%)' },
  inspMin: { he: 'מינימום', en: 'Minimum' },
  inspMax: { he: 'מקסימום', en: 'Maximum' },
  inspTier2: { he: 'מתיחת אותיות אהלת״ם', en: 'Ahalterm letter stretching' },
  inspEnableStretch: { he: 'מתיחת אותיות', en: 'Letter stretching' },
  inspMaxStretch: { he: 'מקסימום מתיחה', en: 'Max stretch' },
  inspTier3: { he: 'מיקרו-ריווח בין אותיות (±2%)', en: 'Micro-tracking (±2%)' },
  inspTrackingRange: { he: 'טווח', en: 'Range' },
  inspSacred: { he: 'ניקוד, טעמים ושמות קדושים', en: 'Niqqud, cantillation & divine names' },
  inspDivineShield: { he: 'מגן שמות קדושים (בלי שבירה)', en: 'Divine names shield (no break)' },
  inspImageFrame: { he: 'מסגרת תמונה ועיטור', en: 'Image & Ornament Frame' },
  inspImage: { he: 'תמונה', en: 'Image' },
  inspFitting: { he: 'התאמת תמונה', en: 'Image fitting' },
  inspFitProportional: { he: 'התאם פרופורציונלית', en: 'Fit proportionally' },
  inspFitFill: { he: 'מלא מסגרת לחלוטין', en: 'Fill frame' },
  inspFitFrame: { he: 'התאם מסגרת לתוכן', en: 'Fit frame to content' },
  inspEffRes: { he: 'רזולוציה אפקטיבית', en: 'Effective resolution' },
  inspEffResOk: { he: '300 DPI (תקין)', en: '300 DPI (OK)' },
  inspColorSpace: { he: 'מרחב צבע', en: 'Color space' },
  inspWrap: { he: 'גלישת טקסט', en: 'Text wrap' },
  inspWrapAround: { he: 'סביב מסגרת (4mm)', en: 'Around frame (4mm)' },
  inspMulti: { he: 'בחירה מרובה', en: 'Multiple Selection' },
  inspMultiCount: { he: '3 אובייקטים', en: '3 objects' },
  inspAlignDist: { he: 'יישור ופיזור', en: 'Align & distribute' },
  inspAlignRight: { he: 'ימין', en: 'Right' },
  inspAlignCenter: { he: 'מרכז', en: 'Center' },
  inspAlignLeft: { he: 'שמאל', en: 'Left' },
  inspAlignTop: { he: 'למעלה', en: 'Top' },
  inspAlignMiddle: { he: 'אמצע', en: 'Middle' },
  inspAlignBottom: { he: 'למטה', en: 'Bottom' },
  inspDistribute: { he: 'פיזור מרווחים שווה אנכית', en: 'Distribute vertical spacing' },
  inspScrubHint: { he: 'גררו ימינה או שמאלה לכוונון. בשדה: חיצים למעלה ולמטה.', en: 'Drag left/right to adjust (↑/↓ in the field)' },
  inspThreading: { he: 'ממשיך אל', en: 'Threaded to' },
  inspSyncStyle: { he: 'עדכן סגנון גלובלי מהשינוי הנוכחי', en: 'Update global style from this override' },
  inspNormalize: { he: 'נרמול סדר הניקוד (ת״י 6100)', en: 'Normalize niqqud order (SI 6100)' },

  // About Modal
  aboutTitle: { he: 'אודות TypesetOK', en: 'About TypesetOK' },
  aboutVersionLabel: { he: 'גרסה', en: 'Version' },
  aboutCoreLabel: { he: 'מנוע עימוד', en: 'Typesetting engine' },
  aboutRustVersion: { he: 'Rust · שבירת שורות Knuth-Plass ויישור אהלת״ם', en: 'Rust Native (Knuth-Plass & Ahalterm Justifier)' },
  aboutShellLabel: { he: 'מעטפת', en: 'Desktop shell' },
  aboutShellValue: { he: 'Electron ו-Chromium', en: 'Electron + Chromium Pre-Press Platform' },
  aboutGithubBtn: { he: 'מאגר הפרויקט ב-GitHub', en: 'Project on GitHub' },
  aboutClose: { he: 'סגירה', en: 'Close' },

  // App shell (app.ts / renderer.ts): command palette
  cmdCatProjects: { he: 'פרויקטים ומסמכים', en: 'Projects & Documents' },
  cmdCatSystem: { he: 'מערכת והעדפות', en: 'System & Preferences' },
  cmdCatTypography: { he: 'פעולות טיפוגרפיה', en: 'Typography' },
  cmdCatPrint: { he: 'מערכת ודפוס', en: 'Output & Print' },
  cmdCatPages: { he: 'עמודים וניווט', en: 'Pages & Navigation' },
  cmdCatView: { he: 'תצוגה ורשת', en: 'View & Grid' },
  cmdWelcomeTitle: { he: 'מסך פתיחה', en: 'Start screen' },
  cmdWelcomeSub: { he: 'בחירת תבנית או פרויקט קיים', en: 'Choose a template or an existing project' },
  cmdSettingsTitle: { he: 'הגדרות', en: 'Settings' },
  cmdSettingsSub: { he: 'ערכות עיצוב, שפה, לוגים ותוספים', en: 'Themes, language, logs and plugins' },
  cmdAboutTitle: { he: 'אודות TypesetOK', en: 'About TypesetOK' },
  cmdAboutSub: { he: 'גרסה, רישיון ומאגר GitHub', en: 'Version, license and GitHub repository' },
  cmdToggleLangTitle: { he: 'החלפת שפה (עברית / English)', en: 'Switch language (Hebrew / English)' },
  cmdJustifyTitle: { he: 'יישור עברי מלא', en: 'Full Hebrew justification' },
  cmdJustifySub: { he: 'שילוב 3 שכבות יישור', en: 'Combines all 3 justification tiers' },
  cmdNormalizeTitle: { he: 'נרמל ניקוד וטעמים (ת"י 6100)', en: 'Normalize niqqud & cantillation (SI 6100)' },
  cmdNormalizeSub: { he: 'תיקון סדר תווי יוניקוד', en: 'Fixes the order of Unicode marks' },
  cmdShieldTitle: { he: 'מגן שמות קדושים (איסור שבירה)', en: 'Divine names shield (no break)' },
  cmdShieldSub: { he: 'הגנה על שמות הוי"ה ואדנות', en: 'Protects the Tetragrammaton and Adonai' },
  cmdGematriaTitle: { he: 'סנכרן מספור עמודים עברי (גימטריה)', en: 'Sync Hebrew page numbering (gematria)' },
  cmdGematriaSub: { he: 'החלת גרשיים וכללי טו/טז', en: 'Applies geresh/gershayim and the 15/16 rule' },
  cmdExportTitle: { he: 'ייצוא לדפוס', en: 'Export for print' },
  cmdExportSub: { he: 'PDF/X-1a, שחור נקי', en: 'PDF/X-1a, clean black' },
  cmdNewPageTitle: { he: 'הוסף עמוד חדש לספר', en: 'Add a new page' },
  cmdMarginsTitle: { he: 'הצגה או הסתרה של קווי שוליים', en: 'Show/hide margin guides' },
  cmdBaselineTitle: { he: 'הצגה או הסתרה של רשת שורות בסיס', en: 'Show/hide baseline grid' },
  cmdZoom100Title: { he: 'זום 100%', en: 'Zoom 100%' },

  // App shell: pages, panels and notifications
  appStoryEditorTitle: { he: 'עורך טקסט', en: 'Text Editor' },
  sidebarSettingsDesc: { he: 'מראה, שפה ומקשים', en: 'Appearance & keys' },
  sidebarAboutDesc: { he: 'גרסה ורישיון', en: 'Version & info' },
  sidebarExpand: { he: 'הרחבת סרגל', en: 'Expand sidebar' },
  sidebarCollapse: { he: 'צמצום סרגל', en: 'Collapse sidebar' },
  appearanceZoom: { he: 'זום ברירת מחדל לתצוגת עמודים', en: 'Default zoom for pages view' },
  appPageThumb: { he: 'דף {page}', en: 'Page {page}' },
  appPageStatus: { he: 'דף {page} · {index} מתוך {total}', en: 'Page {page} · {index} of {total}' },
  appLangHebrewRtl: { he: 'עברית (RTL)', en: 'Hebrew (RTL)' },
  appLangEnglishLtr: { he: 'English (LTR)', en: 'English (LTR)' },
  appLangHebrew: { he: 'עברית', en: 'Hebrew' },
  appLangEnglish: { he: 'English', en: 'English' },
  appLayerShown: { he: 'מוצגת', en: 'shown' },
  appLayerHidden: { he: 'מוסתרת', en: 'hidden' },
  toastShellConnected: { he: 'מעטפת TypesetOK פעילה ומחוברת לליבת Rust', en: 'TypesetOK shell is active and connected to the Rust core' },
  toastLangSwitched: { he: 'שפת הממשק הוחלפה ל-{lang}', en: 'Interface language switched to {lang}' },
  toastLangUpdated: { he: 'שפת הממשק עודכנה: {lang}', en: 'Interface language updated: {lang}' },
  toastFlowSelected: { he: 'תזרים נבחר: {name}', en: 'Flow selected: {name}' },
  toastStyleApplied: { he: 'החלת סגנון: {name}', en: 'Style applied: {name}' },
  toastQuickStyle: { he: 'הוחל סגנון מהיר: {name}', en: 'Quick style applied: {name}' },
  toastLayerToggled: { he: 'שכבה {name}: {state}', en: 'Layer {name}: {state}' },
  toastGlobalStyleSynced: { he: 'הסגנון הגלובלי עודכן בהצלחה מכל השינויים המקומיים!', en: 'Global style updated from all local overrides!' },
  toastAlignApplied: { he: 'יישור אובייקטים הוחל: {type}', en: 'Alignment applied: {type}' },
  toastPreflightOk: { he: 'בדיקת דפוס: הכול תקין', en: 'Print check: all good' },
  toastMarginsToggled: { he: 'מתג קווי שוליים הופעל', en: 'Margin guides toggled' },
  toastBaselineToggled: { he: 'מתג רשת שורות בסיס הופעל', en: 'Baseline grid toggled' },
  toastPageAdded: { he: 'נוסף עמוד חדש: דף {page} (עמ\' {index})', en: 'New page added: {page} (p. {index})' },
  toastProjectCreated: { he: 'נוצר פרויקט חדש מתבנית: {name}', en: 'New project created from template: {name}' },
  toastDemoLoaded: { he: 'פרויקט לדוגמה נטען בהצלחה', en: 'Sample project loaded' },
  toastOpenFile: { he: 'פתיחת קובץ: {path}', en: 'Opening file: {path}' },
  toastSaved: { he: 'המסמך נשמר בהצלחה בפורמט .tok', en: 'Document saved in .tok format' },
  toastExporting: { he: 'מייצא לקובץ לדפוס ISO PDF/X-1a...', en: 'Exporting print-ready ISO PDF/X-1a...' },
  toastExportDone: { he: 'הייצוא לדפוס הושלם בהצלחה!', en: 'Print export completed!' },
  toastExportError: { he: 'שגיאת ייצוא: {error}', en: 'Export error: {error}' },
  toastExportSimulated: { he: 'הדמיית ייצוא: קובץ ISO PDF/X-1a הופק בהצלחה!', en: 'Export simulation: ISO PDF/X-1a file produced!' },
  toastNormalized: { he: 'נרמול ניקוד וטעמים ת"י 6100 הוחל בהצלחה על כל הפסקאות', en: 'SI 6100 niqqud & cantillation normalization applied to all paragraphs' },
  toastShieldOn: { he: 'מגן שמות קדושים הופעל (איסור שבירה בשמות הויה ואדנות)', en: 'Divine names shield enabled (no line breaks inside the Divine Names)' },
  toastGematriaSynced: { he: 'סנכרון מספור עמודים עברי בגימטריה הושלם', en: 'Hebrew gematria page numbering synced' },
  toastJustified: { he: 'יישור עברי מלא הוחל: רווחי מילים 85%-125% + מתיחת אהלתר"ם 120%', en: 'Full Hebrew justification applied: word spacing 85%–125% + Ahalterm stretch 120%' },
  // Redesign: shared
  densitySpacious: { he: 'מרווחת', en: 'Spacious' },
  topBarUndo: { he: 'ביטול פעולה', en: 'Undo' },
  topBarRedo: { he: 'ביצוע מחדש', en: 'Redo' },
  railLabel: { he: 'פאנלים', en: 'Panels' },
  railHidePanel: { he: 'הסתרת הפאנל', en: 'Hide panel' },
  canvasFit: { he: 'התאמה לחלון', en: 'Fit to window' },
  canvasZoomIn: { he: 'הגדלה', en: 'Zoom in' },
  canvasZoomOut: { he: 'הקטנה', en: 'Zoom out' },
  canvasZoomReset: { he: 'זום 100%', en: 'Zoom 100%' },
  canvasAria: { he: 'משטח עבודה', en: 'Canvas' },
  pageHeadTractate: { he: 'ברכות', en: 'ברכות' },
  pageHeadChapter: { he: 'פרק ראשון', en: 'פרק ראשון' },
  inspSelected: { he: 'נבחר', en: 'Selected' },
  inspNothingSelected: { he: 'מסמך', en: 'Document' },
  inspAlignTitle: { he: 'יישור', en: 'Alignment' },
  inspPosition: { he: 'מיקום בדף', en: 'Position on page' },
  storyFlowChip: { he: 'תזרים: {name}', en: 'Flow: {name}' },
  storyShowOnPage: { he: 'הצגה בדף', en: 'Show on page' },
  storyWordsCount: { he: '{n} מילים', en: '{n} words' },
  storyToolbar: { he: 'כלי עריכה', en: 'Editing tools' },
  storyFontSmaller: { he: 'הקטנת טקסט העורך', en: 'Smaller editor text' },
  storyFontLarger: { he: 'הגדלת טקסט העורך', en: 'Larger editor text' },
  splitResize: { he: 'שינוי רוחב החלוניות', en: 'Resize panes' },
  splitSynced: { he: 'מסונכרן עם העורך', en: 'Synced with the editor' },
  splitPageTitle: { he: 'תצוגת דף', en: 'Page view' },
  welcomeFirstRunTitle: { he: 'פעם ראשונה כאן?', en: 'First time here?' },
  welcomeFirstRunDesc: { he: 'פתחו את מסכת ברכות לדוגמה וראו איך נראה דף גמרא מעומד מקצה לקצה.', en: 'Open the sample tractate Berakhot to see a typeset Talmud page end to end.' },
  welcomeDropHint: { he: 'גררו לכאן קובץ .tok כדי לפתוח אותו', en: 'Drop a .tok file here to open it' },
  welcomeGuide: { he: 'מדריך למתחילים', en: 'Getting started' },
  welcomeVersion: { he: 'גרסה {v}', en: 'Version {v}' },
  welcomeMoreActions: { he: 'עוד', en: 'More' },
  templateMikraot: { he: 'מקראות גדולות', en: 'Mikraot Gedolot' },
  templateMikraotDesc: { he: 'פסוק ותרגום בראש הדף, ומפרשים בטורים מתחת.', en: 'Verse and Targum on top, commentaries in columns below.' },
  templateNotes: { he: 'ספר עם הערות', en: 'Book with notes' },
  templateNotesDesc: { he: 'טקסט ראשי עם הערות שוליים בתחתית כל עמוד.', en: 'Main text with footnotes at the bottom of each page.' },
  templateMetaGemara: { he: '17×24 ס״מ · 3 תזרימים', en: '17×24 cm · 3 flows' },
  templateMetaMikraot: { he: 'A4 · 5 תזרימים', en: 'A4 · 5 flows' },
  templateMetaProse: { he: '15×23 ס״מ · תזרים אחד', en: '15×23 cm · 1 flow' },
  templateMetaNotes: { he: '17×24 ס״מ · 2 תזרימים', en: '17×24 cm · 2 flows' },
  templateMetaBulletin: { he: 'A4 · 2 טורים', en: 'A4 · 2 columns' },
  appearanceTheme: { he: 'ערכת נושא', en: 'Theme' },
  appearanceMoreThemes: { he: 'ערכות צבע נוספות', en: 'More color themes' },
  appearanceAccentHint: { he: 'משמש לכפתורים, לבחירה ולסימון בדף.', en: 'Used for buttons, selection and highlights on the page.' },
  appearancePageView: { he: 'תצוגת הדף', en: 'Page view' },
  appearanceShowMargins: { he: 'קווי שוליים וטורים', en: 'Margin and column guides' },
  appearanceShowBaseline: { he: 'רשת שורות בסיס', en: 'Baseline grid' },
  themeSystem: { he: 'לפי המערכת', en: 'Match system' },
  settingsAutoSaved: { he: 'השינויים נשמרים אוטומטית', en: 'Changes are saved automatically' },
  settingsDone: { he: 'סיום', en: 'Done' },
  settingsCategories: { he: 'קטגוריות הגדרות', en: 'Settings categories' },
  paletteNavigateShort: { he: 'ניווט', en: 'Navigate' },
  paletteRunShort: { he: 'ביצוע', en: 'Run' },
  paletteResultsCount: { he: '{n} תוצאות', en: '{n} results' },
  exportTitle: { he: 'ייצוא לדפוס', en: 'Export for print' },
  exportSubtitle: { he: '{doc} · {n} עמודים', en: '{doc} · {n} pages' },
  exportFormat: { he: 'פורמט', en: 'Format' },
  exportX1aTitle: { he: 'לבית דפוס', en: 'for the print house' },
  exportX1aDesc: { he: 'שחור נקי, צבעי CMYK בלבד וגופנים מוטמעים. התקן המקובל בדפוס.', en: 'Clean black, CMYK only, embedded fonts. The standard print-house format.' },
  exportX4Title: { he: 'דפוס מתקדם', en: 'advanced print' },
  exportX4Desc: { he: 'תומך בשקיפויות ובצבעי ספוט. כדאי לבדוק מול בית הדפוס.', en: 'Supports transparency and spot colors. Check with your printer first.' },
  exportScreenTitle: { he: 'לתצוגה ולשליחה', en: 'for screen and sharing' },
  exportScreenDesc: { he: 'קובץ קל לקריאה על המסך, למייל או לאתר.', en: 'A light file for reading on screen, email or the web.' },
  exportRecommended: { he: 'מומלץ', en: 'Recommended' },
  exportPages: { he: 'עמודים', en: 'Pages' },
  exportAllPages: { he: 'כל המסמך', en: 'Whole document' },
  exportCurrentSpread: { he: 'כפולה נוכחית', en: 'Current spread' },
  exportRange: { he: 'טווח', en: 'Range' },
  exportRangeFrom: { he: 'מעמוד', en: 'From page' },
  exportRangeTo: { he: 'עד עמוד', en: 'To page' },
  exportMarks: { he: 'סימני דפוס', en: 'Printer marks' },
  exportBleed: { he: 'שולי גלישה של 3 מ״מ', en: '3 mm bleed' },
  exportCropMarks: { he: 'סימני חיתוך', en: 'Crop marks' },
  exportRegMarks: { he: 'סימני רישום וסרגל צבע', en: 'Registration marks and color bar' },
  exportProfile: { he: 'פרופיל צבע', en: 'Color profile' },
  exportProfileFogra39: { he: 'Fogra 39 · נייר מצופה, אופסט', en: 'Fogra 39 · coated paper, offset' },
  exportProfileFogra51: { he: 'Fogra 51 · נייר מצופה (PSO)', en: 'Fogra 51 · coated paper (PSO)' },
  exportProfileIsoCoated: { he: 'ISO Coated v2 · 100% K', en: 'ISO Coated v2 · 100% K' },
  exportFileName: { he: 'שם הקובץ', en: 'File name' },
  exportCancel: { he: 'ביטול', en: 'Cancel' },
  exportGo: { he: 'ייצוא PDF', en: 'Export PDF' },
  exportCheckTitle: { he: 'בדיקת דפוס', en: 'Print check' },
  exportCheckK: { he: 'שחור נקי (100% K)', en: 'Clean black (100% K)' },
  exportCheckFonts: { he: 'כל הגופנים מוטמעים', en: 'All fonts embedded' },
  exportCheckOverset: { he: 'אין טקסט גולש', en: 'No overset text' },
  exportCheckImages: { he: 'תמונות ברזולוציה תקינה', en: 'Image resolution OK' },
  exportCheckWarnings: { he: 'יש אזהרות', en: 'Has warnings' },
  exportCheckErrors: { he: 'יש שגיאות', en: 'Has errors' },
  exportPreview: { he: 'תצוגה מקדימה', en: 'Preview' },
  exportCheckAria: { he: 'בדיקה לפני ייצוא', en: 'Pre-export check' },
  exportBrowse: { he: 'עיון...', en: 'Browse...' },
  exportChooseLocation: { he: 'בחר מיקום שמירה', en: 'Choose export location' },
};

export function directionOf(lang: Language): 'rtl' | 'ltr' {
  return lang === 'he' ? 'rtl' : 'ltr';
}

class I18nManager {
  private currentLang: Language = 'he';
  private listeners: ((lang: Language) => void)[] = [];

  constructor() {
    try {
      const saved = localStorage.getItem('tok_lang');
      if (saved === 'en' || saved === 'he') {
        this.currentLang = saved;
      }
    } catch {}
    // index.html ships as <html dir="rtl" lang="he">. Sync it with the saved language
    // at startup too, not only on change; otherwise an English session keeps an RTL
    // document (toasts, scrollbars and anything outside #app laid out right-to-left).
    this.applyToDocument();
  }

  public getLanguage(): Language {
    return this.currentLang;
  }

  public getDirection(): 'rtl' | 'ltr' {
    return directionOf(this.currentLang);
  }

  private applyToDocument(): void {
    if (typeof document === 'undefined' || !document.documentElement) return;
    document.documentElement.lang = this.currentLang;
    document.documentElement.dir = this.getDirection();
  }

  public setLanguage(lang: Language): void {
    if (this.currentLang === lang) return;
    this.currentLang = lang;
    try {
      localStorage.setItem('tok_lang', lang);
    } catch {}

    // Update document HTML direction & lang
    this.applyToDocument();

    for (const listener of [...this.listeners]) {
      try {
        listener(lang);
      } catch (err) {
        console.error('[i18n] language listener failed:', err);
      }
    }
  }

  public toggleLanguage(): void {
    this.setLanguage(this.currentLang === 'he' ? 'en' : 'he');
  }

  public t(key: string): string {
    const entry = strings[key];
    if (!entry) return key;
    return entry[this.currentLang] || entry['he'] || key;
  }

  public onChange(listener: (lang: Language) => void): () => void {
    this.listeners.push(listener);
    return () => {
      this.listeners = this.listeners.filter(l => l !== listener);
    };
  }
}

export const i18n = new I18nManager();
export const t = (key: string) => i18n.t(key);

/** Translates `key` and substitutes `{name}` placeholders. */
export const tf = (key: string, vars: Record<string, string | number>) =>
  i18n.t(key).replace(/\{(\w+)\}/g, (m, name) => (name in vars ? String(vars[name]) : m));

/** Localized "enabled"/"disabled" state word. */
export const onOff = (on: boolean) => t(on ? 'stateOn' : 'stateOff');
