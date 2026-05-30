module.exports = {
  root: true,
  ignorePatterns: [
    "dist/",
    "node_modules/",
  ],
  env: {
    browser: true,
    es2020: true,
    node: true,
  },
  parser: "@typescript-eslint/parser",
  parserOptions: {
    ecmaFeatures: {
      jsx: true,
    },
    ecmaVersion: "latest",
    sourceType: "module",
  },
  plugins: ["@typescript-eslint"],
  extends: ["eslint:recommended"],
  rules: {
    "no-undef": "off",
    "no-unused-vars": "off",
  },
};
