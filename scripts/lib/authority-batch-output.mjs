import { strictParseJson } from "../../profiles/automation-authority/v1/validator.mjs";

export function parseBatchAnswers(text, expectedCount) {
  const fail = (message, index) => {
    const error = new Error(message);
    error.caseIndex = Math.max(0, Math.min(expectedCount - 1, index));
    throw error;
  };
  const lines = text.trimEnd() ? text.trimEnd().split("\n") : [];
  const answers = [];
  for (const [index, line] of lines.entries()) {
    let answer;
    try {
      strictParseJson(line);
      answer = JSON.parse(line); // Retain own __proto__ for the closed shape.
    } catch {
      fail("batch answer is not strict JSON", index);
    }
    if (!answer || Array.isArray(answer) || typeof answer !== "object" ||
        Object.keys(answer).sort().join(",") !== "code,result" ||
        !(answer.code === null || typeof answer.code === "string") ||
        !(answer.result === null || typeof answer.result === "string")) {
      fail("batch answer must contain only code/result strings or null", index);
    }
    answers.push(answer);
  }
  if (answers.length !== expectedCount) fail("batch answer count differs", answers.length);
  return answers;
}
