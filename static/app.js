function copyTextValue(text, button) {
  if (!text) {
    return;
  }
  const write = navigator.clipboard?.writeText(text);
  if (!write) {
    return;
  }
  write.then(() => {
    const original = button.textContent;
    button.textContent = "Copied";
    window.setTimeout(() => {
      button.textContent = original;
    }, 1200);
  });
}

document.addEventListener("click", (event) => {
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