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
  await Promise.all([...document.images].map(async image => {
    try {
      await image.decode();
    } catch (error) {
      if (/^data:image\\/x-(?:emf|wmf);base64,/i.test(image.src)) return;
      throw error;
    }
  }));
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

NORMALIZE_PRESENTATION_SCRIPT = """
({format, width, height}) => {
  const nodes = format === "pptx"
    ? [...document.querySelectorAll(".slide")]
    : [...document.querySelectorAll('div[id^="page"][id$="-div"]')];
  if (!nodes.length) throw new Error("presentation units are missing");
  nodes.forEach(node => {
    const rect = node.getBoundingClientRect();
    if (!(rect.width > 0 && rect.height > 0)) {
      throw new Error(`invalid presentation geometry: ${node.id}`);
    }
    const existing = getComputedStyle(node).transform;
    if (existing !== "none") {
      const matrix = new DOMMatrix(existing);
      if (matrix.b !== 0 || matrix.c !== 0 || matrix.e !== 0 || matrix.f !== 0
          || !(matrix.a > 0 && matrix.d > 0)) {
        throw new Error(`unsupported presentation transform: ${node.id}`);
      }
    }
    node.style.transformOrigin = "top left";
    const prefix = existing === "none" ? "" : `${existing} `;
    node.style.transform = `${prefix}scale(${width / rect.width}, ${height / rect.height})`;
  });
}
"""

DISCOVER_UNITS_SCRIPT = """
({format, aggregatePages}) => {
  const corePresentation = format === "pptx";
  const nodes = corePresentation
    ? [...document.querySelectorAll(".slide")]
    : [...document.querySelectorAll('div[id^="page"][id$="-div"]')];
  let previousSlide = 0;
  const units = nodes.map((node, index) => {
    const ordinal = index + 1;
    if (corePresentation) {
      const match = /^slide-([1-9][0-9]*)$/.exec(node.id);
      const slide = match ? Number(match[1]) : 0;
      if (slide <= previousSlide) throw new Error(`nonsequential unit: ${node.id}`);
      previousSlide = slide;
    } else if (node.id !== `page${ordinal}-div`) {
      throw new Error(`nonsequential unit: ${node.id}`);
    }
    return {
      selector: `#${CSS.escape(node.id)}`,
      x: node.getBoundingClientRect().x,
      y: node.getBoundingClientRect().y,
      width: node.getBoundingClientRect().width,
      height: node.getBoundingClientRect().height,
    };
  });
  if (!aggregatePages) return units;
  if (corePresentation || !units.length) throw new Error("invalid paged-unit aggregation");
  const left = Math.min(...units.map(unit => unit.x));
  const top = Math.min(...units.map(unit => unit.y));
  const right = Math.max(...units.map(unit => unit.x + unit.width));
  const bottom = Math.max(...units.map(unit => unit.y + unit.height));
  return [{
    selectors: units.map(unit => unit.selector),
    pages: units.map(({x, y, width, height}) => ({x, y, width, height})),
    pageCount: units.length,
    textCodeUnits: nodes.reduce((total, node) => total + (node.textContent || "").length, 0),
    elementCount: nodes.reduce((total, node) => total + node.querySelectorAll("*").length + 1, 0),
    x: left,
    y: top,
    width: right - left,
    height: bottom - top,
  }];
}
"""

EXTERNAL_RESOURCES_SCRIPT = """
() => [...document.querySelectorAll(
  "img[src],link[href],script[src],source[src],video[src],audio[src],iframe[src],object[data],embed[src]"
)].map(element => element.src || element.href || element.data || "")
  .filter(value => /^(?:https?|file|wss?):/i.test(value))
"""

EXTRACT_DOM_SCRIPT = """
({selector, selectors, spreadsheet}) => {
  const roots = selectors
    ? selectors.map(value => document.querySelector(value))
    : [document.querySelector(selector)];
  if (roots.some(root => !root)) throw new Error("missing unit root");
  const rootRects = roots.map(root => root.getBoundingClientRect());
  const rootRect = {
    left: Math.min(...rootRects.map(rect => rect.left)),
    top: Math.min(...rootRects.map(rect => rect.top)),
    right: Math.max(...rootRects.map(rect => rect.right)),
    bottom: Math.max(...rootRects.map(rect => rect.bottom)),
  };
  rootRect.width = rootRect.right - rootRect.left;
  rootRect.height = rootRect.bottom - rootRect.top;
  const root = roots[0];
  const rootBounds = root.getBoundingClientRect();
  const scaleY = root.offsetHeight > 0 ? rootBounds.height / root.offsetHeight : 1;
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
    roots.flatMap(root => [...root.querySelectorAll(cellSelector)]).forEach((element, index) => {
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
  let textOrder = 0;
  roots.forEach(root => {
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    let node;
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
  });
  const objects = [];
  const addObject = (element, type, semantic) => {
    if (!semantic || !visible(element)) return;
    const rect = element.getBoundingClientRect();
    if (type === "image" && rect.width >= rootRect.width * 0.9
        && rect.height >= rootRect.height * 0.9) return;
    objects.push({type, semantic, box: relativeBox(rect)});
  };
  roots.flatMap(root => [...root.querySelectorAll("img")]).forEach(element =>
    addObject(element, "image", element.currentSrc || element.src || element.alt));
  roots.flatMap(root => [...root.querySelectorAll("svg")]).forEach(element =>
    addObject(element, "svg", element.outerHTML));
  roots.flatMap(root => [...root.querySelectorAll("a[href]")]).forEach(element =>
    addObject(element, "link", element.getAttribute("href")));
  roots.flatMap(root => [...root.querySelectorAll('input,button,select,textarea,[role="button"]')]).forEach(element => {
    const label = normalized(element.innerText || element.value
      || element.getAttribute("aria-label") || element.getAttribute("role") || element.tagName);
    addObject(element, "control", `${element.tagName.toLowerCase()}:${label}`);
  });
  return {texts, cells, objects};
}
"""
