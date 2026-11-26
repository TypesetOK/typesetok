import { hostSettings } from '../platform/hostSettings.js';
import type { TemplateType } from '../engine/FlowPaginator';

export type BaseTemplateType = Exclude<TemplateType, 'blank'>;

export interface CustomTemplate {
  id: string;
  name: string;
  description?: string;
  baseTemplate: BaseTemplateType;
  pageSize: '17x24' | 'A4' | 'B5' | 'Crown';
  columns: number;
  fontFamily: string;
  fontSizePt: number;
  lineHeightPt: number;
  createdAt: string;
}

const STORAGE_KEY = 'tok_custom_templates';

export function getCustomTemplates(): CustomTemplate[] {
  try {
    const raw = hostSettings.get(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(
      (t): t is CustomTemplate =>
        Boolean(t && typeof t.id === 'string' && typeof t.name === 'string' && typeof t.baseTemplate === 'string')
    );
  } catch (err) {
    console.warn('[TEMPLATES] Failed to read custom templates:', err);
    return [];
  }
}

export function getCustomTemplateById(id: string): CustomTemplate | undefined {
  return getCustomTemplates().find((t) => t.id === id);
}

export function addCustomTemplate(input: Omit<CustomTemplate, 'id' | 'createdAt'>): CustomTemplate {
  const list = getCustomTemplates();
  const created: CustomTemplate = {
    ...input,
    id: `custom-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
    createdAt: new Date().toISOString(),
  };
  list.push(created);
  try {
    hostSettings.set(STORAGE_KEY, JSON.stringify(list));
  } catch (err) {
    console.warn('[TEMPLATES] Failed to save custom template:', err);
  }
  return created;
}

export function deleteCustomTemplate(id: string): void {
  const list = getCustomTemplates().filter((t) => t.id !== id);
  try {
    hostSettings.set(STORAGE_KEY, JSON.stringify(list));
  } catch (err) {
    console.warn('[TEMPLATES] Failed to delete custom template:', err);
  }
}
