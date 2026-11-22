import type {ReactNode} from 'react';
import Link from '@docusaurus/Link';
import {count, deliverySlices, status} from '../data/status';

/**
 * swell as the program that embeds it sees it: the host boundary and what crosses it, the path of
 * a script through the crates, the budgets of a run and the outcome each one ends in, the three
 * absences, and how much of the standard is delivered. HTML and CSS on the shared tokens; every
 * number from data/status.json. `hero` is the landing page's aside, `full` the architecture
 * page's figure with the names of the hooks, the limits and the outcomes.
 *
 * Sources: README.md ("What swell is", "What swell is not"); docs/design/engine.md sections 2,
 * 3, 4 and 13; docs/design/host-interface.md decisions 2, 5 and 16;
 * crates/swell/src/host/{outcome,limits,hooks}.rs for the names.
 */

type Variant = 'hero' | 'full';

function Glyph({kind}: {kind: 'absent'}): ReactNode {
  if (kind === 'absent') {
    return (
      <svg className="sw-fig__glyph" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false">
        <circle cx="8" cy="8" r="6.25" fill="none" stroke="currentColor" strokeWidth="1.5" />
        <path d="M3.6 12.4 L12.4 3.6" fill="none" stroke="currentColor" strokeWidth="1.5" />
      </svg>
    );
  }
  return null;
}

function Port({dir, title, children, spine}: {dir: 'in' | 'out'; title: string; children: ReactNode; spine?: boolean}): ReactNode {
  return (
    <li className={['sw-port', `sw-port--${dir}`, spine ? 'sw-port--spine' : ''].filter(Boolean).join(' ')}>
      <p className="sw-port__title">
        <span className="sw-port__dir"><span aria-hidden="true">↓ </span>{dir}</span>
        <span className="sw-port__name">{title}</span>
      </p>
      {children}
    </li>
  );
}

function Meter({name, ends, field}: {name: string; ends: ReactNode; field?: ReactNode}): ReactNode {
  return (
    <li className="sw-meter">
      <span className="sw-meter__name">{name}{field ? <span className="sw-meter__field">{field}</span> : null}</span>
      <span className="sw-meter__track" aria-hidden="true">
        <span className="sw-meter__fill" />
        <span className="sw-meter__stop" />
      </span>
      <span className="sw-meter__ends"><span className="sw-meter__arrow" aria-hidden="true">→ </span><span className="sw-sr">ends in </span>{ends}</span>
    </li>
  );
}

export default function EngineFigure({variant = 'hero'}: {variant?: Variant}): ReactNode {
  const full = variant === 'full';
  const t = status.test262;
  const c = status.crates;
  const delivered = status.delivered_count;
  const total = status.delivery_slices;
  const id = `sw-fig-${variant}`;
  return (
    <figure className={`sw-fig sw-fig--${variant}`} aria-labelledby={`${id}-title`} aria-describedby={`${id}-alt`}>
      <p className="sw-fig__kicker" id={`${id}-title`}>swell, as the program that embeds it sees it</p>

      <div className="sw-host">
        <p className="sw-host__label">Your program · the host</p>

        <ul className="sw-ports sw-ports--in" aria-label="What enters swell from the host">
          <Port dir="in" title="source text" spine>
            <p className="sw-port__body"><code>parse_<wbr />script</code></p>
            {full ? <p className="sw-port__note">a script, as UTF-8 or UTF-16</p> : null}
          </Port>
          <Port dir="in" title="supplies">
            {full ? (
              <ul className="sw-port__list">
                <li>clock <span className="sw-port__hook"><code>host_system_utc_epoch_nanoseconds</code></span></li>
                <li>time zone <span className="sw-port__hook"><code>system_time_zone_identifier</code></span></li>
                <li>random numbers <span className="sw-port__hook"><code>Seeds</code></span></li>
                <li>module loading <span className="sw-port__hook"><code>host_load_imported_module</code></span></li>
                <li>jobs <span className="sw-port__hook"><code>host_enqueue_*_job</code></span></li>
              </ul>
            ) : (
              <ul className="sw-port__list">
                <li>clock</li>
                <li>time zone</li>
                <li>random numbers</li>
                <li>module loading</li>
                <li>jobs</li>
              </ul>
            )}
          </Port>
          <Port dir="in" title="sets">
            {full ? (
              <ul className="sw-port__list">
                <li>step budget <span className="sw-port__hook"><code>steps_per_entry</code></span></li>
                <li>memory budget <span className="sw-port__hook"><code>memory_bytes</code></span></li>
                <li>depth limits <span className="sw-port__hook"><code>frame_depth</code>, <code>reentry_depth</code>, <code>parser_nesting</code></span></li>
                <li>interrupt, from any thread <span className="sw-port__hook"><code>InterruptHandle</code></span></li>
              </ul>
            ) : (
              <ul className="sw-port__list">
                <li>step budget</li>
                <li>memory budget</li>
                <li>depth limits</li>
                <li>interrupt</li>
              </ul>
            )}
          </Port>
        </ul>

        <div className="sw-agent">
          <p className="sw-agent__label"><code>swell</code> · one <code>Agent</code></p>
          <p className="sw-agent__note">No other way to reach files, the network, time or randomness.</p>
          <ol className="sw-path" aria-label="The path of a script through the crates">
            <li className="sw-stage">
              <p className="sw-stage__head"><code className="sw-stage__crate">swell-syntax</code><span className="sw-stage__role">parse, early errors, strict mode</span></p>
              {full ? <p className="sw-stage__note">A construct of a later slice is refused with a <code>SyntaxError</code> that gives the slice. Nesting past <code>parser_nesting</code> is a <code>SyntaxError</code> too.</p> : null}
            </li>
            <li className="sw-stage">
              <p className="sw-stage__head"><code className="sw-stage__crate">swell-compiler</code><span className="sw-stage__role">bytecode</span></p>
              {full ? <p className="sw-stage__note">A name gets a slot at compile time, unless <code>with</code> or a direct <code>eval</code> can change what it means.</p> : null}
            </li>
            <li className="sw-stage sw-stage--run">
              <p className="sw-stage__head"><code className="sw-stage__crate">swell</code><span className="sw-stage__role">runs it over the heap</span></p>
              <ul className="sw-parts" aria-label="Inside the swell crate">
                <li><strong>run loop</strong><span>{full ? 'a stack machine; every frame on its own frame stack' : 'frames on its own stack'}</span></li>
                <li><strong>heap</strong><span>{full ? 'arenas; a handle is an index and a generation' : 'arenas and handles'}</span></li>
                <li><strong>builtins</strong><span>{full ? 'one module per clause of ECMA-262' : 'by clause of ECMA-262'}</span></li>
              </ul>
              <ul className="sw-meters" aria-label="The budgets of a run, and what each ends in">
                <Meter name="steps" field={full ? <code>steps_per_entry</code> : undefined} ends={<code>Terminated(StepBudget)</code>} />
                <Meter name="memory" field={full ? <code>memory_bytes</code> : undefined} ends={<code>Terminated(MemoryBudget)</code>} />
                {full ? (
                  <>
                    <Meter name="frames" field={<code>frame_depth</code>} ends={<><code>RangeError</code> in the script</>} />
                    <Meter name="re-entry" field={<code>reentry_depth</code>} ends={<><code>RangeError</code> in the script</>} />
                    <Meter name="nesting" field={<code>parser_nesting</code>} ends={<><code>SyntaxError</code> naming the limit</>} />
                  </>
                ) : (
                  <Meter name="depth" ends={<code>RangeError</code>} />
                )}
              </ul>
              <p className="sw-stage__note sw-stage__note--end">Each limit ends the run in an outcome, not in a panic or an abort.{full ? <> A stale handle inside the engine ends it as <code>Terminated(EngineFault)</code>.</> : null}</p>
            </li>
          </ol>
        </div>

        <ul className="sw-ports sw-ports--out" aria-label="What leaves swell for the host">
          <Port dir="out" title="reads" spine>
            <p className="sw-port__body sw-outcome">
              <code>Outcome</code>
              <span className="sw-outcome__arms"><code>Normal</code> · <code>Throw</code> · <code>Terminated</code></span>
            </p>
            <p className="sw-port__note">
              {full ? 'Terminated by ' : null}
              <code>StepBudget</code> · <code>MemoryBudget</code> · <code>Interrupted</code> · <code>EngineFault</code>
            </p>
          </Port>
        </ul>
      </div>

      <ul className="sw-absences" aria-label="What swell does without">
        <li>
          <Glyph kind="absent" />
          <p className="sw-absences__title">No <code>unsafe</code></p>
          <p className="sw-absences__proof"><code>#![forbid(unsafe_code)]</code> in {count(c.roots_forbidding_unsafe)} of {count(c.roots)} crate roots</p>
          <p className="sw-absences__source">gate: <code>forbid-unsafe-check</code></p>
        </li>
        <li>
          <Glyph kind="absent" />
          <p className="sw-absences__title">No third-party crate</p>
          <p className="sw-absences__proof"><code>Cargo.lock</code>: {count(c.lock_packages)} packages, {count(c.lock_packages_outside)} from outside</p>
          <p className="sw-absences__source">gate: <code>lock-check</code></p>
        </li>
        <li>
          <Glyph kind="absent" />
          <p className="sw-absences__title">No machine code</p>
          <p className="sw-absences__proof">no just-in-time compiler, no executable memory</p>
          <p className="sw-absences__source">README · design § 13</p>
        </li>
      </ul>

      <div className="sw-measure">
        <div className="sw-measure__row">
          <p className="sw-measure__label">Delivery slices</p>
          <ol className="sw-slices" aria-label={`${delivered} of ${total} delivery slices delivered`}>
            {deliverySlices.map((s) => (
              <li key={s.slice} className={s.delivered ? 'is-delivered' : undefined} title={`slice ${Number(s.slice)}, ${s.name}`}>
                <span className="sw-sr">slice {Number(s.slice)}, {s.name}: {s.delivered ? 'delivered' : 'not delivered'}</span>
              </li>
            ))}
          </ol>
          <p className="sw-measure__value"><strong>{count(delivered)} of {count(total)}</strong> delivered</p>
        </div>
        <div className="sw-measure__row">
          <p className="sw-measure__label">test262</p>
          <div className="sw-pass" aria-hidden="true">
            <span className="sw-pass__passed" style={{flexGrow: t.passed}} />
            <span className="sw-pass__failed" style={{flexGrow: t.failed}} />
          </div>
          <p className="sw-measure__value"><strong>{t.passed_percent} %</strong> pass</p>
          <p className="sw-measure__legend">
            <span><span className="sw-swatch sw-swatch--passed" aria-hidden="true" />{count(t.passed)} pass</span>
            <span><span className="sw-swatch sw-swatch--failed" aria-hidden="true" />{count(t.failed)} fail</span>
            <span>of the {count(t.run)} cases that run{full ? <>, and {count(t.excluded)} more excluded by recorded decision</> : null}</span>
          </p>
        </div>
      </div>

      <figcaption className="sw-fig__caption">
        {full ? (
          <>What crosses between a host and swell, the crates a script passes through, the budgets of a run, and the gate's measure. The numbers come from <Link to="/docs/status">the status page's</Link> data.</>
        ) : (
          <>The host gives swell everything from outside and reads one outcome back. <Link to="/docs/architecture">The full figure</Link> names every hook, limit and outcome.</>
        )}
      </figcaption>
      <p className="sw-sr" id={`${id}-alt`}>
        Your program, the host, surrounds swell. Three things enter swell from it: the source text of a
        script, passed to parse_script; what the host supplies, the clock, the time zone, random
        numbers, module loading and jobs; and what it sets, a step budget, a memory budget, depth
        limits and an interrupt. Inside, a script goes through swell-syntax, which parses it and checks
        early errors, then swell-compiler, which compiles it to bytecode, then the swell crate, whose
        run loop runs it over the heap with the builtins, one module per clause of ECMA-262. The step
        budget ends a run as Terminated(StepBudget), the memory budget as Terminated(MemoryBudget),
        and a depth limit with a RangeError. One thing leaves swell: the Outcome the host reads,
        Normal, Throw or Terminated. swell has no unsafe code ({count(c.roots_forbidding_unsafe)} of{' '}
        {count(c.roots)} crate roots forbid it), no third-party crate (Cargo.lock lists{' '}
        {count(c.lock_packages)} packages, {count(c.lock_packages_outside)} from outside the
        repository) and writes no machine code. {count(delivered)} of {count(total)} delivery slices are
        delivered, and {count(t.passed)} of the {count(t.run)} test262 cases that run pass,{' '}
        {t.passed_percent} %; {count(t.failed)} fail.
      </p>
    </figure>
  );
}
