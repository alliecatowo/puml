#!/usr/bin/env node
// Mirror docs/specs/*.md into site/content/developer/specs/<slug>.md with
// the frontmatter Zola expects. The first H1 in the source becomes the
// page title; the rest of the content is copied verbatim.

import { promises as fs } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, '..');
const srcDir = path.join(repoRoot, 'docs', 'specs');
const dstDir = path.join(repoRoot, 'site', 'content', 'developer', 'specs');

// Map source filename -> (slug, weight, description).
// Anything not in this map still gets mirrored under a slugified name.
const MAP = {
  'picouml-language.md':                            { slug: 'picouml-language',                weight: 10, desc: 'PicoUML language baseline.' },
  'diagram_families_architecture_spec.md':          { slug: 'diagram-families-architecture',   weight: 20, desc: 'Per-family compile contract and module layout.' },
  'puml_studio_spa_spec.md':                        { slug: 'studio-spa',                       weight: 30, desc: 'Local-first, WASM-first browser studio.' },
  'puml_syntax_highlighting_spec(1).md':            { slug: 'syntax-highlighting',              weight: 40, desc: 'Token taxonomy across grammars and LSP semantic tokens.' },
  'puml_lsp_spec.md':                               { slug: 'lsp',                              weight: 50, desc: 'LSP capabilities, messages, and semantic-token legend.' },
  'puml_vscode_extension_spec(1).md':               { slug: 'vscode-extension',                 weight: 60, desc: 'First-party VS Code extension contract.' },
  'puml_markdown_fence_renderer_spec(1).md':        { slug: 'markdown-fence-renderer',          weight: 70, desc: 'How `puml --from-markdown` extracts and renders fenced blocks.' },
  'puml_agent_plugin_mcp_spec.md':                  { slug: 'agent-plugin-mcp',                 weight: 80, desc: 'Agent / MCP plugin surface for Codex, Claude, and friends.' },
};

function slugify(s) {
  return s
    .replace(/\.md$/, '')
    .replace(/[()]/g, '')
    .replace(/[_\s]+/g, '-')
    .replace(/-+/g, '-')
    .toLowerCase();
}

function escapeToml(s) {
  return String(s)
    .replace(/\\/g, '\\\\')
    .replace(/"/g, '\\"')
    .replace(/\n/g, '\\n')
    .replace(/\r/g, '\\r')
    .replace(/\t/g, '\\t');
}

function extractTitle(md, fallback) {
  const m = md.match(/^#\s+(.+?)\s*$/m);
  return m ? m[1].replace(/`/g, '') : fallback;
}

function stripFirstH1(md) {
  return md.replace(/^#\s+.+?\n+/, '');
}

async function main() {
  const entries = (await fs.readdir(srcDir)).filter((f) => f.endsWith('.md'));
  await fs.mkdir(dstDir, { recursive: true });

  // index.md for the specs subsection.
  const indexFm =
    `+++\n` +
    `title = "Reference specs"\n` +
    `description = "Mirrored canonical specifications from docs/specs/."\n` +
    `sort_by = "weight"\n` +
    `template = "section.html"\n` +
    `page_template = "page.html"\n` +
    `+++\n\n` +
    `These pages mirror the source-of-truth specifications stored in [\`docs/specs/\`](https://github.com/alliecatowo/puml/tree/main/docs/specs) of the repository. They are the definitive contracts for the engine, the language server, the syntax highlighter, the studio SPA, the VS Code extension, the markdown fence renderer, and the agent / MCP plugin.\n\n` +
    `If you came here looking for a quick orientation, start with the [user guide](@/guide/_index.md) instead.\n`;
  await fs.writeFile(path.join(dstDir, '_index.md'), indexFm);

  for (const file of entries) {
    const conf = MAP[file] || { slug: slugify(file), weight: 100, desc: '' };
    const srcPath = path.join(srcDir, file);
    const md = await fs.readFile(srcPath, 'utf8');
    const title = extractTitle(md, conf.slug);
    const body = stripFirstH1(md);

    const fm = [
      '+++',
      `title = "${escapeToml(title)}"`,
      `description = "${escapeToml(conf.desc || title)}"`,
      `weight = ${conf.weight}`,
      '+++',
      '',
      `> Mirror of [\`docs/specs/${file}\`](https://github.com/alliecatowo/puml/blob/main/docs/specs/${file}) &mdash; the in-repo file is the source of truth.`,
      '',
      body,
    ].join('\n');

    const dstPath = path.join(dstDir, `${conf.slug}.md`);
    await fs.writeFile(dstPath, fm);
    console.log(`Mirrored ${file} -> ${conf.slug}.md (${body.length} chars)`);
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
