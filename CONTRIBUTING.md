# מדריך לתורמים — Contributing to TypesetOK (TOK)

<div align="center">
  <b>[ <a href="#-מדריך-לתורמים-עברית">עברית</a> | <a href="#-contributors-guide-english">English</a> ]</b>
</div>

---

## 🇮🇱 מדריך לתורמים (עברית)

ברוכים הבאים לקהילת הפיתוח של **TypesetOK (TOK)**!  
אנו מקדמים בברכה תרומות קוד, עיצוב, תיעוד, בדיקות טיפוגרפיות ומשוב ממומחי עימוד ודפוס עברי.

### 📋 עקרונות יסוד ומשמעת טיפוגרפית (Invariants)
כל תרומת קוד למערכת מחויבת לציית לחוקי הברזל הבאים:
1. **נרמול עברי קנוני (Canonical Hebrew Normalization — Unicode NFC + סדר מסורה/SBL):** כל טקסט עברי המוזן למערכת עובר נרמול דטרמיניסטי:  
   `אות בסיס ← נקודת שין/שין ← דגש/מפיק ← ניקוד ← מתג ← טעמי מקרא`.
2. **איסור מוחלט על כשידה ערבית (`Tatweel U+0640`):** יישור עברי נעשה אך ורק לפי מדרג 3 השלבים: רווחי מילים $\rightarrow$ מתיחת אותיות אהלתר״ם (א, ה, ל, ת, ר, ם) $\rightarrow$ מיקרו-טרקינג.
3. **מגן שמות קדושים:** חל איסור מוחלט על שבירת שורה או פיצול אותיות בשמות הוי"ה, אדנות ושמות קודש.
4. **מספור עברי בגימטריה:** שימוש אך ורק בגרש עברי תקני (`U+05F3`, ׳) ובגרשיים עבריים תקניים (`U+05F4`, ״), תוך אכיפת המרות טאבו מסורתיות (15 $\rightarrow$ ט״ו, 16 $\rightarrow$ ט״ז, 270 $\rightarrow$ ע״ר, 272 $\rightarrow$ ער״ב וכו').
5. **דטרמיניזם מלא (Bit-for-Bit Determinism):** הרצה חוזרת של העימוד על אותו קלט חייבת לייצר קובץ PDF עם חתימת SHA-256 זהה לחלוטין.

### 🛠️ סביבת הפיתוח והרצת בדיקות

#### דרישות מקדימות:
- **Rust 1.85+** עם `cargo`, `rustfmt`, ו-`clippy`.
- **Node.js 20+** ו-`npm`.

#### בדיקות חובה לפני הגשת Pull Request:
לפני יצירת PR יש לוודא כי כל הבדיקות עוברות באופן נקי:
```bash
# 1. בדיקות ליבת Rust
cargo test --workspace

# 2. בדיקת עיצוב קוד Rust
cargo fmt --check

# 3. בדיקת Clippy ללא אזהרות
cargo clippy --workspace --all-targets -- -D warnings

# 4. בדיקת טיפוסים ובדיקות Frontend ואינטגרציה
npm run lint
npm test

# 5. קומפילציית TypeScript וחבילת UI
npm run build
```

### 🔀 תהליך הגשת תרומה (Git Workflow)
1. פתח ענף חדש מ-`main`:
   ```bash
   git checkout -b feature/your-feature-name
   # או
   git checkout -b fix/your-bug-fix
   ```
2. בצע קומיטים בעלי הודעות ברורות לפי תקן [Conventional Commits](https://www.conventionalcommits.org):
   - `feat(typeset): ...`
   - `fix(pdf): ...`
   - `docs(readme): ...`
   - `test(canvas): ...`
3. פתח Pull Request ב-GitHub עם פירוט השינויים, מטרתם ובדיקות האימות שבוצעו.

---

## 🇺🇸 Contributors Guide (English)

Thank you for your interest in contributing to **TypesetOK (TOK)**!  
We welcome contributions from Rust developers, frontend engineers, font designers, and Hebrew DTP professionals.

### 📋 Core Typography Invariants
All contributions must adhere to the following strict invariants:
1. **Canonical Hebrew Unicode Normalization (NFC + Masoretic/SBL Ordering):** Mandatory canonical order:  
   `Base Consonant -> Shin/Sin Dot -> Dagesh/Mapiq -> Niqqud -> Meteg -> Te'amim`.
2. **Strict Rejection of Arabic Tatweel (`U+0640`):** Hebrew line justification uses only the 3-Tier engine: Word Spacing $\rightarrow$ Ahalterm Letter Stretching (Alef, He, Lamed, Tav, Resh, Final Mem) $\rightarrow$ Micro-tracking.
3. **Holy Names Protection:** No hyphenation or word-breaking across divine Hebrew names.
4. **Deterministic Gematria:** Uses native Hebrew geresh (`U+05F3`) and gershayim (`U+05F4`) with required traditional taboo substitutions (15 $\rightarrow$ ט״ו, 16 $\rightarrow$ ט״ז, etc.).
5. **Bit-for-Bit Determinism:** Output must produce identical SHA-256 hashes across identical runs.

### 🛠️ Verification Steps Before Submitting
```bash
# Rust test suite
cargo test --workspace

# Formatting & Clippy linter
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings

# TypeScript type-check, tests & build
npm run lint
npm test
npm run build
```
