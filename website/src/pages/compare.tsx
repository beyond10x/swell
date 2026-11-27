import type {ReactNode} from 'react';
import Layout from '@theme/Layout';
import Link from '@docusaurus/Link';
import {useHistory, useLocation} from '@docusaurus/router';
import useIsBrowser from '@docusaurus/useIsBrowser';
import {bestOf, count, engineById, engines, groups, kpiAnchor, loadedReasons, selectionFrom} from '../data/engines';
import type {Cell, Engine, Row} from '../data/engines';

/**
 * The compare page: 2 to 4 engine configurations side by side, rows grouped by KPI. Every value
 * and every reason is read from data/engines.json (src/data/engines.ts); the page types no
 * number. The selection is the URL query `?e=`, so a link opens on the exact comparison; JIT
 * configurations are offered only with `jit=1`, and shown as "JIT reference".
 */

const MIN = 2;
const MAX = 4;

function modeLabel(e: Engine): string {
  if (e.mode === 'jit') return 'JIT reference';
  if (e.mode === 'aot') return 'ahead-of-time';
  return 'interpreter';
}

function SourceLine({cell}: {cell: Cell}): ReactNode {
  const s = cell.source;
  if (!s) return null;
  const when = s.date ? s.date : 'as of this site’s commit';
  const detail = [when, s.revision, s.method, s.fetched ? `fetched ${s.fetched}` : '', s.sha256 ? `sha256 ${s.sha256}` : '', s.file ?? '']
    .filter(Boolean)
    .join(' · ');
  return (
    <div className="sw-cmp-source" title={detail}>
      {s.url ? <a href={s.url}>{s.label}</a> : <span>{s.label}</span>}
      <span className="sw-cmp-when"> · {when}</span>
    </div>
  );
}

const LOADED_MEANS = 'the figure was measured while another job ran on the build node';

/** The mark beside a figure of our runs that was measured beside another job; nothing otherwise. */
function LoadedMark({cell}: {cell: Cell}): ReactNode {
  if (cell.loaded !== true) return null;
  const why = loadedReasons(cell);
  return (
    <span className="sw-cmp-loaded" title={`loaded${why ? ` (${why})` : ''}: ${LOADED_MEANS}`}>
      loaded{why ? `: ${why}` : ''}
    </span>
  );
}

function Value({row, cell}: {row: Row; cell: Cell}): ReactNode {
  if (cell.status === 'not-measured' || cell.status === 'not-applicable' || (cell.status === 'unknown' && !cell.text)) {
    const word = cell.status === 'not-measured' ? 'not measured' : cell.status === 'not-applicable' ? 'not applicable' : 'unknown';
    return (
      <>
        <span className="sw-cmp-missing">{word}</span>
        {cell.reason ? <div className="sw-cmp-reason">{cell.reason}</div> : null}
      </>
    );
  }
  if (typeof cell.value === 'number' && typeof cell.of === 'number') {
    return (
      <>
        <span className="sw-cmp-number">{count(cell.value)}</span> of {count(cell.of)}
        {cell.percent ? <span className="sw-cmp-percent"> · {cell.percent} %</span> : null}
        <LoadedMark cell={cell} />
        {cell.secondary ? (
          <div className="sw-cmp-secondary">
            {count(cell.secondary.value)} of {count(cell.secondary.of)} {cell.secondary.label}, {cell.secondary.percent} %
          </div>
        ) : null}
      </>
    );
  }
  if (typeof cell.value === 'number') {
    return (
      <>
        <span className="sw-cmp-number">{count(cell.value)}</span> {cell.unit ?? row.unit ?? ''}
        <LoadedMark cell={cell} />
        {cell.aggregate ? <div className="sw-cmp-secondary">{cell.aggregate}</div> : null}
        {typeof cell.runs === 'number' ? (
          <div className="sw-cmp-secondary">
            median of {count(cell.runs)} runs
            {typeof cell.min === 'number' && typeof cell.max === 'number' ? `, ${count(cell.min)} to ${count(cell.max)}` : ''}
          </div>
        ) : null}
      </>
    );
  }
  return (
    <>
      <span className={cell.status === 'unknown' ? 'sw-cmp-missing' : undefined}>{cell.text}</span>
      {cell.note ? <div className="sw-cmp-reason">{cell.note}</div> : null}
    </>
  );
}

function direction(row: Row): string {
  if (row.direction === 'higher') return 'higher is better';
  if (row.direction === 'lower') return 'lower is better';
  return '';
}

export default function Compare(): ReactNode {
  const location = useLocation();
  const history = useHistory();
  // The static page is built without a query; the query is read after hydration, so the first
  // render in the browser equals the built page.
  const search = useIsBrowser() ? location.search : '';
  const params = new URLSearchParams(search);
  const requested = selectionFrom(search, true);
  const showJit = params.get('jit') === '1' || requested.some((id) => engineById(id)?.mode === 'jit');
  const picked = requested.map((id) => engineById(id)).filter((e): e is Engine => e !== undefined);
  const pickedIds = picked.map((e) => e.id);

  const go = (ids: string[], jit: boolean) => {
    const q = new URLSearchParams();
    q.set('e', ids.join(','));
    if (jit) q.set('jit', '1');
    history.replace({pathname: location.pathname, search: `?${q.toString().replace(/%2C/g, ',')}`});
  };
  const toggle = (id: string) => {
    if (pickedIds.includes(id)) {
      if (pickedIds.length > MIN) go(pickedIds.filter((x) => x !== id), showJit);
    } else if (pickedIds.length < MAX) {
      go([...pickedIds, id], showJit);
    }
  };
  const toggleJit = () => {
    const next = !showJit;
    const ids = next ? pickedIds : pickedIds.filter((id) => engineById(id)?.mode !== 'jit');
    go(ids.length >= MIN ? ids : engines.default_selection, next);
  };

  const offered = engines.engines.filter((e) => showJit || e.mode !== 'jit');
  const swell = offered.filter((e) => e.id === 'swell');
  const famous = offered.filter((e) => e.default_set);
  const others = offered.filter((e) => e.id !== 'swell' && !e.default_set);

  const picker = (title: string, list: Engine[]) => (
    <fieldset className="sw-cmp-pick-group">
      <legend>{title}</legend>
      {list.map((e) => {
        const on = pickedIds.includes(e.id);
        const disabled = (on && pickedIds.length <= MIN) || (!on && pickedIds.length >= MAX);
        return (
          <label key={e.id} className={on ? 'sw-cmp-pick sw-cmp-pick--on' : 'sw-cmp-pick'}>
            <input type="checkbox" checked={on} disabled={disabled} onChange={() => toggle(e.id)} /> {e.label}
            {e.mode === 'jit' ? <span className="sw-cmp-tag">JIT</span> : null}
          </label>
        );
      })}
    </fieldset>
  );

  const published = engines.published;
  return (
    <Layout title="Compare engines" description="swell beside other ECMAScript engines, KPI by KPI, each value with its source.">
      <main className="container margin-vert--lg sw-cmp">
        <h1>Compare engines</h1>
        <p>
          Pick {MIN} to {MAX} engine configurations. Each row is a KPI of the <Link to="/docs/kpis">KPI page</Link>, which says how it is
          measured and where each value comes from. Every cell links to its source. Values of two test262 revisions or two runners
          are shown side by side but not ranked against each other; the best of the picked cells is marked only among cells of the
          same source.
        </p>
        <div className="sw-cmp-picker">
          {picker('swell', swell)}
          {picker('Default set', famous)}
          {picker('Other engines', others)}
          <label className="sw-cmp-jit">
            <input type="checkbox" checked={showJit} onChange={toggleJit} /> Offer JIT configurations (shown as “JIT reference”)
          </label>
        </div>
        <div className="sw-cmp-scroll">
          <table className="sw-cmp-table">
            <thead>
              <tr>
                <th scope="col">KPI</th>
                {picked.map((e) => (
                  <th scope="col" key={e.id}>
                    <a href={e.catalog_url}>{e.label}</a>
                    <div className="sw-cmp-mode">
                      {modeLabel(e)}
                      {e.flags.length ? ` · ${e.flags.join(' ')}` : ''}
                      {e.flags.length && !e.flags_verified ? ' (flags unverified)' : ''}
                    </div>
                  </th>
                ))}
              </tr>
            </thead>
            {groups().map((g) => (
              <tbody key={g.group}>
                <tr className="sw-cmp-group">
                  <th scope="rowgroup" colSpan={picked.length + 1}>
                    <Link to={`/docs/kpis#${kpiAnchor(g.kpi)}`}>
                      {g.kpi === 'facts' ? '' : `${g.kpi} · `}
                      {g.group}
                    </Link>
                  </th>
                </tr>
                {g.rows.map((row) => {
                  const best = bestOf(row, picked);
                  return (
                    <tr key={row.id}>
                      <th scope="row">
                        <Link to={`/docs/kpis#${kpiAnchor(row.kpi)}`}>{row.label}</Link>
                        {direction(row) ? <div className="sw-cmp-direction">{direction(row)}</div> : null}
                      </th>
                      {picked.map((e) => {
                        const cell = e.values[row.id] ?? {status: 'not-measured' as const, reason: 'no value in the data file'};
                        const isBest = best.has(e.id);
                        return (
                          <td key={e.id} className={isBest ? 'sw-cmp-best' : undefined}>
                            <Value row={row} cell={cell} />
                            {isBest ? <div className="sw-cmp-best-mark">best of the picked</div> : null}
                            <SourceLine cell={cell} />
                          </td>
                        );
                      })}
                    </tr>
                  );
                })}
              </tbody>
            ))}
          </table>
        </div>
        <p className="sw-cmp-legend">
          <span className="sw-cmp-loaded" title={`loaded: ${LOADED_MEANS}`}>
            loaded
          </span>{' '}
          beside a figure of our runs: {LOADED_MEANS}. A figure made of others, such as a geomean, is loaded when any of them was. The
          reason follows the mark: <code>neighbour</code>, another pool job ran on the node; <code>load</code>, the node’s load per CPU
          was over the bound; <code>unwatched</code>, the run has no record of the node. A loaded figure does not rank: when one of the
          figures compared in a row is loaded, none of them is marked “best of the picked”. See the{' '}
          <Link to="/docs/kpis">KPI page</Link>.
        </p>
        <p className="sw-cmp-foot">
          Published values:{' '}
          {published.map((p, i) => (
            <span key={p.file}>
              {i ? '; ' : ''}
              <a href={p.url}>{p.label}</a> ({p.license},{' '}
              <a href={`https://github.com/beyond10x/swell/blob/main/${p.license_file}`}>notice</a>)
            </span>
          ))}
          . swell’s test262 values come from its own runner at the pinned{' '}
          <a href={engines.swell_test262.source}>test262 commit</a>; the engine catalog is{' '}
          <a href="https://github.com/beyond10x/swell/blob/main/bench/engines.jsonl">bench/engines.jsonl</a>.
        </p>
      </main>
    </Layout>
  );
}
