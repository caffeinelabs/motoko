// @ts-check
// Note: type annotations allow type checking and IDEs autocompletion

const {themes} = require('prism-react-renderer');
const lightCodeTheme = themes.github;
const darkCodeTheme = themes.dracula;

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'Motoko Doc Preview',
  tagline: '(a work in progress)',
  url: 'https://your-docusaurus-test-site.com',
  baseUrl: '/',
  onBrokenLinks: 'warn',
  onBrokenMarkdownLinks: 'warn',
  favicon: 'img/motoko.png',

  // GitHub pages deployment config.
  // If you aren't using GitHub pages, you don't need these.
  organizationName: 'dfinity', // Usually your GitHub org/user name.
  projectName: 'motoko', // Usually your repo name.

  plugins: [
    'docusaurus-plugin-sass'
  ],

  markdown: {
    format: 'detect',
  },

  // Even if you don't use internalization, you can use this field to set useful
  // metadata like html lang. For example, if your site is Chinese, you may want
  // to replace "en" with "zh-Hans".
  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
          docs: {
          path: "../md",
          sidebarPath: require.resolve('./sidebars.js'),
          remarkPlugins: [require("remark-code-import")],
          // Please change this to your repo.
          // Remove this to remove the "edit this page" links.
          editUrl:
            'https://github.com/dfinity/motoko/tree/master/doc/md/',
        },
        blog: false,
        theme: {
          customCss: require.resolve('./src/css/custom.scss'),
        },
      }),
    ],
  ],

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      navbar: {
        title: 'Motoko',
        logo: {
          alt: 'My Site Logo',
          src: 'img/motoko.png',
        },
        items: [
          {
            type: 'doc',
            docId: 'base/index',
            position: 'left',
            label: 'Base',
          },
          {
            type: 'doc',
            docId: 'core/index',
            position: 'left',
            label: 'Core',
          },
          {
            href: 'https://github.com/dfinity/motoko',
            label: 'GitHub',
            position: 'right',
          },
        ],
      },
      footer: {
        style: 'dark',
        links: [
          {
            title: 'Docs',
            items: [
              {
                label: 'Motoko',
                to: '/docs/attic/motoko',
              },
              {
                label: 'Base',
                to: '/docs/base/index',
              },
            ],
          }
        ],
        copyright: `Copyright © ${new Date().getFullYear()} My Project, Inc. Built with Docusaurus.`,
      },
      prism: {
        theme: lightCodeTheme,
        darkTheme: darkCodeTheme,
      },
    }),
  clientModules: [require.resolve("./static/load_moc.ts")],
};

module.exports = config;
