import { chromium } from "playwright";

const browser = await chromium.launch({ headless: false });
const page = await browser.newPage();

try {
  console.log("Loading frontend...");
  await page.goto("http://localhost:8080/login", { waitUntil: "networkidle", timeout: 30000 });
  
  console.log("Page loaded, taking screenshot...");
  await page.screenshot({ path: "./screenshots/before-login.png" });
  
  console.log("Waiting for username field...");
  await page.waitForSelector('#username', { timeout: 30000 });
  
  console.log("Filling credentials...");
  await page.fill('#username', 'admin');
  await page.fill('#password', 'admin123');
  
  console.log("Taking pre-submit screenshot...");
  await page.screenshot({ path: "./screenshots/filled-form.png" });
  
  console.log("Clicking submit...");
  await page.click('button[type="submit"]');
  
  console.log("Waiting for redirect...");
  await page.waitForURL('**/dashboard', { timeout: 30000 });
  
  console.log("SUCCESS! URL:", page.url());
  await page.screenshot({ path: "./screenshots/dashboard.png" });
  
  await browser.close();
  process.exit(0);
} catch (e) {
  console.error("FAILED:", e.message);
  console.error("Current URL:", page.url());
  await page.screenshot({ path: "./screenshots/error.png" });
  await browser.close();
  process.exit(1);
}
