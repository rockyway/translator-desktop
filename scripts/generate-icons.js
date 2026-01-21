/**
 * Icon Generator Script
 * Generates all required icon formats for Tauri desktop app
 *
 * Usage: node scripts/generate-icons.js
 *
 * Requirements:
 *   npm install sharp png-to-ico --save-dev
 */

import sharp from 'sharp';
import pngToIco from 'png-to-ico';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const ICONS_DIR = path.join(__dirname, '..', 'src-tauri', 'icons');
const SVG_PATH = path.join(ICONS_DIR, 'icon.svg');

// Icon sizes needed for Tauri
const SIZES = [
  { name: '32x32.png', size: 32 },
  { name: '128x128.png', size: 128 },
  { name: '128x128@2x.png', size: 256 },
  { name: 'icon-512.png', size: 512 }, // For high-res displays
];

// ICO sizes (Windows)
const ICO_SIZES = [16, 24, 32, 48, 64, 128, 256];

async function generateIcons() {
  console.log('🎨 Generating icons from SVG...\n');

  // Read the SVG file
  const svgBuffer = fs.readFileSync(SVG_PATH);

  // Generate PNG icons at each size
  for (const { name, size } of SIZES) {
    const outputPath = path.join(ICONS_DIR, name);
    await sharp(svgBuffer)
      .resize(size, size, {
        fit: 'contain',
        background: { r: 0, g: 0, b: 0, alpha: 0 }
      })
      .png()
      .toFile(outputPath);
    console.log(`  ✓ Generated ${name} (${size}x${size})`);
  }

  // Generate temporary PNGs for ICO
  console.log('\n📦 Generating ICO file...');
  const icoPngs = [];

  for (const size of ICO_SIZES) {
    const tempPath = path.join(ICONS_DIR, `_temp_${size}.png`);
    await sharp(svgBuffer)
      .resize(size, size, {
        fit: 'contain',
        background: { r: 0, g: 0, b: 0, alpha: 0 }
      })
      .png()
      .toFile(tempPath);
    icoPngs.push(tempPath);
  }

  // Create ICO file
  const icoBuffer = await pngToIco(icoPngs);
  fs.writeFileSync(path.join(ICONS_DIR, 'icon.ico'), icoBuffer);
  console.log(`  ✓ Generated icon.ico (${ICO_SIZES.join(', ')}px)`);

  // Clean up temp files
  for (const tempPath of icoPngs) {
    fs.unlinkSync(tempPath);
  }

  console.log('\n✅ All icons generated successfully!');
  console.log(`   Output directory: ${ICONS_DIR}`);
}

generateIcons().catch((err) => {
  console.error('❌ Error generating icons:', err);
  process.exit(1);
});
