import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

// Configuration
const projectRoot = process.cwd();
const screenshotsDir = process.env.SCREENSHOT_DIR || path.join(projectRoot, "e2e", "screenshots");
const reportFile = path.join(projectRoot, "VISUAL_REVIEW_REPORT.md");

if (!fs.existsSync(screenshotsDir)) {
    console.error(`Screenshots directory not found: ${screenshotsDir}`);
    process.exit(1);
}

// Helper to parse filename
function parseFilename(filename) {
    // Format: pageName-viewport-colorScheme.png
    // But pageName might contain hyphens, so we should look for the last two hyphens.
    // Actually, the format is controlled by us: ${pageDef.name}-${viewport.name}-${colorScheme}.png
    // viewport is desktop/mobile (no hyphens)
    // colorScheme is light/dark (no hyphens)
    // So we can split by '-' and take the last two as viewport/color.

    const ext = path.extname(filename);
    if (ext !== ".png") return null;

    const nameBody = path.basename(filename, ext);
    const parts = nameBody.split("-");

    if (parts.length < 3) return null;

    const colorScheme = parts.pop();
    const viewport = parts.pop();
    const pageName = parts.join("-");

    return { pageName, viewport, colorScheme, filename };
}

// Scan and Group
const files = fs.readdirSync(screenshotsDir);
const items = files.map(parseFilename).filter(Boolean);

const pages = {};

items.forEach(item => {
    if (!pages[item.pageName]) {
        pages[item.pageName] = {};
    }
    if (!pages[item.pageName][item.viewport]) {
        pages[item.pageName][item.viewport] = { light: null, dark: null };
    }
    pages[item.pageName][item.viewport][item.colorScheme] = item.filename;
});

// Generate Markdown
let md = "# Visual Review Report\n\nGenerated on: " + new Date().toLocaleString() + "\n\n";

// Sort pages alphabetically or by some order if desired
const sortedPageNames = Object.keys(pages).sort();

sortedPageNames.forEach(pageName => {
    md += `## ${pageName}\n\n`;

    const viewports = pages[pageName];
    Object.keys(viewports).sort().forEach(viewport => {
        md += `### ${viewport}\n\n`;

        const images = viewports[viewport];
        const lightPath = images.light ? `e2e/screenshots/${images.light}` : "";
        const darkPath = images.dark ? `e2e/screenshots/${images.dark}` : "";

        md += "| Light | Dark |\n| :---: | :---: |\n";
        md += `| ${lightPath ? `![Light](${lightPath})` : "N/A"} | ${darkPath ? `![Dark](${darkPath})` : "N/A"} |\n\n`;
    });

    md += "---\n\n";
});

fs.writeFileSync(reportFile, md);
console.log(`Report generated at: ${reportFile}`);
