// playwright login test
import { chromium } from "playwright";
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
  await page.goto(base + '/settings', { waitUntil: 'domcontentloaded' });
  await page.waitForSelector('text=本人対応申請', { timeout: 15000 });
  console.log('Login redirect OK:', page.url());
  await browser.close();
  process.exit(0);
} catch (e) {
  console.error('Login test failed:', e);
  console.error('Final URL:', page.url());
  await browser.close();
  process.exit(1);
}
