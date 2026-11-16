export const appCss: string = Object.values(
  import.meta.glob<string>("/src/App.css", { query: "?raw", import: "default", eager: true }),
)[0].replace(/\r\n/g, "\n");

export function ruleBody(selector: string): string {
  const start = appCss.indexOf(`\n${selector} {`);
  if (start === -1) throw new Error(`no rule for ${selector}`);
  const open = appCss.indexOf("{", start);
  const close = appCss.indexOf("}", open);
  return appCss.slice(open + 1, close);
}
