STATIC_STYLE = """
*, *::before, *::after {
  animation: none !important;
  caret-color: transparent !important;
  transition: none !important;
}
.slide { display: block !important; visibility: visible !important; opacity: 1 !important; }
"""

READINESS_SCRIPT = """
async () => {
  await document.fonts.ready;
  if (document.fonts.status !== "loaded") throw new Error("fonts are not loaded");
  await Promise.all([...document.images].map(image => image.decode()));
  if (document.readyState !== "complete") {
    await new Promise(resolve => addEventListener("load", resolve, {once: true}));
  }
  const snapshot = () => [...document.querySelectorAll(".slide,div[id^='page'][id$='-div']")]
    .map(node => {
      const rect = node.getBoundingClientRect();
      return [rect.x, rect.y, rect.width, rect.height];
    });
  const before = JSON.stringify(snapshot());
  await new Promise(resolve => requestAnimationFrame(resolve));
  await new Promise(resolve => requestAnimationFrame(resolve));
  if (before !== JSON.stringify(snapshot())) throw new Error("geometry is unstable");
}
"""

DISCOVER_UNITS_SCRIPT = """
format => {
  const corePresentation = format === "pptx";
  const nodes = corePresentation
    ? [...document.querySelectorAll(".slide")]
    : [...document.querySelectorAll('div[id^="page"][id$="-div"]')];
  return nodes.map((node, index) => {
    const ordinal = index + 1;
    const expected = corePresentation ? `slide-${ordinal}` : `page${ordinal}-div`;
    if (node.id !== expected) throw new Error(`nonsequential unit: ${node.id}`);
    return {
      selector: `#${CSS.escape(node.id)}`,
      width: node.getBoundingClientRect().width,
      height: node.getBoundingClientRect().height,
    };
  });
}
"""

EXTERNAL_RESOURCES_SCRIPT = """
() => [...document.querySelectorAll(
  "img[src],link[href],script[src],source[src],video[src],audio[src],iframe[src],object[data],embed[src]"
)].map(element => element.src || element.href || element.data || "")
  .filter(value => /^(?:https?|file|wss?):/i.test(value))
"""

EXTRACT_DOM_SCRIPT = """
({selector, spreadsheet}) => {
  const root = document.querySelector(selector);
  if (!root) throw new Error(`missing unit ${selector}`);
  const rootRect = root.getBoundingClientRect();
  const scaleY = root.offsetHeight > 0 ? rootRect.height / root.offsetHeight : 1;
  const normalized = value => value.normalize("NFC").replace(/\\s+/g, " ").trim();
  const intersects = rect => rect.right > rootRect.left && rect.left < rootRect.right
    && rect.bottom > rootRect.top && rect.top < rootRect.bottom;
  const visible = element => {
    const style = getComputedStyle(element);
    const rect = element.getBoundingClientRect();
    return style.display !== "none" && style.visibility !== "hidden"
      && Number(style.opacity) > 0 && rect.width > 0 && rect.height > 0
      && intersects(rect);
  };
  const relativeBox = rect => {
    const left = Math.max(rect.left, rootRect.left);
    const top = Math.max(rect.top, rootRect.top);
    const right = Math.min(rect.right, rootRect.right);
    const bottom = Math.min(rect.bottom, rootRect.bottom);
    return {
      x: left - rootRect.left,
      y: top - rootRect.top,
      width: Math.max(0, right - left),
      height: Math.max(0, bottom - top),
    };
  };
  const union = rects => {
    const left = Math.min(...rects.map(rect => rect.left));
    const top = Math.min(...rects.map(rect => rect.top));
    const right = Math.max(...rects.map(rect => rect.right));
    const bottom = Math.max(...rects.map(rect => rect.bottom));
    return relativeBox({left, top, right, bottom, width: right-left, height: bottom-top});
  };
  const textBaseline = (element, text, rect) => {
    const style = getComputedStyle(element);
    const canvas = document.createElement("canvas");
    const context = canvas.getContext("2d");
    context.font = style.font || `${style.fontSize} ${style.fontFamily}`;
    const metrics = context.measureText(text);
    const descent = Number(metrics.actualBoundingBoxDescent || 0) * scaleY;
    return rect.bottom - rootRect.top - descent;
  };
  const cellSelector = "[data-cell-coordinate][data-worksheet]";
  const cells = [];
  if (spreadsheet) {
    [...root.querySelectorAll(cellSelector)].forEach((element, index) => {
      if (!visible(element)) return;
      const coordinate = element.dataset.cellCoordinate || "";
      const worksheet = normalized(element.dataset.worksheet || "");
      if (!worksheet || !/^[A-Z]{1,3}[1-9][0-9]{0,6}$/.test(coordinate)) return;
      const value = normalized(element.innerText || element.textContent || "");
      if (!value) return;
      const rect = element.getBoundingClientRect();
      cells.push({
        worksheet,
        coordinate,
        value,
        box: relativeBox(rect),
        baseline: textBaseline(element, value, rect),
        order: index,
      });
    });
  }
  const texts = [];
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  let node;
  let textOrder = 0;
  while ((node = walker.nextNode())) {
    const parent = node.parentElement;
    if (!parent || ["SCRIPT", "STYLE", "NOSCRIPT"].includes(parent.tagName)) continue;
    if (spreadsheet && parent.closest(cellSelector)) continue;
    if (!visible(parent)) continue;
    const content = node.nodeValue || "";
    const fragments = [];
    for (let offset = 0; offset < content.length; offset++) {
      const range = document.createRange();
      range.setStart(node, offset);
      range.setEnd(node, offset + 1);
      const rect = range.getBoundingClientRect();
      if (!(rect.width > 0 && rect.height > 0 && intersects(rect))) continue;
      const current = fragments[fragments.length - 1];
      if (current && Math.abs(current.rects[0].top - rect.top) < 1) {
        current.value += content[offset];
        current.rects.push(rect);
      } else {
        fragments.push({value: content[offset], rects: [rect]});
      }
    }
    fragments.forEach(fragment => {
      const value = normalized(fragment.value);
      if (!value) return;
      const last = fragment.rects[fragment.rects.length - 1];
      texts.push({
        value,
        box: union(fragment.rects),
        baseline: textBaseline(parent, value, last),
        order: textOrder++,
      });
    });
  }
  const objects = [];
  const addObject = (element, type, semantic) => {
    if (!semantic || !visible(element)) return;
    const rect = element.getBoundingClientRect();
    if (type === "image" && rect.width >= rootRect.width * 0.9
        && rect.height >= rootRect.height * 0.9) return;
    objects.push({type, semantic, box: relativeBox(rect)});
  };
  [...root.querySelectorAll("img")].forEach(element =>
    addObject(element, "image", element.currentSrc || element.src || element.alt));
  [...root.querySelectorAll("svg")].forEach(element =>
    addObject(element, "svg", element.outerHTML));
  [...root.querySelectorAll("a[href]")].forEach(element =>
    addObject(element, "link", element.getAttribute("href")));
  [...root.querySelectorAll('input,button,select,textarea,[role="button"]')].forEach(element => {
    const label = normalized(element.innerText || element.value
      || element.getAttribute("aria-label") || element.getAttribute("role") || element.tagName);
    addObject(element, "control", `${element.tagName.toLowerCase()}:${label}`);
  });
  return {texts, cells, objects};
}
"""
