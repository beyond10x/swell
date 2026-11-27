/**
 * The compare page's only source of values: data/engines.json, written by `task site-data`
 * (taskfiles/site-engines.sh) from bench/engines.jsonl, bench/published/*.tsv, bench/results/*.json
 * and swell's committed files, and held by `task site-data-check`. Nothing here computes a number
 * the file does not hold; it reads, orders and formats.
 */
import raw from '@site/data/engines.json';

export type CellStatus = 'measured' | 'published' | 'documented' | 'not-measured' | 'not-applicable' | 'unknown';

export interface Source {
  label: string;
  url: string | null;
  urls?: string[];
  date: string | null;
  fetched?: string;
  revision: string;
  method: string;
  sha256?: string;
  file?: string;
}

export interface Cell {
  status: CellStatus;
  value?: number;
  of?: number;
  percent?: string;
  unit?: string | null;
  text?: string;
  note?: string;
  reason?: string;
  basis?: string;
  runs?: number;
  min?: number;
  max?: number;
  secondary?: {value: number; of: number; percent: string; label: string};
  /**
   * A figure of our runs: true when it was measured while another job ran on the build node (a
   * figure made of others, such as a geomean, when any of them was), false when measured alone;
   * absent when the result file does not say.
   */
  loaded?: boolean;
  /** Why it is loaded, when it is: neighbour, load or unwatched (bench/RUNBOOK.md); a list when more than one holds. */
  loaded_reason?: string | string[];
  /**
   * A figure of our runs: false when it may not rank (it was not measured alone); a comparison
   * that holds such a figure gets no mark "best of the picked". Absent on other sources.
   */
  ranks?: boolean;
  /** A derived figure (a geomean): what it is, in place of a median of runs with min and max. */
  aggregate?: string;
  source?: Source;
}

/** The reasons of a loaded cell as one line: "neighbour", "neighbour, load". */
export function loadedReasons(cell: Cell): string {
  const r = cell.loaded_reason;
  if (!r) return '';
  return Array.isArray(r) ? r.join(', ') : r;
}

export interface Row {
  id: string;
  kpi: string;
  group: string;
  label: string;
  unit: string | null;
  direction: 'higher' | 'lower' | null;
  kind: 'count' | 'measure' | 'text';
}

export interface Engine {
  id: string;
  engine: string;
  name: string;
  label: string;
  mode: 'interpreter' | 'jit' | 'aot';
  flags: string[];
  flags_verified: boolean;
  default_set: boolean;
  catalog_url: string;
  values: Record<string, Cell>;
}

export interface EnginesData {
  format: string;
  generated_by: string;
  inputs: Array<{path: string; sha256: string}>;
  published: Array<{
    file: string;
    path: string;
    data_commit: string;
    test262_revision: string;
    label: string;
    url: string;
    license: string;
    license_file: string;
  }>;
  swell_test262: {revision: string; source: string};
  default_selection: string[];
  default_set: string[];
  rows: Row[];
  engines: Engine[];
}

export const engines = raw as unknown as EnginesData;

/** A count as the README writes it: 44,812. */
export function count(value: number): string {
  return value.toLocaleString('en-US');
}

/** The configuration of an id, or undefined. */
export function engineById(id: string): Engine | undefined {
  return engines.engines.find((e) => e.id === id);
}

/** The KPI groups in the order of the rows, each with its rows. */
export function groups(): Array<{group: string; kpi: string; rows: Row[]}> {
  const out: Array<{group: string; kpi: string; rows: Row[]}> = [];
  for (const row of engines.rows) {
    const last = out[out.length - 1];
    if (last && last.group === row.group) last.rows.push(row);
    else out.push({group: row.group, kpi: row.kpi, rows: [row]});
  }
  return out;
}

/** The anchor of a KPI's section on the KPI page: `k1`, ..., `facts`. */
export function kpiAnchor(kpi: string): string {
  return kpi.toLowerCase();
}

/**
 * The best of the picked cells of a row, by the row's direction, among cells of the same basis
 * (the same runner and revision): a test262 count of swell's runner is never ranked against a
 * count of the zoo runner at another revision. A basis needs two picked cells to have a best.
 * A count with a total is ranked by its share of that total. A basis that holds a figure with
 * `ranks: false` (written by site-data for a figure not measured alone) has no best: a figure
 * measured beside another job does not rank, and nothing ranks against it. Returns the ids of
 * the best cells.
 */
export function bestOf(row: Row, picked: Engine[]): Set<string> {
  const best = new Set<string>();
  if (!row.direction) return best;
  const byBasis = new Map<string, Array<{id: string; score: number; ranks: boolean}>>();
  for (const e of picked) {
    const c = e.values[row.id];
    if (!c || typeof c.value !== 'number' || !c.basis) continue;
    if (c.status !== 'measured' && c.status !== 'published') continue;
    const score = typeof c.of === 'number' && c.of > 0 ? c.value / c.of : c.value;
    const list = byBasis.get(c.basis) ?? [];
    list.push({id: e.id, score, ranks: c.ranks !== false});
    byBasis.set(c.basis, list);
  }
  for (const list of byBasis.values()) {
    if (list.length < 2) continue;
    if (list.some((x) => !x.ranks)) continue;
    const top = row.direction === 'higher' ? Math.max(...list.map((x) => x.score)) : Math.min(...list.map((x) => x.score));
    for (const x of list) if (x.score === top) best.add(x.id);
  }
  return best;
}

/** The ids of a URL query `?e=a,b,c`, kept when they name a configuration, 2 to 4 of them. */
export function selectionFrom(search: string, allowJit: boolean): string[] {
  const query = new URLSearchParams(search).get('e');
  const ids = (query ?? '')
    .split(',')
    .map((s) => s.trim())
    .filter((id, i, all) => id && all.indexOf(id) === i)
    .filter((id) => {
      const e = engineById(id);
      return e !== undefined && (allowJit || e.mode !== 'jit');
    })
    .slice(0, 4);
  return ids.length >= 2 ? ids : engines.default_selection.filter((id) => engineById(id) !== undefined);
}
