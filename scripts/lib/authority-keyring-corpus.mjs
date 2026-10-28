// Synthetic public-key mutations for #93. No private key or credential input.
export function* keyringCases(keys) {
  yield { label: "valid-control", keys: structuredClone(keys) };
  const roles = ["threads_authority", "principal", "protected_owner", "auditor", "unknown"];
  for (const [id, record] of Object.entries(keys)) {
    const emit = (label, replacement) => ({
      label: `${id}/${label}`,
      keys: { ...structuredClone(keys), [id]: replacement },
    });
    for (const value of [null, false, [], "record"]) yield emit(`record:${JSON.stringify(value)}`, value);
    for (const member of ["role", "principal_id", "public_key_pem"]) {
      const missing = structuredClone(record);
      delete missing[member];
      yield emit(`missing:${member}`, missing);
      for (const value of [null, false, 0, "", [], {}]) {
        yield emit(`${member}:${JSON.stringify(value)}`, { ...record, [member]: value });
      }
    }
    for (const role of roles) {
      const replacement = { ...record, role };
      if (role === "threads_authority") delete replacement.principal_id;
      else replacement.principal_id = record.principal_id ?? "principal:synthetic";
      yield emit(`role:${role}`, replacement);
    }
    for (const principal of ["principal:substitution", "principal:alice", "principal:bob"]) {
      yield emit(`principal:${principal}`, { ...record, principal_id: principal });
    }
    const pem = record.public_key_pem;
    const base64 = pem.replace(/-----[^\n]+-----/g, "").replace(/\s/g, "");
    const der = Buffer.from(base64, "base64");
    const wrap = (bytes) => `-----BEGIN PUBLIC KEY-----\n${bytes.toString("base64")}\n-----END PUBLIC KEY-----\n`;
    const oid = Buffer.from(der);
    oid[8] ^= 1; // Ed25519 -> another algorithm OID, same key bytes.
    const variants = {
      "crlf": pem.replaceAll("\n", "\r\n"),
      "outer-whitespace": ` \n${pem}\t`,
      "base64-short-lines": `-----BEGIN PUBLIC KEY-----\n${base64.match(/.{1,16}/g).join("\n")}\n-----END PUBLIC KEY-----\n`,
      "wrong-algorithm": wrap(oid),
      "truncated-der": wrap(der.subarray(0, -1)),
      "trailing-der": wrap(Buffer.concat([der, Buffer.from([0])])),
      "wrong-label": pem.replaceAll("PUBLIC KEY", "CERTIFICATE"),
      "missing-footer": pem.split("-----END")[0],
      "garbage": "not a PEM public key",
      "other-valid-key": Object.values(keys).find((other) => other.public_key_pem !== pem).public_key_pem,
    };
    for (const [label, value] of Object.entries(variants)) {
      yield emit(`pem:${label}`, { ...record, public_key_pem: value });
    }
    for (const value of [null, false, 0, "", true, 1, {}, [], pem, variants["other-valid-key"]]) {
      yield emit(`public_key:${JSON.stringify(value)}`, { ...record, public_key: value });
    }
    const removed = structuredClone(keys);
    delete removed[id];
    yield { label: `${id}/removed-key`, keys: removed };
  }
  // Multiple faults: JSON integer-like enumeration, reversed insertion, and
  // UTF-16 vs Unicode-scalar order. Preserve the actual order, never sort it
  // in this corpus to hide an implementation's first-error divergence.
  for (const ids of [["z", "a"], ["a", "z"], ["10", "2"], ["2", "10"], ["\ue000", "\u{10000}"], ["\u{10000}", "\ue000"]]) {
    yield {
      label: `error-order:${JSON.stringify(ids)}`,
      keys: { ...keys, [ids[0]]: { role: "unknown" }, [ids[1]]: { role: "principal" } },
    };
  }
}
