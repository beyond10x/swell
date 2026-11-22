import type {ReactNode} from 'react';
import Layout from '@theme/Layout';
import Link from '@docusaurus/Link';
import {Feature, FeatureGrid, FlowStep, Flow, ProductHero, ProductSection, StatusStrip} from '@beyond10x/docs-system/product';
import type {ProductStatusItem} from '@beyond10x/docs-system/product-data';
import EngineFigure from '../components/EngineFigure';
import {count, deliverySlices, nextSlice, status} from '../data/status';

/**
 * The landing page: its own page so that the hero's aside is the engine figure and every number
 * is read from data/status.json; the sections below the hero are the shared docs-system
 * components (ProductHero, ProductSection, FeatureGrid, Flow, StatusStrip). Statements from
 * README.md, CHANGELOG.md [0.2.0] and docs/design/engine.md, as the pages in docs/ cite them.
 */
export default function Home(): ReactNode {
  const t = status.test262;
  const c = status.crates;
  const r = status.host_requirements;
  const delivered = status.delivered_count;
  const slices = status.delivery_slices;
  const items: ProductStatusItem[] = deliverySlices.map((s) => ({
    label: `${Number(s.slice)} · ${s.name}`,
    status: s.delivered ? 'shipped' : 'planned',
    detail: s.delivered ? `${count(s.cases)} cases, every one passes` : `${count(s.passed)} of ${count(s.cases)} cases pass`,
    href: '/docs/roadmap',
  }));
  return (
    <Layout
      title="swell: an ECMAScript engine written from scratch in Rust"
      description="An ECMAScript (JavaScript) engine written from scratch in Rust, with no unsafe code and no third-party crate, for programs that run JavaScript they do not trust. Early."
      wrapperClassName="b10x-product-page"
    >
      <main id="b10x-product-main">
        <div className="b10x-product">
          <ProductHero
            eyebrow="JavaScript engine · written from scratch in Rust · early"
            promise={['An ECMAScript engine,', 'written from scratch in Rust.']}
            lede={
              <>
                swell parses JavaScript, compiles it to bytecode and interprets it, for a program that runs scripts it
                does not trust. It is early: {count(delivered)} of {count(slices)} delivery slices are delivered, and{' '}
                {t.passed_percent} % of the test262 cases it runs pass. Nothing is marked complete yet.
              </>
            }
            actions={[
              {label: 'Where it stands', href: '/docs/status', kind: 'primary'},
              {label: 'Architecture', href: '/docs/architecture'},
              {label: 'Embedding', href: '/docs/embedding'},
              {label: 'Repository', href: 'https://github.com/beyond10x/swell'},
            ]}
            meta={<>Release {status.version} · a library, no binary yet · Rust · Apache-2.0</>}
            aside={<EngineFigure variant="hero" />}
          />

          <ProductSection id="apart" number={1} eyebrow="What sets it apart" title="Safe Rust, bounded runs, no ambient authority.">
            <FeatureGrid label="What sets swell apart">
              <Feature title="No unsafe code, no third-party crate, no machine code" href="/docs/principles">
                <p>
                  {count(c.roots_forbidding_unsafe)} of {count(c.roots)} crate roots carry <code>#![forbid(unsafe_code)]</code>.{' '}
                  <code>Cargo.lock</code> lists {count(c.lock_packages)} packages and {count(c.lock_packages_outside)} from outside the
                  repository. It has no JIT compiler. The price is speed: swell is not fast first.
                </p>
              </Feature>
              <Feature title="Built for input it does not trust" href="/docs/architecture#budgets">
                <p>
                  Every run has a step budget, a memory budget and depth limits, and each ends the run with an outcome the host
                  reads. A stale handle ends the run as an engine fault, not a crash. <code>Limits::min_stack_bytes()</code> tells
                  the host the thread stack an agent needs.
                </p>
              </Feature>
              <Feature title="No ambient authority" href="/docs/embedding#what-the-host-supplies">
                <p>
                  The host supplies the clock, the time zone, random numbers, module loading and jobs. swell has no other way to
                  reach files, the network, time or randomness, so a run repeats exactly when the host's answers do.
                </p>
              </Feature>
              <Feature title="It says where it is incomplete" href="/docs/status">
                <p>
                  test262 is pinned by commit and counted per delivery slice. A construct the parser does not have yet is refused
                  with a <code>SyntaxError</code> that gives the slice that brings it, and a refusal is not counted as a pass.
                </p>
              </Feature>
            </FeatureGrid>
          </ProductSection>

          <ProductSection id="run" number={2} eyebrow="How a script runs" title="From source text to an outcome the host reads.">
            <Flow label="How a script runs">
              <FlowStep label="Parse" title="swell-syntax">
                <p>The ES5 grammar in strict and non-strict mode, with the early errors of strict code. Nesting is bounded by <code>parser_nesting</code>.</p>
              </FlowStep>
              <FlowStep label="Compile" title="swell-compiler">
                <p>Bytecode for a stack machine. A name gets a slot at compile time, unless <code>with</code> or a direct <code>eval</code> can change what it means.</p>
              </FlowStep>
              <FlowStep label="Run under budgets" title="swell">
                <p>The run loop executes the bytecode over the heap. Each instruction spends a step, and each allocation is charged to the memory budget.</p>
              </FlowStep>
              <FlowStep label="Outcome" title="Outcome">
                <p><code>Normal</code>, <code>Throw</code> or <code>Terminated</code> with its reason. No <code>catch</code> or <code>finally</code> block of the script runs after a budget ends it.</p>
              </FlowStep>
            </Flow>
          </ProductSection>

          <ProductSection id="built" number={3} eyebrow="How it is built" title="The specification first, then the code, held by one gate.">
            <FeatureGrid label="How swell is built">
              <Feature title="ECMA-262 and test262, pinned" href="/docs/status">
                <p>The language is ECMA-262 at a pinned commit, held by test262 at a pinned commit. The gate counts the cases that exist, are excluded, run and pass, per directory and per delivery slice.</p>
              </Feature>
              <Feature title="The host interface, specified before its code" href="/docs/embedding">
                <p>An ESS model states the lifecycles of agents, realms, scripts, modules and jobs. The rest is {count(r.total)} numbered requirements, {count(r.test)} of them held by a test today.</p>
              </Feature>
              <Feature title="One gate, task check" href="/docs/principles">
                <p>It pins test262 and the Unicode data by checksum, checks both specifications, runs the engine's tests and test262, and refuses its planted defects.</p>
              </Feature>
            </FeatureGrid>
          </ProductSection>

          <ProductSection
            id="status"
            number={4}
            eyebrow="Where it stands"
            kind="status"
            title={<>{count(delivered)} of {count(slices)} delivery slices are delivered.</>}
            lede={
              <>
                A delivered slice passes every test262 case it holds. The rest are counted already: {count(t.passed)} of the{' '}
                {count(t.run)} cases that run pass today.{nextSlice ? <> Next is slice {Number(nextSlice.slice)}, <code>{nextSlice.name}</code>.</> : null}
              </>
            }
          >
            <StatusStrip
              items={items}
              label="Delivery slices"
              groupLabels={{shipped: 'Delivered · every case of the slice passes', planned: 'Not delivered yet · counted already'}}
              source="conformance/test262/expected.txt and conformance/slices-delivered.txt, through task site-data"
            />
          </ProductSection>

          <ProductSection
            id="choices"
            number={5}
            eyebrow="Compared to others"
            title="swell's choices, and what each one costs."
            lede={<>From the README's “What swell is not” and the design's rejected options. <Link to="/docs/principles">Principles</Link> has the rest.</>}
          >
            <FeatureGrid label="swell's choices and their price" columns={3}>
              <Feature title="No JIT compiler" href="/docs/principles#what-swell-is-not">
                <p>swell writes no machine code and has no executable memory. It is not fast first, and how much slower it is than an engine with unsafe code is not measured.</p>
              </Feature>
              <Feature title="Handles, not pointers" href="/docs/architecture#the-heap">
                <p>A value holds a typed handle: an index and a generation into an arena. Every access pays a bounds check and a generation compare, and a stale handle reaches no other object.</p>
              </Feature>
              <Feature title="An interpreter over bytecode" href="/docs/architecture#the-path-of-a-script">
                <p>Frames are data on the engine's own frame stack, so a generator can be suspended without unsafe stack switching. A tree walker is rejected for that reason, a register machine for now.</p>
              </Feature>
              <Feature title="Property lists, no shapes" href="/docs/limits">
                <p>Shapes are rejected for the first slices because their gain is not measured. A property is found by a scan of its object's list, and that scan is not charged as steps yet.</p>
              </Feature>
              <Feature title="Not a wrapper" href="/docs/principles#what-swell-is-not">
                <p>swell is not a wrapper around V8, SpiderMonkey, JavaScriptCore or any other engine. The parser, the compiler and the interpreter are written in this repository.</p>
              </Feature>
              <Feature title="Not a browser" href="/docs/principles#what-swell-is-not">
                <p>The DOM and everything else a web page sees beyond the language belong to the program that embeds swell, such as the browser surf.</p>
              </Feature>
            </FeatureGrid>
          </ProductSection>
        </div>
      </main>
    </Layout>
  );
}
