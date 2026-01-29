import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

// Define legacy patterns (regex or simple strings)
const legacyPatterns = [
    { pattern: /rounded px-2 py-1/g, name: "Legacy Input Padding/Rounding" },
    { pattern: /border border-form-control-border/g, name: "Legacy 1px Border (New is border-2)" },
    { pattern: /mt-1 w-full/g, name: "Legacy Margin (Check alignment)" },
    { pattern: /px-3 py-1/g, name: "Legacy Button Padding" },
];

const excludeDirs = ['target', 'node_modules', '.git', '.agent', 'dist', 'pkg'];
const scanExtensions = ['.rs', '.html', '.css'];

function scanFile(filePath) {
    const content = fs.readFileSync(filePath, 'utf-8');
    const params = [];

    legacyPatterns.forEach(({ pattern, name }) => {
        let match;
        // Reset regex state if global
        if (pattern.global) pattern.lastIndex = 0;

        while ((match = pattern.exec(content)) !== null) {
            params.push({
                line: content.substring(0, match.index).split('\n').length,
                name: name,
                match: match[0]
            });
        }
    });

    return params;
}

function walkDir(dir) {
    const files = fs.readdirSync(dir);

    files.forEach(file => {
        const fullPath = path.join(dir, file);
        const stat = fs.statSync(fullPath);

        if (stat.isDirectory()) {
            if (!excludeDirs.includes(file)) {
                walkDir(fullPath);
            }
        } else {
            if (scanExtensions.includes(path.extname(file))) {
                const results = scanFile(fullPath);
                if (results.length > 0) {
                    console.log(`\n📄 ${path.relative(process.cwd(), fullPath)}`);
                    results.forEach(r => {
                        console.log(`  L${r.line}: ${r.name} ("${r.match}")`);
                    });
                }
            }
        }
    });
}

console.log("🔍 Scanning for legacy UI patterns...");
walkDir(path.join(process.cwd(), 'frontend/src')); // Focus on frontend
console.log("\n✅ Scan complete.");
