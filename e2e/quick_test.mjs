import { chromium } from "playwright";

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage();

try {
  console.log("Navigating to login...");
  await page.goto("http://localhost:8080/login", { waitUntil: "networkidle", timeout: 30000 });
  console.log("Page loaded");
  
  console.log("Filling form...");
  await page.waitForSelector('#username', { timeout: 10000 });
  await page.fill('#username', 'admin');
  await page.fill('#password', 'admin123');
  await page.click('button[type="submit"]');
  
  console.log("Waiting for dashboard...");
  await page.waitForURL('**/dashboard', { timeout: 30000 });
  
  console.log("✅ SUCCESS! Reached dashboard at:", page.url());
  await browser.close();
  process.exit(0);
} catch (e) {
  console.error("❌ FAILED:", e.message);
  console.error("Current URL:", page.url());
  await browser.close();
  process.exit(1);
}
