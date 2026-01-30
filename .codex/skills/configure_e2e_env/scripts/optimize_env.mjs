import fs from 'fs';
import path from 'path';

const composeFile = path.join(process.cwd(), 'docker-compose.yml');

if (!fs.existsSync(composeFile)) {
    console.error(`❌ docker-compose.yml not found at: ${composeFile}`);
    process.exit(1);
}

let content = fs.readFileSync(composeFile, 'utf-8');
let modified = false;

// Variables to update/ensure
const updates = {
    'RATE_LIMIT_IP_MAX_REQUESTS': '10000',
    'RATE_LIMIT_IP_WINDOW_SECONDS': '60',
    'RATE_LIMIT_USER_MAX_REQUESTS': '10000',
    'RATE_LIMIT_USER_WINDOW_SECONDS': '60'
};

// Simple line-based processing to avoid YAML parser dependency issues in non-node envs
// This assumes standard indentation and formatting.

const lines = content.split('\n');
const newLines = [];
let inBackendService = false;
let inEnvironment = false;
let indentation = '';

// Helper to check if a line sets a variable
function getVarName(line) {
    const match = line.match(/^\s*-\s*([A-Z_]+)=/);
    return match ? match[1] : null;
}

for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    // Detect backend service
    if (line.trim() === 'backend:') {
        inBackendService = true;
    } else if (inBackendService && line.match(/^  [a-z]+:/) && IsPreviousIndentationLess(line)) {
        // simplistic check: if we hit another service at same level or root
        // actually checking indentation is safer.
        // 'backend:' is usually indent 2. 'services:' is indent 0.
        // specific logic: if we see a line starting with 2 spaces and not 'backend:', we potentially exited.
        // simpler: just look for 'environment:' inside backend
    }

    // Detect environment block within backend
    if (inBackendService && line.trim() === 'environment:') {
        inEnvironment = true;
        // capture indentation of the next line usually
        newLines.push(line);

        // We will process subsequent lines until we hit a dedent or new block
        continue;
    }

    if (inEnvironment) {
        // If line is empty or comment, just keep it
        if (!line.trim() || line.trim().startsWith('#')) {
            newLines.push(line);
            continue;
        }

        // Check for dedent (end of environment block)
        const currentIndent = line.search(/\S/);
        if (currentIndent < 6) { // '      - ' is typical (6 spaces). '    environment:' is 4. So < 6 might be end.
            // Wait, if environment is at 4 spaces, items are at 6. 
            // If we see something at 4 spaces, we are out.

            // Check if we updated all vars
            Object.keys(updates).forEach(key => {
                if (!processedVars.has(key)) {
                    newLines.push(`      - ${key}=${updates[key]}`);
                    modified = true;
                    console.log(`➕ Added ${key}=${updates[key]}`);
                }
            });

            inEnvironment = false;
            inBackendService = false; // assume we are done with backend env
            newLines.push(line);
            continue;
        }

        // Check if this line is one of our target vars
        const varName = getVarName(line);
        if (varName && updates[varName]) {
            // Updated value
            newLines.push(`      - ${varName}=${updates[varName]}`);
            processedVars.add(varName);
            if (!line.includes(updates[varName])) {
                modified = true;
                console.log(`✏️  Updated ${varName} to ${updates[varName]}`);
            } else {
                console.log(`✅ ${varName} already set to ${updates[varName]}`);
            }
        } else {
            newLines.push(line);
        }

    } else {
        newLines.push(line);
    }
}

// Special case: if we never found environment block in backend, we should add it.
// This script assumes 'environment:' block exists for 'backend'. Use with caution.
// Given the known structure, it does exist.

// Re-scanning strategy failed logic above because streaming line validation is hard.
// Let's rely on string replacement for specific values if they exist, or append if missing?
// No, the above logic is too fragile for a generic script but might work for this specific file.
// Let's Try a simpler approach: Regex Replace of known patterns, and instructions if missing.

// Reset
const originalContent = fs.readFileSync(composeFile, 'utf-8');
let newContent = originalContent;

Object.keys(updates).forEach(key => {
    const regex = new RegExp(`${key}=[0-9]+`, 'g');
    const newValue = `${key}=${updates[key]}`;

    if (regex.test(newContent)) {
        newContent = newContent.replace(regex, newValue);
        if (originalContent !== newContent) {
            console.log(`✏️  Updated ${key} to ${updates[key]}`);
            modified = true;
        } else {
            console.log(`✅ ${key} is already optimized`);
        }
    } else {
        // If not found, we need to insert it. 
        // Locating 'environment:' under 'backend:' is tricky with just regex.
        // But we viewed the file, we know where they are.
        // Let's look for known anchors e.g. "RUST_LOG=debug"
        const anchor = 'RUST_LOG=debug';
        if (newContent.includes(anchor)) {
            newContent = newContent.replace(anchor, `${anchor}\n      - ${newValue}`);
            console.log(`➕ Added ${key}=${updates[key]}`);
            modified = true;
        } else {
            console.warn(`⚠️  Could not find anchor to insert ${key}. Please check manually.`);
        }
    }
});

if (modified) {
    fs.writeFileSync(composeFile, newContent, 'utf-8');
    console.log(`\n💾 Successfully updated docker-compose.yml`);
} else {
    console.log(`\n✨ No changes needed.`);
}

// Helper mock variable
const processedVars = new Set();
