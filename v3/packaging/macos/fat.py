"""Склеивает тонкие Mach-O (arm64, x86_64) в один universal: python3 fat.py out in1 in2..."""
import struct
import sys

ALIGN = 14  # 2^14 = 16 КБ, как у lipo для arm64

out, inputs = sys.argv[1], sys.argv[2:]
slices = [open(p, "rb").read() for p in inputs]
header = struct.pack(">II", 0xCAFEBABE, len(slices))
offset = 1 << ALIGN
entries, body = b"", b""
for data in slices:
    cputype, subtype = struct.unpack("<ii", data[4:12])
    entries += struct.pack(">iiIII", cputype, subtype, offset, len(data), ALIGN)
    pad = (-len(data)) % (1 << ALIGN)
    body += data + b"\0" * pad
    offset += len(data) + pad
first = header + entries
with open(out, "wb") as f:
    f.write(first + b"\0" * ((1 << ALIGN) - len(first)) + body)
