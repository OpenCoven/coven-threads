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
      "long-form-outer-length-two": wrap(Buffer.concat([Buffer.from([0x30, 0x82, 0, der[1]]), der.subarray(2)])),
      "long-form-algorithm-length": wrap(Buffer.concat([Buffer.from([0x30, 43, 0x30, 0x81, 5]), der.subarray(4)])),
      "long-form-oid-length": wrap(Buffer.concat([Buffer.from([0x30, 43, 0x30, 6, 6, 0x81, 3]), der.subarray(6)])),
      "long-form-bitstring-length": wrap(Buffer.concat([Buffer.from([0x30, 43]), der.subarray(2, 10), Buffer.from([0x81, 33]), der.subarray(11)])),
      "indefinite-outer": wrap(Buffer.concat([Buffer.from([0x30, 0x80]), der.subarray(2), Buffer.from([0, 0])])),
      "indefinite-algorithm": wrap(Buffer.concat([Buffer.from([0x30, 44, 0x30, 0x80]), der.subarray(4, 9), Buffer.from([0, 0]), der.subarray(9)])),
      "constructed-bitstring": wrap(Buffer.concat([Buffer.from([0x30, 44]), der.subarray(2, 9), Buffer.from([0x23, 35]), der.subarray(9)])),
      "indefinite-bitstring": wrap(Buffer.concat([Buffer.from([0x30, 46]), der.subarray(2, 9), Buffer.from([0x23, 0x80]), der.subarray(9), Buffer.from([0, 0])])),
      "long-form-outer-length": wrap(Buffer.concat([Buffer.from([0x30, 0x81, der[1]]), der.subarray(2)])),
      "trailing-der": wrap(Buffer.concat([der, Buffer.from([0])])),
      "wrong-label": pem.replaceAll("PUBLIC KEY", "CERTIFICATE"),
      "missing-footer": pem.split("-----END")[0],
      "garbage": "not a PEM public key",
      "other-valid-key": Object.values(keys).find((other) => other.public_key_pem !== pem).public_key_pem,
      "preamble-postamble": `synthetic preamble\n${pem}synthetic postamble\n`,
      "two-public-blocks": pem + Object.values(keys).find((other) => other.public_key_pem !== pem).public_key_pem,
      "other-public-block-first": Object.values(keys).find((other) => other.public_key_pem !== pem).public_key_pem + pem,
      "inline-header": pem.replace("-----BEGIN PUBLIC KEY-----\n", "-----BEGIN PUBLIC KEY-----"),
      "inline-footer": pem.replace("\n-----END PUBLIC KEY-----", "-----END PUBLIC KEY-----"),
      "prefix-space": ` ${pem}`,
      "prefix-tab": `\t${pem}`,
      "prefix-nbsp": `\u00a0${pem}`,
      "header-trailing-space": pem.replace("-----\n", "----- \n"),
      "footer-trailing-space": pem.replace("-----END PUBLIC KEY-----\n", "-----END PUBLIC KEY----- \n"),
      "footer-no-final-newline": pem.trimEnd(),
      "cr-only": pem.replaceAll("\n", "\r"),
      "blank-before-body": pem.replace("-----\n", "-----\n\n"),
      "two-blanks-before-body": pem.replace("-----\n", "-----\n\n\n"),
      "blank-inside-body": pem.replace(base64, `${base64.slice(0, 16)}\n\n${base64.slice(16)}`),
      "space-inside-body": pem.replace(base64, `${base64.slice(0, 16)} ${base64.slice(16)}`),
      "tab-inside-body": pem.replace(base64, `${base64.slice(0, 16)}\t${base64.slice(16)}`),
      "formfeed-inside-body": pem.replace(base64, `${base64.slice(0, 16)}\f${base64.slice(16)}`),
      "vtab-inside-body": pem.replace(base64, `${base64.slice(0, 16)}\v${base64.slice(16)}`),
      "utf8-bom": `\ufeff${pem}`,
    };
    for (let unused = 1; unused <= 8; unused++) {
      const bytes = Buffer.from(der);
      bytes[11] = unused;
      variants[`unused-bits-${unused}`] = wrap(bytes);
    }
    for (const count of [8, 9]) variants[`length-octets-${count}`] = wrap(Buffer.concat([Buffer.from([0x30, 0x80 | count]), Buffer.alloc(count - 1), Buffer.from([42]), der.subarray(2)]));
    for (const depth of [2, 6, 7]) {
      let bits = der.subarray(9);
      for (let n = 0; n < depth; n++) bits = Buffer.concat([Buffer.from([0x23, bits.length]), bits]);
      variants[`nested-bitstring-${depth}`] = wrap(Buffer.concat([Buffer.from([0x30, 7 + bits.length]), der.subarray(2, 9), bits]));
    }
    const split = Buffer.concat([Buffer.from([3, 17, 0]), der.subarray(12, 28), Buffer.from([3, 17, 0]), der.subarray(28)]);
    variants["split-bitstring"] = wrap(Buffer.concat([Buffer.from([0x30, 7 + 2 + split.length]), der.subarray(2, 9), Buffer.from([0x23, split.length]), split]));
    for (const [label, value] of Object.entries(variants)) {
      yield emit(`pem:${label}`, { ...record, public_key_pem: value });
      yield emit(`public_key-pem:${label}`, { ...record, public_key: value });
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
