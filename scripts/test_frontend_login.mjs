// playwright login test
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const { chromium } = require(require.resolve("playwright", {
  paths: ["./e2e", process.cwd()],
}));
const base = process.env.FRONTEND_BASE_URL || "http://localhost:8080";
const browser = await chromium.launch({ headless: true });
const page = await browser.newPage();
try {
  await page.goto(base + "/login", { waitUntil: "domcontentloaded" });
  await page.waitForSelector('#username');
  await page.fill('#username', 'admin');
  await page.fill('#password', 'admin123');
  await page.click('button[type="submit"]');
  await page.waitForURL('**/dashboard', { timeout: 15000 });
  console.log('Login redirect OK:', page.url());
  await browser.close();
  process.exit(0);
} catch (e) {
  console.error('Login test failed:', e);
  console.error('Final URL:', page.url());
  await browser.close();
  process.exit(1);
}
