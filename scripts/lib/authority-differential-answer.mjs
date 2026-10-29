// Outcome comparison, shared with focused regressions so exception fallback
// cannot hide the text corpus's explicit normative expectations.
const same = (left, right) => left.code === right.code && left.result === right.result;

export function compareAnswer(want, got, rewrite, anchorExpected, anchorActual, referenceCodes) {
  const failures = [];
  if (rewrite?.equals !== null && rewrite?.equals !== undefined) {
    if (!same(anchorExpected, want)) failures.push(`reference answer differs from case ${rewrite.equals}`);
    if (!same(anchorActual, got)) failures.push(`port answer differs from case ${rewrite.equals}`);
  }
  if (rewrite?.code) {
    if (want.code !== rewrite.code) failures.push(`reference did not refuse with ${rewrite.code}`);
    if (got.code !== rewrite.code || got.result !== null) failures.push(`port did not refuse with ${rewrite.code}`);
  }
  if (want.code?.startsWith("THROW:")) {
    return { failures, failOpen: !referenceCodes.has(got.code) || got.result !== null };
  }
  if (!same(want, got)) failures.push("answers differ");
  return { failures, failOpen: false };
}
