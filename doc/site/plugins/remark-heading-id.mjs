/**
 * Remark plugin that handles explicit heading IDs in the form `## My Heading {#custom-id}`.
 * Strips the token from visible text and sets the HTML anchor id.
 */
import { visit } from "unist-util-visit";

const HEADING_ID = /\s*\{[#$]([\w-]+)\}\s*$/;

export default function remarkHeadingId() {
  return (tree) => {
    visit(tree, "heading", (node) => {
      if (!node.children?.length) return;

      const last = node.children[node.children.length - 1];
      if (last.type !== "text") return;

      const match = last.value.match(HEADING_ID);
      if (!match) return;

      last.value = last.value.replace(HEADING_ID, "");
      if (last.value === "") node.children.pop();

      node.data = node.data ?? {};
      node.data.hProperties = node.data.hProperties ?? {};
      node.data.hProperties.id = match[1];
      node.data.id = match[1];
    });
  };
}
