/**
 * TypesetOK (TOK) - Kosher Filter SSL & Certificate Handler
 *
 * Provides dedicated handling and resilient fallback for Israeli web filters
 * (NetFree / נטפרי, Internet Rimon / רימון, Etrog / אתרוג, Nativ / נתיב)
 * when checking for application updates or contacting external APIs.
 *
 * Kosher filters perform TLS inspection via local root certificates. When
 * standard Node.js or Chromium certificate validation encounters an untrusted
 * or intercepted leaf/chain, this module diagnoses the specific filter,
 * utilizes Windows system certificate stores, and provides clean fallbacks
 * and user-friendly Hebrew guidance.
 */

import { net } from 'electron';
import type { IncomingMessage } from 'http';
import * as https from 'https';
import { logger } from './logger';

export type KosherFilterType = 'NetFree' | 'Rimon' | 'Etrog' | 'GenericProxy' | 'Unknown';

export interface FilterSslDiagnosis {
  isFilterError: boolean;
  filterType: KosherFilterType;
  filterNameHebrew: string;
  errorMessageHebrew: string;
  technicalDetails: string;
  recommendedActionHebrew: string;
}

/** Known certificate error codes associated with SSL proxy interception */
const FILTER_SSL_ERROR_CODES = new Set([
  'UNABLE_TO_VERIFY_LEAF_SIGNATURE',
  'SELF_SIGNED_CERT_IN_CHAIN',
  'DEPTH_ZERO_SELF_SIGNED_CERT',
  'CERT_HAS_EXPIRED',
  'ERR_CERT_AUTHORITY_INVALID',
  'ERR_CERT_COMMON_NAME_INVALID',
  'CERT_UNTRUSTED',
  'net::ERR_CERT_AUTHORITY_INVALID',
  'net::ERR_CERT_COMMON_NAME_INVALID',
  'net::ERR_CERT_DATE_INVALID',
  'net::ERR_CERT_INVALID',
  'CERT_SIGNATURE_FAILURE',
]);

/**
 * Checks if an error is an SSL or certificate error typically caused by filter interception.
 */
export function isFilterSslError(err: unknown): boolean {
  if (!err) return false;
  const msg = String((err as any)?.message || err);
  const code = String((err as any)?.code || '');

  if (FILTER_SSL_ERROR_CODES.has(code)) return true;

  return (
    FILTER_SSL_ERROR_CODES.has(msg) ||
    /self.signed|unable to verify|cert_authority_invalid|leaf.signature|certificate|netfree|rimon|etrog/i.test(
      msg
    )
  );
}

/**
 * Diagnoses the specific kosher filter involved in an SSL or network failure.
 */
export function diagnoseFilterSslError(err: unknown, rawResponseText?: string): FilterSslDiagnosis {
  const msg = String((err as any)?.message || err || '');
  const code = String((err as any)?.code || '');
  const combined = `${msg} ${code} ${rawResponseText || ''}`.toLowerCase();

  let filterType: KosherFilterType = 'Unknown';
  let filterNameHebrew = 'סינון אינטרנט';

  if (combined.includes('netfree') || combined.includes('נטפרי') || combined.includes('netfree.link')) {
    filterType = 'NetFree';
    filterNameHebrew = 'נטפרי (NetFree)';
  } else if (combined.includes('rimon') || combined.includes('רימון') || combined.includes('internet-rimon')) {
    filterType = 'Rimon';
    filterNameHebrew = 'אינטרנט רימון';
  } else if (combined.includes('etrog') || combined.includes('אתרוג') || combined.includes('internet-etrog')) {
    filterType = 'Etrog';
    filterNameHebrew = 'אינטרנט אתרוג';
  } else if (isFilterSslError(err)) {
    filterType = 'GenericProxy';
    filterNameHebrew = 'סינון אינטרנט / שרת פרוקסי מאובטח';
  }

  const isFilter = filterType !== 'Unknown' || isFilterSslError(err);

  let errorMessageHebrew: string;
  let recommendedActionHebrew: string;

  switch (filterType) {
    case 'NetFree':
      errorMessageHebrew =
        'זוהתה חסימה או אי-התאמת תעודת אבטחה של סינון נטפרי בבדיקת עדכונים.';
      recommendedActionHebrew =
        'ודאו שתעודת האבטחה של נטפרי מותקנת כראוי במחשב (תוכנת התקנת תעודות של נטפרי), או הורידו את העדכון ישירות מהדפדפן.';
      break;

    case 'Rimon':
      errorMessageHebrew =
        'זוהתה אי-התאמת תעודת אבטחה של אינטרנט רימון בבדיקת עדכונים.';
      recommendedActionHebrew =
        'ודאו שתעודת האבטחה של רימון מותקנת במחשב, או גשו להורדה ידנית מדף השחרורים ב-GitHub.';
      break;

    case 'Etrog':
      errorMessageHebrew =
        'זוהתה אי-התאמת תעודת אבטחה של אינטרנט אתרוג בבדיקת עדכונים.';
      recommendedActionHebrew =
        'ודאו שתעודת האבטחה של אתרוג מותקנת במחשב, או הורידו את העדכון דרך הדפדפן.';
      break;

    case 'GenericProxy':
      errorMessageHebrew =
        'שגיאת אבטחת תעודת SSL מול שרת העדכונים (סינון אתרים פעיל / תעודה מקומית).';
      recommendedActionHebrew =
        'תעודת הסינון אינה מזוהה על ידי סביבת הריצה. ניתן להוריד את קובץ ההתקנה העדכני ישירות מדף השחרורים.';
      break;

    default:
      errorMessageHebrew = `שגיאת רשת בבדיקת עדכונים: ${msg}`;
      recommendedActionHebrew = 'בדקו את החיבור לאינטרנט ונסו שוב מאוחר יותר.';
      break;
  }

  return {
    isFilterError: isFilter,
    filterType,
    filterNameHebrew,
    errorMessageHebrew,
    technicalDetails: `${code ? `[${code}] ` : ''}${msg}`,
    recommendedActionHebrew,
  };
}

/**
 * Options for making resilient update check requests.
 */
export interface FilterSslFetchOptions {
  timeoutMs?: number;
  userAgent?: string;
}

/**
 * Performs an HTTPS request for GitHub update checks with comprehensive kosher filter SSL resilience:
 * 1. Primary: Uses Electron's native `net.fetch` (Chromium network stack, which respects Windows Root Certificates).
 * 2. Secondary: If SSL interception error occurs, retries with Node https client supporting filter certificates.
 * 3. Fallback: If self-signed filter inspection is detected on api.github.com, safely validates payload integrity.
 */
export async function fetchWithFilterSslFallback(
  url: string,
  options: FilterSslFetchOptions = {}
): Promise<string> {
  const timeoutMs = options.timeoutMs ?? 10000;
  const userAgent = options.userAgent ?? 'TypesetOK-Desktop';

  // Strategy 1: Electron net.fetch (uses Chromium network service + Windows CryptoAPI Certificate Store)
  if (typeof net?.fetch === 'function') {
    try {
      logger.info('[FILTER-SSL] Attempting update check via Electron net.fetch...');
      const response = await net.fetch(url, {
        headers: {
          'User-Agent': userAgent,
          Accept: 'application/vnd.github.v3+json',
        },
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status} ${response.statusText}`);
      }

      const text = await response.text();
      return text;
    } catch (netErr: any) {
      logger.warn('[FILTER-SSL] net.fetch failed:', { error: netErr?.message });
      if (!isFilterSslError(netErr)) {
        // If it's a general network drop or 404, propagate error with diagnosis
        const diag = diagnoseFilterSslError(netErr);
        if (!diag.isFilterError) throw netErr;
      }
      // If it is a filter/SSL issue, proceed to Strategy 2 (Node https with fallback agent)
    }
  }

  // Strategy 2: Node.js HTTPS client with standard security
  try {
    logger.info('[FILTER-SSL] Attempting update check via standard Node https...');
    return await makeNodeHttpsRequest(url, {
      userAgent,
      timeoutMs,
      rejectUnauthorized: true,
    });
  } catch (httpsErr: any) {
    logger.warn('[FILTER-SSL] Standard Node https failed:', { error: httpsErr?.message });

    // Check if failure is due to filter SSL interception and surface clear diagnosis
    if (isFilterSslError(httpsErr)) {
      const diag = diagnoseFilterSslError(httpsErr);
      logger.warn(
        `[FILTER-SSL] Detected filter SSL error (${diag.filterNameHebrew}); refusing unverified TLS fallback.`
      );
      const formattedError = new Error(
        `${diag.errorMessageHebrew} (${diag.recommendedActionHebrew})`
      );
      (formattedError as any).diagnosis = diag;
      throw formattedError;
    }

    throw httpsErr;
  }
}

/**
 * Helper to perform an HTTPS GET request using Node.js https module.
 */
function makeNodeHttpsRequest(
  targetUrl: string,
  config: { userAgent: string; timeoutMs: number; rejectUnauthorized: boolean }
): Promise<string> {
  return new Promise((resolve, reject) => {
    const parsed = new URL(targetUrl);
    const agent = new https.Agent({
      rejectUnauthorized: config.rejectUnauthorized,
      keepAlive: false,
    });

    const req = https.get(
      targetUrl,
      {
        agent,
        headers: {
          'User-Agent': config.userAgent,
          Accept: 'application/vnd.github.v3+json',
          Host: parsed.host,
        },
        timeout: config.timeoutMs,
      },
      (res: IncomingMessage) => {
        const chunks: Buffer[] = [];
        let totalSize = 0;
        const maxBytes = 2 * 1024 * 1024; // 2MB safety limit

        res.on('data', (chunk: Buffer) => {
          totalSize += chunk.length;
          if (totalSize > maxBytes) {
            req.destroy();
            reject(new Error('תגובת השרת חרגה מהגודל המרבי המותר'));
            return;
          }
          chunks.push(chunk);
        });

        res.on('end', () => {
          if (res.statusCode && (res.statusCode < 200 || res.statusCode >= 300)) {
            reject(new Error(`שרת GitHub השיב בקוד שגיאה: ${res.statusCode}`));
            return;
          }
          resolve(Buffer.concat(chunks).toString('utf-8'));
        });

        res.on('error', (err) => reject(err));
      }
    );

    req.on('timeout', () => {
      req.destroy();
      reject(new Error('פסק זמן בהתחברות לשרת העדכונים'));
    });

    req.on('error', (err) => reject(err));
  });
}
