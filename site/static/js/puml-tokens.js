const GROUP_KEYWORDS = new Set([
  'alt', 'else', 'end', 'opt', 'loop', 'par', 'and', 'critical', 'option', 'break',
  'group', 'box', 'rect', 'ref', 'over',
  'partition', 'fork', 'again', 'endif', 'elseif', 'if', 'then',
  'repeat', 'while', 'endwhile', 'endpartition',
]);

const FLOW_KEYWORDS = new Set([
  'start', 'stop', 'end',
]);

const LIFECYCLE_KEYWORDS = new Set([
  'activate', 'deactivate', 'destroy', 'create', 'autonumber', 'newpage', 'return',
]);

const PARTICIPANT_KEYWORDS = new Set([
  'participant', 'actor', 'boundary', 'control', 'entity', 'database', 'collections',
  'queue', 'class', 'abstract', 'interface', 'enum', 'object', 'state', 'usecase',
  'component', 'node', 'cloud', 'frame', 'folder', 'package', 'rectangle', 'agent',
  'artifact', 'card', 'storage', 'stack', 'file',
]);

const NOTE_KEYWORDS = new Set([
  'note', 'hnote', 'rnote', 'legend', 'title', 'caption', 'header', 'footer',
  'left', 'right', 'top', 'bottom', 'of', 'across', 'on', 'link',
  'endnote', 'endlegend', 'endtitle', 'endheader', 'endfooter', 'as',
]);

const SKINPARAM_KEYWORDS = new Set(['skinparam', 'skinparams']);
const INCLUDE_KEYWORDS = new Set(['include', 'includesub', 'theme', 'startsub', 'endsub']);

const ARROW_RE = /^[<o]?-{1,2}(?:\[[^\]]*\])?-?-?>>?|^[<o]?-{1,2}(?:\[[^\]]*\])?[xX]|^[<o]-{1,2}/;

const TOKEN_CLASS_BY_NAME = {
  comment: 'tok-comment',
  meta: 'tok-directive',
  keyword: 'tok-keyword',
  atom: 'tok-lifecycle',
  typeName: 'tok-stereo',
  operator: 'tok-arrow',
  string: 'tok-string',
  number: 'tok-number',
  literal: 'tok-color',
  bracket: 'tok-bracket',
};

export const PUML_HIGHLIGHT_LANGS = new Set([
  'puml',
  'pumlx',
  'plantuml',
  'uml',
  'puml-sequence',
  'uml-sequence',
  'picouml',
]);

export function createPumlTokenizerState() {
  return { inBlockComment: false };
}

export function readPumlToken(line, index, state) {
  const rest = line.slice(index);

  if (!rest) return null;

  if (state.inBlockComment) {
    const endIndex = rest.indexOf("'/");
    if (endIndex === -1) return { text: rest, token: 'comment', nextIndex: line.length };
    state.inBlockComment = false;
    return { text: rest.slice(0, endIndex + 2), token: 'comment', nextIndex: index + endIndex + 2 };
  }

  const whitespace = rest.match(/^\s+/);
  if (whitespace) {
    return { text: whitespace[0], token: null, nextIndex: index + whitespace[0].length };
  }

  if (rest.startsWith("'")) {
    return { text: rest, token: 'comment', nextIndex: line.length };
  }

  if (rest.startsWith("/'")) {
    const endIndex = rest.indexOf("'/", 2);
    if (endIndex === -1) {
      state.inBlockComment = true;
      return { text: rest, token: 'comment', nextIndex: line.length };
    }
    return { text: rest.slice(0, endIndex + 2), token: 'comment', nextIndex: index + endIndex + 2 };
  }

  const directive = rest.match(/^@(start|end)[a-zA-Z]*/);
  if (directive) return { text: directive[0], token: 'meta', nextIndex: index + directive[0].length };

  const bangDirective = rest.match(/^![A-Za-z_$][A-Za-z0-9_]*/);
  if (bangDirective) return { text: bangDirective[0], token: 'meta', nextIndex: index + bangDirective[0].length };

  const stringToken = rest.match(/^"(?:\\.|[^"\\])*"/);
  if (stringToken) return { text: stringToken[0], token: 'string', nextIndex: index + stringToken[0].length };

  const stereotype = rest.match(/^<<[^>]*>>/);
  if (stereotype) return { text: stereotype[0], token: 'typeName', nextIndex: index + stereotype[0].length };

  const color = rest.match(/^#[0-9a-fA-F]{3,8}\b/);
  if (color) return { text: color[0], token: 'literal', nextIndex: index + color[0].length };

  const number = rest.match(/^-?\d+(?:\.\d+)?/);
  if (number) return { text: number[0], token: 'number', nextIndex: index + number[0].length };

  const arrow = rest.match(ARROW_RE);
  if (arrow) return { text: arrow[0], token: 'operator', nextIndex: index + arrow[0].length };

  const bracket = rest.match(/^[(){}\[\]]/);
  if (bracket) return { text: bracket[0], token: 'bracket', nextIndex: index + 1 };

  const identifier = rest.match(/^[A-Za-z_][A-Za-z0-9_\-.]*/);
  if (identifier) {
    const text = identifier[0];
    const word = text.toLowerCase();
    let token = null;
    if (SKINPARAM_KEYWORDS.has(word)) token = 'typeName';
    else if (INCLUDE_KEYWORDS.has(word)) token = 'meta';
    else if (LIFECYCLE_KEYWORDS.has(word) || FLOW_KEYWORDS.has(word)) token = 'atom';
    else if (GROUP_KEYWORDS.has(word) || PARTICIPANT_KEYWORDS.has(word) || NOTE_KEYWORDS.has(word)) token = 'keyword';
    return { text, token, nextIndex: index + text.length };
  }

  return { text: rest[0], token: null, nextIndex: index + 1 };
}

export function highlightPumlToHtml(source) {
  const state = createPumlTokenizerState();
  return source
    .split('\n')
    .map((line) => highlightPumlLine(line, state))
    .join('\n');
}

function highlightPumlLine(line, state) {
  let html = '';
  let index = 0;

  while (index < line.length) {
    const segment = readPumlToken(line, index, state);
    if (!segment) break;
    const escaped = escapeHtml(segment.text);
    const className = segment.token ? TOKEN_CLASS_BY_NAME[segment.token] : '';
    html += className ? `<span class="${className}">${escaped}</span>` : escaped;
    index = segment.nextIndex;
  }

  return html;
}

function escapeHtml(value) {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;');
}
