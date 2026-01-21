// Generate minimal valid icons for Tauri
// This creates simple solid-color icons sufficient for development

const fs = require('fs');
const path = require('path');
const zlib = require('zlib');

const iconsDir = path.join(__dirname, '..', 'src-tauri', 'icons');

// CRC32 implementation for PNG chunks
function crc32(data) {
  let crc = 0xffffffff;
  const table = [];
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) {
      c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    }
    table[n] = c;
  }
  for (let i = 0; i < data.length; i++) {
    crc = table[(crc ^ data[i]) & 0xff] ^ (crc >>> 8);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function createPNGChunk(type, data) {
  const typeBytes = Buffer.from(type, 'ascii');
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length, 0);
  const crcData = Buffer.concat([typeBytes, data]);
  const crcValue = crc32(crcData);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crcValue, 0);
  return Buffer.concat([length, typeBytes, data, crc]);
}

function createPNG(width, height, r, g, b) {
  // PNG signature
  const signature = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

  // IHDR chunk
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;  // bit depth
  ihdr[9] = 2;  // color type (RGB)
  ihdr[10] = 0; // compression
  ihdr[11] = 0; // filter
  ihdr[12] = 0; // interlace
  const ihdrChunk = createPNGChunk('IHDR', ihdr);

  // Create raw image data (filter byte + RGB pixels per row)
  const rawData = [];
  for (let y = 0; y < height; y++) {
    rawData.push(0); // filter type: none
    for (let x = 0; x < width; x++) {
      rawData.push(r, g, b);
    }
  }

  // Compress with zlib
  const compressed = zlib.deflateSync(Buffer.from(rawData), { level: 9 });
  const idatChunk = createPNGChunk('IDAT', compressed);

  // IEND chunk
  const iendChunk = createPNGChunk('IEND', Buffer.alloc(0));

  return Buffer.concat([signature, ihdrChunk, idatChunk, iendChunk]);
}

function createICO(pngBuffers) {
  // ICO header
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);     // reserved
  header.writeUInt16LE(1, 2);     // type: 1 = ICO
  header.writeUInt16LE(pngBuffers.length, 4); // number of images

  // Calculate offsets
  let dataOffset = 6 + (16 * pngBuffers.length);
  const entries = [];
  const dataChunks = [];

  for (const { width, png } of pngBuffers) {
    const entry = Buffer.alloc(16);
    entry[0] = width >= 256 ? 0 : width;  // width (0 = 256)
    entry[1] = width >= 256 ? 0 : width;  // height (0 = 256)
    entry[2] = 0;  // color palette
    entry[3] = 0;  // reserved
    entry.writeUInt16LE(1, 4);    // color planes
    entry.writeUInt16LE(32, 6);   // bits per pixel
    entry.writeUInt32LE(png.length, 8);  // size of image data
    entry.writeUInt32LE(dataOffset, 12); // offset to image data

    entries.push(entry);
    dataChunks.push(png);
    dataOffset += png.length;
  }

  return Buffer.concat([header, ...entries, ...dataChunks]);
}

// Blue color: #3B82F6 (Tailwind blue-500)
const r = 0x3b, g = 0x82, b = 0xf6;

// Generate PNGs
const sizes = {
  '32x32.png': 32,
  '128x128.png': 128,
  '128x128@2x.png': 256,
};

console.log('Generating icons in:', iconsDir);

for (const [filename, size] of Object.entries(sizes)) {
  const png = createPNG(size, size, r, g, b);
  fs.writeFileSync(path.join(iconsDir, filename), png);
  console.log(`Created ${filename} (${size}x${size})`);
}

// Generate ICO with multiple sizes
const icoSizes = [16, 32, 48, 256];
const icoPngs = icoSizes.map(size => ({
  width: size,
  png: createPNG(size, size, r, g, b)
}));
const ico = createICO(icoPngs);
fs.writeFileSync(path.join(iconsDir, 'icon.ico'), ico);
console.log('Created icon.ico');

// Generate ICNS placeholder (for macOS) - just create a symlink or copy
// For now, we'll skip ICNS as it's complex and not needed on Windows
console.log('Note: icon.icns skipped (not needed on Windows)');

console.log('\nAll icons generated successfully!');
