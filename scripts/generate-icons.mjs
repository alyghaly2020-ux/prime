import sharp from "sharp";
import { writeFileSync } from "fs";

const INPUT = "prime.png";
const OUT_DIR = "src-tauri/icons";

const sizes = [
  { name: "32x32.png", size: 32 },
  { name: "128x128.png", size: 128 },
  { name: "128x128@2x.png", size: 256 },
];

async function main() {
  for (const { name, size } of sizes) {
    await sharp(INPUT)
      .resize(size, size, { fit: "contain", background: { r: 0, g: 0, b: 0, alpha: 0 } })
      .png()
      .toFile(`${OUT_DIR}/${name}`);
    console.log(`  ✓ ${name} (${size}x${size})`);
  }

  // Create a 256x256 PNG for ICO conversion (keep icon.ico and icon.icns as-is)
  // Tauri will auto-convert 128x128@2x for bundlers
  console.log("\nDone! ICO and ICNS kept as-is.");
}

main().catch(console.error);
