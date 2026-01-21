import { chromium } from "playwright";

const browser = await chromium.launch({ headless: false });
const page = await browser.newPage();

// Capture console logs
page.on('console', msg => console.log('BROWSER:', msg.text()));

// Capture network requests
page.on('request', request => {
  if (request.url().includes('/api/')) {
    console.log('REQUEST:', request.method(), request.url());
  }
});

page.on('response', response => {
  if (response.url().includes('/api/')) {
    console.log('RESPONSE:', response.status(), response.url());
  }
});

try {
  await page.goto("http://localhost:8080/login", { waitUntil: "networkidle" });
  await page.waitForSelector('#username');
  await page.fill('#username', 'admin');
  await page.fill('#password', 'admin123');
  await page.click('button[type="submit"]');
  
  // Wait a bit to see what happens
  await page.waitForTimeout(5000);
  
  console.log("Final URL:", page.url());
  await page.screenshot({ path: "./screenshots/final.png" });
  
  await browser.close();
} catch (e) {
  console.error("Error:", e.message);
  await page.screenshot({ path: "./screenshots/error.png" });
  await browser.close();
  process.exit(1);
}
