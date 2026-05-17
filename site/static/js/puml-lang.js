// A CodeMirror 6 StreamLanguage for .puml source.
// The token taxonomy aligns with docs/specs/puml_syntax_highlighting_spec(1).md.
// Token return values use CodeMirror's standard style names so they can be
// styled via syntaxHighlighting(HighlightStyle) without custom registration.

import { StreamLanguage } from '@codemirror/language';

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

export const pumlStreamParser = {
  name: 'puml',
  startState() {
    return { inBlockComment: false };
  },
  token(stream, state) {
    if (state.inBlockComment) {
      while (!stream.eol()) {
        if (stream.match("'/")) {
          state.inBlockComment = false;
          return 'comment';
        }
        stream.next();
      }
      return 'comment';
    }

    if (stream.eatSpace()) return null;

    if (stream.match(/^'.*/)) return 'comment';
    if (stream.match("/'")) {
      state.inBlockComment = true;
      while (!stream.eol()) {
        if (stream.match("'/")) {
          state.inBlockComment = false;
          return 'comment';
        }
        stream.next();
      }
      return 'comment';
    }

    if (stream.match(/^@(start|end)[a-zA-Z]*/)) return 'meta';
    if (stream.match(/^![A-Za-z_$][A-Za-z0-9_]*/)) return 'meta';
    if (stream.match(/^"(?:\\.|[^"\\])*"/)) return 'string';
    if (stream.match(/^<<[^>]*>>/)) return 'typeName';
    if (stream.match(/^#[0-9a-fA-F]{3,8}\b/)) return 'literal';
    if (stream.match(/^-?\d+(?:\.\d+)?/)) return 'number';
    if (stream.match(ARROW_RE)) return 'operator';
    if (stream.match(/^[(){}\[\]]/)) return 'bracket';

    const id = stream.match(/^[A-Za-z_][A-Za-z0-9_\-.]*/);
    if (id) {
      const word = id[0].toLowerCase();
      if (SKINPARAM_KEYWORDS.has(word)) return 'typeName';
      if (INCLUDE_KEYWORDS.has(word)) return 'meta';
      if (LIFECYCLE_KEYWORDS.has(word)) return 'atom';
      if (FLOW_KEYWORDS.has(word)) return 'atom';
      if (GROUP_KEYWORDS.has(word)) return 'keyword';
      if (PARTICIPANT_KEYWORDS.has(word)) return 'keyword';
      if (NOTE_KEYWORDS.has(word)) return 'keyword';
      return 'variableName';
    }

    stream.next();
    return null;
  },
};

export const pumlLanguage = StreamLanguage.define(pumlStreamParser);
