// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import remarkIncludeFile from "./plugins/remark-include-file.mjs";
import remarkHeadingId from "./plugins/remark-heading-id.mjs";
import rehypeExternalLinks from "./plugins/rehype-external-links.mjs";
import rehypeRewriteLinks from "./plugins/rehype-rewrite-links.mjs";
import { sidebar } from "./sidebar.mjs";

export default defineConfig({
  markdown: {
    rehypePlugins: [rehypeExternalLinks, rehypeRewriteLinks],
    remarkPlugins: [remarkHeadingId, remarkIncludeFile],
  },
  integrations: [
    starlight({
      title: "Motoko",
      customCss: [
        "@fontsource/inter/400.css",
        "@fontsource/inter/500.css",
        "@fontsource/inter/600.css",
        "@fontsource/inter/700.css",
        "@fontsource/newsreader/400.css",
        "@fontsource/newsreader/400-italic.css",
        "@fontsource/newsreader/500.css",
        "@fontsource/newsreader/500-italic.css",
        "@fontsource/jetbrains-mono/400.css",
        "@fontsource/jetbrains-mono/500.css",
        "./src/styles/custom.css",
      ],
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/caffeinelabs/motoko",
        },
      ],
      sidebar,
    }),
  ],
});
