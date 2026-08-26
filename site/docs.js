const groups = [
  {
    title: "Start here",
    items: [
      ["First install", "first-install.md"],
      ["Linux quickstart", "quickstart-linux.md"],
      ["Demo receipt", "demo.md"],
      ["Troubleshooting", "troubleshooting.md"],
    ],
  },
  {
    title: "Runtime & configuration",
    items: [
      ["Configuration ownership", "configuration.md"],
      ["Host backend", "host-backend.md"],
      ["Host-fast vs Docker", "host-fast-vs-docker.md"],
      ["Support scope", "support-scope.md"],
    ],
  },
  {
    title: "Receipts & CI",
    items: [
      ["CI examples", "ci-examples.md"],
      ["Reusable GitHub Action", "github-action.md"],
      ["Receipt schema v1", "receipt-schema-v1.md"],
      ["Operation plans & dry runs", "operation-plans.md"],
      ["Support matrix", "support-matrix.md"],
    ],
  },
  {
    title: "Workflows & fixtures",
    items: [
      ["APK loop recipes", "recipes.md"],
      ["Reference workflows", "reference-workflows.md"],
      ["Fixture testing", "fixture-testing.md"],
      ["Benchmarking", "benchmarking.md"],
    ],
  },
  {
    title: "Distribution & project",
    items: [
      ["Package distribution", "package-distribution.md"],
      ["Release process", "release-process.md"],
      ["Versioning policy", "versioning-policy.md"],
      ["Community", "community.md"],
      ["1.0 checklist", "1.0-checklist.md"],
    ],
  },
];

const navigation = document.querySelector("#doc-navigation");
const picker = document.querySelector("#doc-picker");
const article = document.querySelector("#doc-content");
const localDocFiles = new Set(groups.flatMap((group) => group.items.map(([, file]) => file)));
let currentDocument = "";

function escapeHtml(value) {
  return value.replace(/[&<>'"]/g, (character) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", "'": "&#039;", '"': "&quot;",
  })[character]);
}

function localUrl(url) {
  const cleanUrl = url.replace(/^\.\//, "");
  if (cleanUrl.startsWith("../assets/")) return { url: `assets/${cleanUrl.slice("../assets/".length)}`, file: null };
  if (localDocFiles.has(cleanUrl)) return { url: `#${encodeURIComponent(cleanUrl)}`, file: cleanUrl };
  if (cleanUrl.endsWith(".md")) return { url: `docs/${cleanUrl}`, file: null };
  return { url, file: null };
}

function inline(value) {
  return escapeHtml(value)
    .replace(/!\[([^\]]*)\]\(([^)\s]+)\)/g, (_, alt, url) => {
      const target = localUrl(url);
      return `<img src="${target.url}" alt="${alt}">`;
    })
    .replace(/`([^`]+)`/g, "<code>$1</code>")
    .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
    .replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (_, label, url) => {
      const target = localUrl(url);
      const attribute = target.file ? ` data-doc-file="${target.file}"` : "";
      return `<a href="${target.url}"${attribute}>${label}</a>`;
    });
}

function renderTable(lines) {
  const cells = (line) => line.trim().replace(/^\||\|$/g, "").split("|").map((cell) => inline(cell.trim()));
  const header = cells(lines[0]);
  const body = lines.slice(2).map((line) => `<tr>${cells(line).map((cell) => `<td>${cell}</td>`).join("")}</tr>`).join("");
  return `<div class="table-wrap"><table><thead><tr>${header.map((cell) => `<th>${cell}</th>`).join("")}</tr></thead><tbody>${body}</tbody></table></div>`;
}

function renderMarkdown(markdown) {
  const lines = markdown.replace(/\r\n/g, "\n").split("\n");
  const html = [];
  let code = [];
  let listType = null;
  let table = [];
  let paragraph = [];

  const flushParagraph = () => {
    if (paragraph.length) html.push(`<p>${inline(paragraph.join(" "))}</p>`);
    paragraph = [];
  };
  const flushList = () => {
    if (listType) html.push(`</${listType}>`);
    listType = null;
  };
  const flushTable = () => {
    if (table.length > 1) html.push(renderTable(table));
    else if (table.length) html.push(`<p>${inline(table[0])}</p>`);
    table = [];
  };

  for (const line of lines) {
    if (line.startsWith("```")) {
      flushParagraph(); flushList(); flushTable();
      if (code.length) {
        html.push(`<pre><code>${escapeHtml(code.join("\n"))}</code></pre>`);
        code = [];
      } else {
        code = [""];
      }
      continue;
    }
    if (code.length) { code.push(line); continue; }
    if (/^\|.*\|\s*$/.test(line)) { flushParagraph(); flushList(); table.push(line); continue; }
    flushTable();
    if (!line.trim()) { flushParagraph(); flushList(); continue; }
    const heading = line.match(/^(#{1,3})\s+(.+)$/);
    if (heading) {
      flushParagraph(); flushList();
      const level = heading[1].length;
      html.push(`<h${level}>${inline(heading[2])}</h${level}>`);
      continue;
    }
    if (/^---+$/.test(line.trim())) { flushParagraph(); flushList(); html.push("<hr>"); continue; }
    const bullet = line.match(/^[-*+]\s+(.+)$/);
    const ordered = line.match(/^\d+\.\s+(.+)$/);
    if (bullet || ordered) {
      flushParagraph();
      const nextType = ordered ? "ol" : "ul";
      if (listType && listType !== nextType) flushList();
      if (!listType) { html.push(`<${nextType}>`); listType = nextType; }
      html.push(`<li>${inline((bullet || ordered)[1])}</li>`);
      continue;
    }
    const quote = line.match(/^>\s?(.+)$/);
    if (quote) { flushParagraph(); flushList(); html.push(`<blockquote><p>${inline(quote[1])}</p></blockquote>`); continue; }
    paragraph.push(line.trim());
  }
  if (code.length) html.push(`<pre><code>${escapeHtml(code.join("\n"))}</code></pre>`);
  flushParagraph(); flushList(); flushTable();
  return html.join("\n");
}

function sourceUrl(file) {
  return `https://github.com/OthmaneBlial/rustdroid/blob/main/docs/${file}`;
}

async function selectDocument(file, title, push = true) {
  if (!file || file === currentDocument) return;
  currentDocument = file;
  document.querySelectorAll(".doc-navigation button").forEach((button) => {
    button.setAttribute("aria-current", String(button.dataset.file === file));
  });
  picker.value = file;
  article.innerHTML = `<p class="eyebrow">Documentation / ${escapeHtml(title)}</p><h1>${escapeHtml(title)}</h1><p>Loading the local guide…</p>`;
  if (push) history.replaceState(null, "", `#${encodeURIComponent(file)}`);

  try {
    const response = await fetch(`docs/${file}`);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const markdown = await response.text();
    article.innerHTML = `${renderMarkdown(markdown)}<p class="doc-source">This guide is shipped with the site from the RustDroid repository. <a href="${sourceUrl(file)}">View source on GitHub ↗</a></p>`;
    document.title = `${title} — RustDroid Docs`;
  } catch (error) {
    article.innerHTML = `<p class="eyebrow">Documentation / unavailable</p><h1>${escapeHtml(title)}</h1><p class="doc-error">The local documentation file could not be loaded. <a href="${sourceUrl(file)}">Open the source on GitHub ↗</a></p>`;
  }
}

for (const group of groups) {
  const groupElement = document.createElement("section");
  groupElement.className = "doc-group";
  groupElement.innerHTML = `<p class="doc-group-title">${group.title}</p>`;
  for (const [title, file] of group.items) {
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = title;
    button.dataset.file = file;
    button.addEventListener("click", () => selectDocument(file, title));
    groupElement.append(button);

    const option = document.createElement("option");
    option.value = file;
    option.textContent = `${group.title}: ${title}`;
    picker.append(option);
  }
  navigation.append(groupElement);
}

picker.addEventListener("change", () => {
  const choice = groups.flatMap((group) => group.items).find(([, file]) => file === picker.value);
  if (choice) selectDocument(choice[1], choice[0]);
});

article.addEventListener("click", (event) => {
  const link = event.target.closest("a[data-doc-file]");
  if (!link) return;
  event.preventDefault();
  const choice = groups.flatMap((group) => group.items).find(([, file]) => file === link.dataset.docFile);
  if (choice) selectDocument(choice[1], choice[0]);
});

const initialFile = decodeURIComponent(window.location.hash.slice(1));
const initialChoice = groups.flatMap((group) => group.items).find(([, file]) => file === initialFile) || groups[0].items[0];
selectDocument(initialChoice[1], initialChoice[0], false);
