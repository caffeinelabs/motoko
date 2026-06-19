/**
 * Rehype plugin that adds target="_blank" and rel="noopener noreferrer"
 * to all external links (links starting with http:// or https://).
 */
import { visit } from "unist-util-visit";

export default function rehypeExternalLinks() {
  return (tree) => {
    visit(tree, "element", (node) => {
      if (node.tagName !== "a") return;

      const href = node.properties?.href;
      if (!href || typeof href !== "string") return;

      if (/^https?:\/\//.test(href)) {
        node.properties.target = "_blank";
        node.properties.rel = "noopener noreferrer";
      }
    });
  };
}
