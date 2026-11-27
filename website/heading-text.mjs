// Heading labels come from Markdown tokens, never from stripping rendered HTML.
// Decode text entities once; code spans retain their literal source spelling.
const entities = { '&amp;': '&', '&lt;': '<', '&gt;': '>', '&quot;': '"', '&#39;': "'", '&apos;': "'" };
export function headingText(tokens) {
  return tokens.map(token => {
    if (token.type === 'html') return '';
    if (token.tokens) return headingText(token.tokens);
    if (token.type === 'codespan' || token.type === 'escape') return token.text;
    if (token.type === 'br') return '';
    return (token.text ?? '').replace(/&(?:amp|lt|gt|quot|#39|apos);/g, entity => entities[entity]);
  }).join('');
}
