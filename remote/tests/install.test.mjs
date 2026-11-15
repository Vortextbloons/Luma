import test from "node:test";
import assert from "node:assert/strict";
import { installSteps, offerInstall } from "../src/lib/install.ts";

const iphone = {
  userAgent: "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15",
  platform: "iPhone",
  maxTouchPoints: 5,
  standalone: false,
  hostname: "192.168.1.5",
};

const android = {
  userAgent: "Mozilla/5.0 (Linux; Android 15; Pixel 9) AppleWebKit/537.36",
  platform: "Linux armv8l",
  maxTouchPoints: 5,
  standalone: false,
  hostname: "192.168.1.5",
};

const desktop = {
  userAgent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
  platform: "Win32",
  maxTouchPoints: 0,
  standalone: false,
  hostname: "192.168.1.5",
};

test("home-screen steps match the device family", () => {
  assert.match(installSteps(iphone), /Share/);
  assert.match(installSteps(android), /browser menu/);
  assert.match(installSteps(desktop), /browser menu/);
});

test("the install hint only appears on a LAN address outside standalone mode", () => {
  assert.equal(offerInstall(iphone), true);
  assert.equal(offerInstall({ ...iphone, hostname: "localhost" }), false);
  assert.equal(offerInstall({ ...iphone, hostname: "127.0.0.1" }), false);
  assert.equal(offerInstall({ ...iphone, standalone: true }), false);
});
