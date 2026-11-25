import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {withProductSite} from '@beyond10x/docs-system/product-site';
import {PRISM_ADDITIONAL_LANGUAGES} from '@beyond10x/docs-system/code';

const siteDir = path.dirname(fileURLToPath(import.meta.url));

// The landing page is src/pages/index.tsx on the shared docs-system product components; its hero
// figure is src/components/EngineFigure.tsx. The pages in docs/ are written by hand from the
// repository's README.md, CHANGELOG.md (section 0.2.0), docs/design/engine.md,
// docs/design/host-interface.md, docs/design/quality-and-comparison.md, docs/plan/waves.md,
// AGENTS.md and the committed files of the gate under conformance/. A page states only what
// those sources state. Every number the site shows is read from data/status.json, which
// `task site-data` writes from the gate's files and `task site-data-check` holds; the compare
// page (src/pages/compare.tsx) reads data/engines.json, which the same tasks write and hold.

const config: Config = {
  title: 'swell',
  tagline: 'An ECMAScript (JavaScript) engine written from scratch in Rust.',
  favicon: 'img/logo.svg',

  future: {v4: true},
  url: 'https://beyond10x.github.io',
  baseUrl: '/swell/',
  organizationName: 'beyond10x',
  projectName: 'swell',
  deploymentBranch: 'gh-pages',
  trailingSlash: false,
  onBrokenLinks: 'throw',
  onBrokenAnchors: 'throw',

  markdown: {
    hooks: {onBrokenMarkdownLinks: 'throw'},
  },
  i18n: {defaultLocale: 'en', locales: ['en']},

  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
          routeBasePath: 'docs',
          editUrl: 'https://github.com/beyond10x/swell/tree/main/website/',
        },
        blog: false,
        theme: {customCss: './src/css/custom.css'},
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    prism: {additionalLanguages: [...PRISM_ADDITIONAL_LANGUAGES]},
    navbar: {
      title: 'swell',
      logo: {alt: 'swell', src: 'img/logo.svg', srcDark: 'img/logo-dark.svg', width: 32, height: 32},
      items: [
        {to: '/docs/', label: 'Docs', position: 'left', activeBaseRegex: '^/swell/docs/(?!status|architecture|embedding|roadmap)'},
        {to: '/docs/status', label: 'Status', position: 'left'},
        {to: '/docs/architecture', label: 'Architecture', position: 'left'},
        {to: '/docs/embedding', label: 'Embedding', position: 'left'},
        {to: '/docs/roadmap', label: 'Roadmap', position: 'left'},
        {to: '/compare', label: 'Compare', position: 'left'},
        {href: 'https://github.com/beyond10x/swell', label: 'GitHub', position: 'right'},
      ],
    },
    footer: {
      links: [
        {
          title: 'Documentation',
          items: [
            {label: 'What swell is', to: '/docs/'},
            {label: 'Status', to: '/docs/status'},
            {label: 'Architecture', to: '/docs/architecture'},
            {label: 'Embedding', to: '/docs/embedding'},
            {label: 'Principles', to: '/docs/principles'},
            {label: 'Limits', to: '/docs/limits'},
            {label: 'KPIs', to: '/docs/kpis'},
            {label: 'Compare engines', to: '/compare'},
          ],
        },
        {
          title: 'Project',
          items: [
            {label: 'Roadmap', to: '/docs/roadmap'},
            {label: 'Changelog', href: 'https://github.com/beyond10x/swell/blob/main/CHANGELOG.md'},
            {label: 'Source', href: 'https://github.com/beyond10x/swell'},
          ],
        },
      ],
      copyright: 'A beyond10x project · Apache-2.0 · built with Docusaurus and the docs-system product template.',
    },
  } satisfies Preset.ThemeConfig,
};

// The navbar shows swell's own mark (static/img/logo.svg, logo-dark.svg), so the shared letter
// tile is turned off with an empty mark and custom.css shows the theme's logo again.
const site = withProductSite(config, {mark: ''});

// withProductSite adds its static directory as an absolute path, which the build would ship in its
// bundle; it is made relative to the site so no machine path reaches the built site.
export default {
  ...site,
  staticDirectories: site.staticDirectories?.map((directory) =>
    directory && path.isAbsolute(directory) ? path.relative(siteDir, directory) : directory,
  ),
} satisfies Config;
