import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

/** One hand-written sidebar: what swell is and where it stands, how it works, the project. */
const sidebars: SidebarsConfig = {
  docsSidebar: [
    {
      type: 'category',
      label: 'Start here',
      collapsed: false,
      items: ['index', 'status', 'limits'],
    },
    {
      type: 'category',
      label: 'Concepts',
      collapsed: false,
      items: ['architecture', 'principles'],
    },
    {
      type: 'category',
      label: 'Reference',
      collapsed: false,
      items: ['embedding', 'kpis'],
    },
    {
      type: 'category',
      label: 'Project',
      collapsed: false,
      items: ['roadmap'],
    },
  ],
};

export default sidebars;
