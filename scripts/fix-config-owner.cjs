#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");

function hostUser() {
  return process.env.SQLG_HOST_USER || process.env.USER || "";
}

function hostIds(user) {
  try {
    const entry = fs
      .readFileSync("/host/etc/passwd", "utf8")
      .split("\n")
      .map((line) => line.split(":"))
      .find(([name]) => name === user);
    if (entry) {
      return { uid: Number(entry[2]), gid: Number(entry[3]) };
    }
  } catch {
    // Fall through to the common first-user Linux ids.
  }
  return { uid: 1000, gid: 1000 };
}

function chownTree(target, uid, gid) {
  if (!fs.existsSync(target)) {
    return;
  }
  fs.chownSync(target, uid, gid);
  for (const item of fs.readdirSync(target, { withFileTypes: true })) {
    const itemPath = path.join(target, item.name);
    fs.chownSync(itemPath, uid, gid);
    if (item.isDirectory()) {
      chownTree(itemPath, uid, gid);
    }
  }
}

const target = process.argv[2] || "/root/.config/sqlg";
const { uid, gid } = hostIds(hostUser());
chownTree(target, uid, gid);
