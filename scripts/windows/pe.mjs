// Read the executable's architecture and runtime imports before creating an installable package.
export function inspectPe(buffer) {
  const invalid = () => { throw new Error("Invalid Windows PE image."); };
  if (buffer.length < 64 || buffer.readUInt16LE(0) !== 0x5a4d) invalid();
  const pe = buffer.readUInt32LE(0x3c);
  if (pe + 24 > buffer.length || buffer.readUInt32LE(pe) !== 0x4550) invalid();
  const architecture = { 0x8664: "x64", 0xaa64: "arm64" }[buffer.readUInt16LE(pe + 4)];
  if (!architecture) throw new Error("Only x64 and ARM64 Windows PE images are supported.");
  const optionalSize = buffer.readUInt16LE(pe + 20);
  const optional = pe + 24;
  const sectionCount = buffer.readUInt16LE(pe + 6);
  const sections = optional + optionalSize;
  if (optionalSize < 128 || sections + sectionCount * 40 > buffer.length || buffer.readUInt16LE(optional) !== 0x20b) invalid();
  const offset = (rva) => {
    for (let index = 0; index < sectionCount; index++) {
      const section = sections + index * 40;
      const start = buffer.readUInt32LE(section + 12);
      const size = buffer.readUInt32LE(section + 16);
      if (rva >= start && rva - start < size) {
        const position = buffer.readUInt32LE(section + 20) + rva - start;
        if (position >= buffer.length) invalid();
        return position;
      }
    }
    invalid();
  };
  const imports = [];
  const importRva = buffer.readUInt32LE(optional + 120);
  if (importRva) {
    let descriptor = offset(importRva);
    for (let count = 0; count < 512; count++, descriptor += 20) {
      if (descriptor + 20 > buffer.length) invalid();
      const nameRva = buffer.readUInt32LE(descriptor + 12);
      if (!nameRva) return { architecture, imports };
      const begin = offset(nameRva);
      const end = buffer.indexOf(0, begin);
      if (end < begin || end - begin > 256) invalid();
      imports.push(buffer.toString("ascii", begin, end));
    }
    invalid();
  }
  return { architecture, imports };
}
