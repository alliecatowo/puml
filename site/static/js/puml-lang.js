// A CodeMirror 6 StreamLanguage for .puml source.
// The token taxonomy aligns with docs/specs/puml_syntax_highlighting_spec(1).md.
// Token return values use CodeMirror's standard style names so they can be
// styled via syntaxHighlighting(HighlightStyle) without custom registration.

import { StreamLanguage } from '@codemirror/language';
import { createPumlTokenizerState, readPumlToken } from './puml-tokens.js';

export const pumlStreamParser = {
  name: 'puml',
  startState() {
    return createPumlTokenizerState();
  },
  token(stream, state) {
    const line = stream.string;
    const segment = readPumlToken(line, stream.pos, state);
    if (!segment) return null;
    stream.pos = segment.nextIndex;
    return segment.token === null && /^[A-Za-z_][A-Za-z0-9_\-.]*$/.test(segment.text) ? 'variableName' : segment.token;
  },
};

export const pumlLanguage = StreamLanguage.define(pumlStreamParser);
