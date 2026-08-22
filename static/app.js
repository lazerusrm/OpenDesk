function copyTextValue(text, button) {
  if (!text) {
    return;
  }
  const write = navigator.clipboard?.writeText(text);
  if (!write) {
    return;
  }
  write
    .then(() => {
      const original = button.textContent;
      button.textContent = "Copied";
      window.setTimeout(() => {
        button.textContent = original;
      }, 1200);
    })
    .catch(() => {});
}

function normalizePath(path) {
  if (!path) {
    return "/";
  }
  const trimmed = path.replace(/\/+$/, "");
  return trimmed === "" ? "/" : trimmed;
}

function sidebarTargetPath(pathname) {
  const path = normalizePath(pathname);
  if (path === "/" || path === "/dashboard") {
    return "/";
  }
  if (path === "/account" || path.startsWith("/account/")) {
    return "/account";
  }
  const prefixes = [
    ["/devices/", "/devices"],
    ["/users/", "/users"],
    ["/access-groups/", "/access-groups"],
    ["/address-books/", "/address-books"],
    ["/deployment/", "/deployment"],
  ];
  for (const [prefix, href] of prefixes) {
    if (path.startsWith(prefix)) {
      return href;
    }
  }
  return path;
}

function markActiveSidebar() {
  const nav = document.querySelector(".sidebar nav");
  if (!nav) {
    return;
  }
  const target = sidebarTargetPath(location.pathname);
  for (const link of nav.querySelectorAll("a[href]")) {
    const href = normalizePath(link.getAttribute("href"));
    const active = href === target || (target === "/" && (href === "/" || href === "/dashboard"));
    link.classList.toggle("is-active", active);
    if (active) {
      link.setAttribute("aria-current", "page");
    } else {
      link.removeAttribute("aria-current");
    }
  }
}

function isArchiveSubmit(form, submitter) {
  const action = form.getAttribute("action") || "";
  const label = (submitter?.textContent || "").trim().toLowerCase();
  const actionArchive = /\/archive\/?$/i.test(action);
  const labelArchive = label === "archive";
  const unarchive = /unarchive/i.test(action) || /unarchive/i.test(label);
  return (actionArchive || labelArchive) && !unarchive;
}

function onReady(fn) {
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", fn);
  } else {
    fn();
  }
}

document.addEventListener("click", (event) => {
  const navToggle = event.target.closest("[data-nav-toggle]");
  if (navToggle) {
    const nav = document.getElementById(navToggle.getAttribute("aria-controls"));
    const open = nav?.classList.toggle("is-open") ?? false;
    navToggle.setAttribute("aria-expanded", String(open));
    return;
  }
  const button = event.target.closest("[data-copy-text], [data-copy-input]");
  if (!button) {
    return;
  }
  event.preventDefault();
  const inputId = button.getAttribute("data-copy-input");
  const text = inputId
    ? document.getElementById(inputId)?.value?.trim()
    : button.getAttribute("data-copy-text");
  copyTextValue(text, button);
});

document.addEventListener("submit", (event) => {
  const form = event.target;
  if (!(form instanceof HTMLFormElement)) {
    return;
  }
  if (!isArchiveSubmit(form, event.submitter)) {
    return;
  }
  if (!window.confirm("Archive this device?")) {
    event.preventDefault();
  }
});

onReady(markActiveSidebar);
