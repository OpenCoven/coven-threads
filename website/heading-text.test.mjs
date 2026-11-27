import test from 'node:test';
import assert from 'node:assert/strict';
import { Lexer } from 'marked';
import { headingText } from './heading-text.mjs';

function text(markdown) {
  return headingText(Lexer.lexInline(markdown));
}

test('heading text decodes entities exactly once and preserves literal code', () => {
  assert.equal(text('A &amp;lt;script&amp;gt; B'), 'A &lt;script&gt; B');
  assert.equal(text('Use `<script>` & `&amp;`'), 'Use <script> & &amp;');
});

test('heading text keeps nested label text while omitting raw HTML tags', () => {
  assert.equal(text('**Bound _writer_** and [evidence](https://example.com) <em>here</em>'), 'Bound writer and evidence here');
  assert.equal(text('A &amp; B &quot;C&quot;'), 'A & B "C"');
});
