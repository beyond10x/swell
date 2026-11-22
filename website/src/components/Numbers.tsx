import type {ReactNode} from 'react';
import CodeBlock from '@theme/CodeBlock';
import {at, count, deliveredWithException, deliverySlices, harnessException, passWords, slice, status} from '../data/status';

/**
 * The pages' numbers, each read from data/status.json (written by `task site-data`). A page
 * writes `<Num at="test262.passed" />` where a sentence states a count, so no count is typed.
 */

/** One value of status.json by its dotted path; a count is written with thousands separators. */
export function Num({at: path}: {at: string}): ReactNode {
  const value = at(path);
  if (typeof value === 'number') return <>{count(value)}</>;
  if (typeof value === 'string') return <>{value}</>;
  throw new Error(`status.json ${path} is not a number or a string`);
}

/** A field of one delivery slice: `<S n={3} f="cases" />`. */
export function S({n, f}: {n: number; f: 'cases' | 'runs' | 'run' | 'passed' | 'failed' | 'passed_percent' | 'name'}): ReactNode {
  const value = slice(n)[f];
  return <>{typeof value === 'number' ? count(value) : value}</>;
}

/** Whether a delivery slice is delivered, in words and a glyph. */
export function Delivered({n}: {n: number}): ReactNode {
  return slice(n).delivered ? <><span aria-hidden="true">● </span>delivered</> : <><span aria-hidden="true">○ </span>not yet</>;
}

/**
 * One sentence per delivered slice with a failed case: its count, and why the case does not pass,
 * from data/status.ts's harnessException. Nothing when every delivered slice passes every case.
 */
export function HarnessException(): ReactNode {
  if (!deliveredWithException.length) return null;
  return (
    <>
      {deliveredWithException.map((s) => (
        <p key={s.slice} className="sw-exception">
          Slice {Number(s.slice)}, <code>{s.name}</code>: {passWords(s)}; the {s.failed === 1 ? 'case that does not pass' : `${count(s.failed)} cases that do not pass`}{' '}
          {s.failed === 1 ? harnessException.text : harnessException.plural} (<a href={harnessException.href}>{harnessException.link}</a>).
        </p>
      ))}
    </>
  );
}

/** Host requirements still pending for a slice, or 0. */
export function Pending({n}: {n: number}): ReactNode {
  const found = status.host_requirements.pending_by_slice.find((p) => p.slice === n);
  return <>{count(found ? found.count : 0)}</>;
}

/** The dependency line of a program that uses swell, with the workspace version as its tag. */
export function DependencyLine(): ReactNode {
  return (
    <CodeBlock language="toml" title="Cargo.toml">
      {`[dependencies]\nswell = { git = "https://github.com/beyond10x/swell", tag = "${status.version}" }`}
    </CodeBlock>
  );
}

/**
 * Every delivery slice as a segment as wide as its cases, filled as far as its cases pass. The
 * part that does not pass is hatched, so what is missing is drawn as plainly as what works.
 */
export function SliceBar(): ReactNode {
  return (
    <figure className="sw-slicebar" aria-labelledby="sw-slicebar-caption">
      <ol className="sw-slicebar__bar" aria-label="Delivery slices, each as wide as its cases">
        {deliverySlices.map((s) => (
          <li key={s.slice} style={{flexGrow: s.cases}} className={s.delivered ? 'is-delivered' : undefined}>
            <span className="sw-slicebar__fill" style={{inlineSize: `${s.cases ? (100 * s.passed) / s.cases : 0}%`}} aria-hidden="true" />
            <span className="sw-sr">
              slice {Number(s.slice)}, {s.name}: {count(s.passed)} of {count(s.cases)} cases pass{s.delivered ? ', delivered' : ''}
            </span>
          </li>
        ))}
      </ol>
      <p className="sw-slicebar__legend" aria-hidden="true">
        <span><span className="sw-swatch sw-swatch--passed" />passes</span>
        <span><span className="sw-swatch sw-swatch--failed" />does not pass yet</span>
        <span><span className="sw-swatch sw-swatch--delivered" />delivered slice</span>
      </p>
      <figcaption id="sw-slicebar-caption" className="sw-slicebar__caption">
        Slices 1 to {status.delivery_slices}, left to right, each as wide as its share of their cases. Slice 1 has{' '}
        {count(slice(1).cases)} cases and is too narrow to see.
      </figcaption>
    </figure>
  );
}

/** The host requirements still pending, by the slice that closes them. */
export function PendingTable(): ReactNode {
  const pending = status.host_requirements.pending_by_slice;
  return (
    <ul className="sw-pending" aria-label="Host requirements pending, by the slice that closes them">
      {pending.map((p) => (
        <li key={p.slice}><span className="sw-pending__slice">slice {p.slice}</span><strong>{count(p.count)}</strong></li>
      ))}
    </ul>
  );
}

/** The crates in Cargo.lock and what each depends on. */
export function Crates(): ReactNode {
  return (
    <table>
      <thead>
        <tr><th scope="col">crate</th><th scope="col">depends on</th></tr>
      </thead>
      <tbody>
        {status.crates.packages.map((p) => (
          <tr key={p.name}>
            <th scope="row"><code>{p.name}</code></th>
            <td>{p.dependencies.length ? p.dependencies.map((d, i) => <span key={d}>{i ? ', ' : ''}<code>{d}</code></span>) : 'nothing'}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
