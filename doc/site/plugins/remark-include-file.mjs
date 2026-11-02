/**
 * Remark plugin that embeds file contents into code blocks using a file= attribute.
 *
 * Usage in markdown code fences:
 *
 *   ```motoko file=<motokoExamples>/counter.mo
 *   ```
 *
 *   ```motoko file=<motokoExamples>/todo-error.mo#L49-L58
 *   ```
 *
 *   ```md file=<motokoRoot>/Changelog.md
 *   ```
 *
 * Placeholders:
 *   <motokoExamples>  doc/md/examples/ (relative to the motoko repo)
 *   <motokoRoot>      root of the motoko repo (where Changelog.md lives)
 *
 * An optional #L<start>-L<end> suffix slices specific lines (1-based, inclusive).
 */
import { visit, SKIP } from "unist-util-visit";
import { readFileSync, existsSync } from "node:fs";
import { resolve, join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { fromMarkdown } from "mdast-util-from-markdown";

// plugins/ directory (where this file lives)
const PLUGIN_DIR = dirname(fileURLToPath(import.meta.url));
// doc/site/ directory
const SITE_DIR = join(PLUGIN_DIR, "..");
// doc/ directory
const DOC_DIR = join(SITE_DIR, "..");
// motoko repo root
const REPO_ROOT = join(DOC_DIR, "..");

const PLACEHOLDERS = {
  "<motokoExamples>": join(DOC_DIR, "md", "examples"),
  "<motokoRoot>": REPO_ROOT,
};

export default function remarkIncludeFile() {
  return function (tree, file) {
    visit(tree, "code", (node, index, parent) => {
      const fileMeta = (node.meta || "")
        .split(" ")
        .find((m) => m.startsWith("file="));
      if (!fileMeta) return;

      let rawPath = fileMeta.slice("file=".length);

      // Extract optional #L<start>-L<end> line range
      let lineStart = null;
      let lineEnd = null;
      const rangeMatch = rawPath.match(/#L(\d+)-L(\d+)$/);
      if (rangeMatch) {
        lineStart = parseInt(rangeMatch[1], 10);
        lineEnd = parseInt(rangeMatch[2], 10);
        rawPath = rawPath.slice(0, -rangeMatch[0].length);
      }

      // Expand placeholders
      for (const [placeholder, expansion] of Object.entries(PLACEHOLDERS)) {
        if (rawPath.startsWith(placeholder)) {
          rawPath = expansion + rawPath.slice(placeholder.length);
          break;
        }
      }

      const absPath = resolve(rawPath);

      if (!existsSync(absPath)) {
        throw new Error(
          `remark-include-file: file not found: ${absPath} (from file=${fileMeta})`,
        );
      }

      let content = readFileSync(absPath, "utf-8");

      if (lineStart !== null) {
        const lines = content.split("\n");
        if (lineStart < 1 || lineEnd > lines.length) {
          throw new Error(
            `remark-include-file: line range L${lineStart}-L${lineEnd} out of bounds ` +
            `(file has ${lines.length} lines): ${absPath}`,
          );
        }
        content = lines.slice(lineStart - 1, lineEnd).join("\n");
      }

      // Inline markdown inclusion: parse and splice AST children
      if (node.lang === "md") {
        const parsed = fromMarkdown(content.trimEnd());
        parent.children.splice(index, 1, ...parsed.children);
        return [SKIP, index + parsed.children.length];
      }

      node.value = content.trimEnd();
    });
  };
}
