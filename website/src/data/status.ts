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
