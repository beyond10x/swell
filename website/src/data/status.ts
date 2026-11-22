/**
 * The site's only source of numbers: data/status.json, written by `task site-data` from the
 * committed gate files (taskfiles/site.yml) and held by `task site-data-check`. Nothing here
 * computes a number the file does not hold; it only reads and formats.
 */
import raw from '@site/data/status.json';

export interface SliceCount {
  slice: string;
  name: string;
  delivery: boolean;
  delivered: boolean;
  line: string;
  cases: number;
  runs: number;
  excluded: number;
  not_applicable: number;
  run: number;
  passed: number;
  failed: number;
  budget: number;
  passed_percent: string;
}

export interface SiteStatus {
  format: string;
  generated_by: string;
  version: string;
  crates: {
    roots: number;
    roots_forbidding_unsafe: number;
    lock_packages: number;
    lock_packages_outside: number;
    packages: Array<{name: string; dependencies: string[]}>;
  };
  test262: {
    line: string;
    exist: number;
    excluded: number;
    not_applicable: number;
    run: number;
    passed: number;
    failed: number;
    budget: number;
    passed_percent: string;
  };
  delivery_slices: number;
  delivered: number[];
  delivered_count: number;
  slices: SliceCount[];
  host_requirements: {
    total: number;
    test: number;
    pending: number;
    omitted: number;
    pending_by_slice: Array<{slice: number; count: number}>;
  };
  moves: {cases: number; by_target_slice: Array<{slice: number; count: number}>};
}

export const status = raw as SiteStatus;

/** A count as the README writes it: 44,812. */
export function count(value: number): string {
  return value.toLocaleString('en-US');
}

/** The delivery slices, 1 to 19, in order. */
export const deliverySlices: SliceCount[] = status.slices.filter((s) => s.delivery);

/** The slice that follows the last delivered one, or undefined. */
export const nextSlice: SliceCount | undefined = deliverySlices.find((s) => !s.delivered);

/**
 * What the site says of a delivered slice whose failed count is not 0, and where the design
 * rules on it (docs/design/engine.md, section 12, paragraph "Slice 3's cases that need a later
 * slice"). The one place this sentence is written.
 */
export const harnessException = {
  text: 'fails in the pinned test262 harness before the engine’s answer is judged',
  plural: 'fail in the pinned test262 harness before the engine’s answer is judged',
  href: 'https://github.com/beyond10x/swell/blob/main/docs/design/engine.md#12-delivery-order',
  link: 'design, section 12',
};

/** Delivered slices with a failed case: the slices the exception sentence is shown for. */
export const deliveredWithException: SliceCount[] = deliverySlices.filter((s) => s.delivered && s.failed > 0);

/** A slice's count in words, as status.json has it: "31 cases, every one passes" or "7,010 of 7,011 cases pass". */
export function passWords(s: SliceCount): string {
  return s.failed === 0 && s.passed === s.cases ? `${count(s.cases)} cases, every one passes` : `${count(s.passed)} of ${count(s.cases)} cases pass`;
}

/** The slice record of a slice number. */
export function slice(number: number): SliceCount {
  const found = status.slices.find((s) => s.delivery && Number(s.slice) === number);
  if (!found) throw new Error(`status.json has no delivery slice ${number}`);
  return found;
}

/** A value of status.json by a dotted path, `test262.passed` or `slices.2.cases`. */
export function at(path: string): unknown {
  let value: unknown = status;
  for (const part of path.split('.')) {
    if (value === null || typeof value !== 'object') throw new Error(`status.json has no ${path}`);
    value = (value as Record<string, unknown>)[part];
  }
  if (value === undefined) throw new Error(`status.json has no ${path}`);
  return value;
}
