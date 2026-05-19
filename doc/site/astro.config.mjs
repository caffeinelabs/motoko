// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import remarkIncludeFile from "./plugins/remark-include-file.mjs";
import remarkHeadingId from "./plugins/remark-heading-id.mjs";
import rehypeExternalLinks from "./plugins/rehype-external-links.mjs";
import rehypeRewriteLinks from "./plugins/rehype-rewrite-links.mjs";

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
      sidebar: [
        { slug: "index", label: "Overview" },
        {
          label: "Fundamentals",
          collapsed: false,
          items: [
            { slug: "fundamentals/hello-world" },
            {
              label: "Basic syntax",
              collapsed: true,
              items: [
                { slug: "fundamentals/basic-syntax/defining-an-actor" },
                { slug: "fundamentals/basic-syntax/imports" },
                { slug: "fundamentals/basic-syntax/printing-values" },
                { slug: "fundamentals/basic-syntax/numbers" },
                { slug: "fundamentals/basic-syntax/characters-text" },
                { slug: "fundamentals/basic-syntax/literals" },
                { slug: "fundamentals/basic-syntax/identifiers" },
                { slug: "fundamentals/basic-syntax/functions" },
                { slug: "fundamentals/basic-syntax/operators" },
                { slug: "fundamentals/basic-syntax/comments" },
                { slug: "fundamentals/basic-syntax/whitespace" },
                { slug: "fundamentals/basic-syntax/traps" },
              ],
            },
            {
              label: "Actors",
              collapsed: true,
              items: [
                { slug: "fundamentals/actors/actors-async" },
                { slug: "fundamentals/actors/state" },
                { slug: "fundamentals/actors/data-persistence" },
                { slug: "fundamentals/actors/compatibility" },
                { slug: "fundamentals/actors/messaging" },
                {
                  label: "Orthogonal persistence",
                  collapsed: true,
                  items: [
                    { slug: "fundamentals/actors/orthogonal-persistence/overview" },
                    { slug: "fundamentals/actors/orthogonal-persistence/enhanced" },
                    { slug: "fundamentals/actors/orthogonal-persistence/classical" },
                  ],
                },
                { slug: "fundamentals/actors/mixins" },
                { slug: "fundamentals/actors/enhanced-multi-migration" },
              ],
            },
            {
              label: "Types",
              collapsed: true,
              items: [
                { slug: "fundamentals/types/primitive-types" },
                { slug: "fundamentals/types/shared-types" },
                { slug: "fundamentals/types/function-types" },
                { slug: "fundamentals/types/tuples" },
                { slug: "fundamentals/types/records" },
                { slug: "fundamentals/types/objects-classes" },
                { slug: "fundamentals/types/variants" },
                { slug: "fundamentals/types/immutable-arrays" },
                { slug: "fundamentals/types/mutable-arrays" },
                { slug: "fundamentals/types/options" },
                { slug: "fundamentals/types/results" },
                { slug: "fundamentals/types/advanced-types" },
                { slug: "fundamentals/types/stable-types" },
                { slug: "fundamentals/types/subtyping" },
                { slug: "fundamentals/types/type-conversions" },
              ],
            },
            {
              label: "Declarations",
              collapsed: true,
              items: [
                { slug: "fundamentals/declarations/variable-declarations" },
                { slug: "fundamentals/declarations/function-declarations" },
                { slug: "fundamentals/declarations/object-declaration" },
                { slug: "fundamentals/declarations/class-declarations" },
                { slug: "fundamentals/declarations/type-declarations" },
                { slug: "fundamentals/declarations/expression-declarations" },
                { slug: "fundamentals/declarations/module-declarations" },
              ],
            },
            {
              label: "Control flow",
              collapsed: true,
              items: [
                { slug: "fundamentals/control-flow/basic-control-flow" },
                { slug: "fundamentals/control-flow/loops" },
                { slug: "fundamentals/control-flow/conditionals" },
                { slug: "fundamentals/control-flow/blocks" },
                { slug: "fundamentals/control-flow/switch" },
              ],
            },
            { slug: "fundamentals/modules-imports" },
            { slug: "fundamentals/pattern-matching" },
            { slug: "fundamentals/error-handling" },
            { slug: "fundamentals/contextual-dot" },
            { slug: "fundamentals/implicit-parameters" },
          ],
        },
        {
          label: "ICP features",
          collapsed: true,
          autogenerate: { directory: "icp-features" },
        },
        {
          label: "Reference",
          collapsed: true,
          autogenerate: { directory: "reference" },
        },
      ],
    }),
  ],
});
