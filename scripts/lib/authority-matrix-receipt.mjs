const positive = (value) => Number.isSafeInteger(value) && value > 0;
const digest = (value) => typeof value === "string" && /^[0-9a-f]{64}$/.test(value);

// Validates execution receipts, never authority artifacts or signatures.
export function validateMatrix(entries, profiles, operations, nodeVersion) {
  const required = new Set(profiles.flatMap((profile) => ["ordinary", "arbitrary_precision"].map((feature) => `${profile}/${feature}`)));
  if (!required.size || entries.length !== required.size) throw new Error("matrix cells are missing");
  for (const { profile, feature, general, keyring } of entries) {
    if (!required.delete(`${profile}/${feature}`)) throw new Error("duplicate or unexpected matrix cell");
    if (!general || !keyring || general.mismatched !== 0 || general.failOpen !== 0 || keyring.mismatches !== 0) throw new Error("matrix has failures or missing outcomes");
    if (!positive(general.cases) || general.vectors !== 132 || !positive(keyring.cases) || !positive(keyring.validControls)) throw new Error("matrix coverage is empty");
    if (!positive(general.multifaults?.pairs) || !positive(general.multifaults?.triples)) throw new Error("pair/triple samples are missing");
    if (!positive(keyring.byCode?.integrity_role_unknown) || !positive(keyring.byCode?.json_duplicate_key)) throw new Error("keyring role/duplicate coverage is missing");
    for (const operation of operations) if (!positive(general.byOperation?.[operation])) throw new Error(`operation ${operation} did not execute`);
    for (const operation of ["request_adoption", "decision_consumption", "lifecycle"]) {
      if (!positive(general.replayResults?.[operation]?.nonNullState)) throw new Error(`non-null ${operation} state was not compared`);
    }
    for (const receipt of [general, keyring]) {
      if (receipt.node !== nodeVersion || !digest(receipt.inputSha256) || !digest(receipt.binarySha256)) throw new Error("tool/input/artifact receipt is missing or differs");
      if (receipt.build?.serde_json_arbitrary_precision !== (feature === "arbitrary_precision") || receipt.build?.debug_assertions !== (profile === "debug")) throw new Error("receipt feature/profile differs");
    }
  }
  for (const kind of ["general", "keyring"]) {
    if (new Set(entries.map((entry) => entry[kind].inputSha256)).size !== 1) throw new Error(`${kind} inputs differ across matrix artifacts`);
  }
  return { generalInputSha256: entries[0].general.inputSha256, keyringInputSha256: entries[0].keyring.inputSha256 };
}
