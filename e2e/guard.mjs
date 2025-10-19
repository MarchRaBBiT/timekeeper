import { chromium } from 'playwright';
const base = process.env.FRONTEND_BASE_URL || 'http://localhost:8080';
const browser = await chromium.launch({ headless: true });
const page = await browser.newPage();
try {
  await page.goto(base + '/login', { waitUntil: 'domcontentloaded' });
  await page.fill('#username', 'admin');
  await page.fill('#password', 'admin123');
  await page.click('button[type="submit"]');
  await page.waitForURL('**/dashboard', { timeout: 15000 });
  // Clear tokens to simulate logged-out session
  await page.evaluate(() => { localStorage.removeItem('access_token'); localStorage.removeItem('refresh_token'); });
  // Navigate to a protected route
  await page.goto(base + '/attendance');
  await page.waitForURL('**/login', { timeout: 15000 });
  console.log('Guard redirect OK:', page.url());
  await browser.close();
  process.exit(0);
} catch (e) {
  console.error('Guard test failed:', e);
  console.error('Final URL:', page.url());
  await browser.close();
  process.exit(1);
}
