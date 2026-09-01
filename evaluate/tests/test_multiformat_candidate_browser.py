import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from evaluate.multiformat_candidate_browser_checks import aggregate_geometry
from evaluate.multiformat_candidate_browser import (
    MAX_AGGREGATE_ELEMENTS,
    MAX_AGGREGATE_HEIGHT,
    MAX_AGGREGATE_PAGES,
    MAX_AGGREGATE_PIXELS,
    MAX_AGGREGATE_TEXT_CODE_UNITS,
    MAX_AGGREGATE_WIDTH,
    CandidateCaptureError,
    _validate_aggregate_unit,
    capture_html_units,
)
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_inventory import parse_inventory
from evaluate.multiformat_candidate_scripts import ISOLATE_DISCOVERED_UNIT_SCRIPT
from evaluate.multiformat_visual_metrics import png_dimensions


def _instrument_discovered_unit_isolation(isolation_script: str) -> str:
    return (
        """payload => {
  const keys = Object.keys(payload).sort().join(",");
  if (keys !== "index,token") throw new Error(`unexpected isolation payload: ${keys}`);
  const state = window.__multiformatCandidateUnitIsolationState;
  if (!state) throw new Error("missing isolation state");
  if (!state.__candidateAccessProbe) {
    const probe = {state: 0, length: 0, index: 0};
    const rawNodes = state.nodes;
    const rawDisplays = state.displays;
    const instrumentedByRaw = new WeakMap();
    const rawByInstrumented = new WeakMap();
    const instrumentNode = node => {
      let instrumented = instrumentedByRaw.get(node);
      if (!instrumented) {
        instrumented = new Proxy(node, {
          get(target, property) {
            if (property !== "style") {
              throw new Error(`isolation attempted node access: ${String(property)}`);
            }
            return Reflect.get(target, property, target);
          },
          defineProperty() {
            throw new Error("isolation attempted node mutation");
          },
          deleteProperty() {
            throw new Error("isolation attempted node mutation");
          },
          getOwnPropertyDescriptor() {
            throw new Error("isolation attempted node introspection");
          },
          getPrototypeOf() {
            throw new Error("isolation attempted node introspection");
          },
          has() {
            throw new Error("isolation attempted node search");
          },
          ownKeys() {
            throw new Error("isolation attempted node enumeration");
          },
          preventExtensions() {
            throw new Error("isolation attempted node mutation");
          },
          set() {
            throw new Error("isolation attempted node mutation");
          },
          setPrototypeOf() {
            throw new Error("isolation attempted node mutation");
          },
        });
        instrumentedByRaw.set(node, instrumented);
        rawByInstrumented.set(instrumented, node);
      }
      return instrumented;
    };
    const instrumentedNodes = new Proxy(rawNodes, {
      get(target, property, receiver) {
        const isIndex = typeof property === "string" && /^(?:0|[1-9]\\d*)$/.test(property);
        if (property === "length") {
          probe.length += 1;
          return Reflect.get(target, property, receiver);
        }
        if (!isIndex) {
          throw new Error(`isolation attempted node collection access: ${String(property)}`);
        }
        probe.index += 1;
        return instrumentNode(Reflect.get(target, property, receiver));
      },
      defineProperty() {
        throw new Error("isolation attempted node collection mutation");
      },
      deleteProperty() {
        throw new Error("isolation attempted node collection mutation");
      },
      getOwnPropertyDescriptor() {
        throw new Error("isolation attempted node collection introspection");
      },
      getPrototypeOf() {
        throw new Error("isolation attempted node collection introspection");
      },
      has() {
        throw new Error("isolation attempted node collection search");
      },
      ownKeys() {
        throw new Error("isolation attempted node enumeration");
      },
      set() {
        throw new Error("isolation attempted node collection mutation");
      },
    });
    const displayCapability = new Proxy(Object.create(null), {
      get(_target, property) {
        if (property !== "get") {
          throw new Error(`isolation attempted display access: ${String(property)}`);
        }
        return node => {
          const rawNode = rawByInstrumented.get(node);
          if (!rawNode) throw new Error("isolation used an uninstrumented node");
          return rawDisplays.get(rawNode);
        };
      },
      defineProperty() {
        throw new Error("isolation attempted display mutation");
      },
      deleteProperty() {
        throw new Error("isolation attempted display mutation");
      },
      getOwnPropertyDescriptor() {
        throw new Error("isolation attempted display introspection");
      },
      getPrototypeOf() {
        throw new Error("isolation attempted display introspection");
      },
      has() {
        throw new Error("isolation attempted display search");
      },
      ownKeys() {
        throw new Error("isolation attempted display enumeration");
      },
      preventExtensions() {
        throw new Error("isolation attempted display mutation");
      },
      set() {
        throw new Error("isolation attempted display mutation");
      },
      setPrototypeOf() {
        throw new Error("isolation attempted display mutation");
      },
    });
    const stateCapability = new Proxy(Object.create(null), {
      get(_target, property) {
        probe.state += 1;
        if (property === "token") return state.token;
        if (property === "nodes") return instrumentedNodes;
        if (property === "displays") return displayCapability;
        if (property === "active") return state.active ? instrumentNode(state.active) : null;
        throw new Error(`isolation attempted state access: ${String(property)}`);
      },
      set(_target, property, value) {
        probe.state += 1;
        if (property !== "active") {
          throw new Error(`isolation attempted state mutation: ${String(property)}`);
        }
        const rawNode = rawByInstrumented.get(value);
        if (!rawNode) throw new Error("isolation assigned an uninstrumented node");
        state.active = rawNode;
        return true;
      },
      defineProperty() {
        throw new Error("isolation attempted state mutation");
      },
      deleteProperty() {
        throw new Error("isolation attempted state mutation");
      },
      getOwnPropertyDescriptor() {
        throw new Error("isolation attempted state introspection");
      },
      getPrototypeOf() {
        throw new Error("isolation attempted state introspection");
      },
      has() {
        throw new Error("isolation attempted state search");
      },
      ownKeys() {
        throw new Error("isolation attempted state enumeration");
      },
      preventExtensions() {
        throw new Error("isolation attempted state mutation");
      },
      setPrototypeOf() {
        throw new Error("isolation attempted state mutation");
      },
    });
    Object.defineProperty(state, "__candidateAccessProbe", {
      configurable: false,
      enumerable: true,
      value: probe,
      writable: false,
    });
    state.__candidateIsolationCapability = stateCapability;
  }
  const probe = state.__candidateAccessProbe;
  probe.state = 0;
  probe.length = 0;
  probe.index = 0;
  const blocked = [
    [Document.prototype, "createNodeIterator"],
    [Document.prototype, "createTreeWalker"],
    [Document.prototype, "evaluate"],
    [Document.prototype, "getElementById"],
    [Document.prototype, "getElementsByClassName"],
    [Document.prototype, "getElementsByName"],
    [Document.prototype, "getElementsByTagName"],
    [Document.prototype, "getElementsByTagNameNS"],
    [Document.prototype, "querySelector"],
    [Document.prototype, "querySelectorAll"],
    [DocumentFragment.prototype, "querySelector"],
    [DocumentFragment.prototype, "querySelectorAll"],
    [Element.prototype, "getElementsByClassName"],
    [Element.prototype, "getElementsByTagName"],
    [Element.prototype, "getElementsByTagNameNS"],
    [Element.prototype, "querySelector"],
    [Element.prototype, "querySelectorAll"],
  ];
  const originals = blocked.map(([owner, name]) => {
    const descriptor = Object.getOwnPropertyDescriptor(owner, name);
    if (!descriptor || typeof descriptor.value !== "function") {
      throw new Error(`missing DOM scan descriptor: ${name}`);
    }
    return [owner, name, descriptor];
  });
  try {
    originals.forEach(([owner, name, descriptor]) => {
      Object.defineProperty(owner, name, {
        ...descriptor,
        value: () => { throw new Error(`isolation attempted DOM scan: ${name}`); },
      });
    });
    const runtime = globalThis;
    const documentCapability = new Proxy(Object.create(null), {
      get(_target, property) {
        throw new Error(`isolation attempted document access: ${String(property)}`);
      },
    });
    const windowCapability = new Proxy(Object.create(null), {
      get(_target, property) {
        if (property !== "__multiformatCandidateUnitIsolationState") {
          throw new Error(`isolation attempted window access: ${String(property)}`);
        }
        return state.__candidateIsolationCapability;
      },
    });
    const invokeIsolated = new Function(
      "payload",
      "window",
      "document",
      "globalThis",
      "self",
      "top",
      "parent",
      "frames",
      `"use strict"; return ("""
        + isolation_script
        + """)(payload);`,
    );
    invokeIsolated(
      payload,
      windowCapability,
      documentCapability,
      windowCapability,
      windowCapability,
      windowCapability,
      windowCapability,
      windowCapability,
    );
  } finally {
    originals.forEach(([owner, name, descriptor]) => {
      Object.defineProperty(owner, name, descriptor);
    });
  }
  if ((probe.state !== 6 && probe.state !== 8) || probe.length !== 1 || probe.index !== 1) {
    throw new Error(`nonconstant isolation-state/DOM collection access: state=${probe.state}, length=${probe.length}, index=${probe.index}`);
  }
}"""
    )


class MultiFormatCandidateBrowserTests(unittest.TestCase):
    def test_aggregate_geometry_uses_cumulative_scaled_boundaries(self) -> None:
        geometry = aggregate_geometry(((1000, 1001), (900, 1002), (800, 1003)))

        self.assertEqual(geometry.scaled_width, 384)
        self.assertEqual(geometry.pages[0].scaled_top, 0)
        self.assertEqual(geometry.pages[-1].scaled_bottom, geometry.scaled_height)
        for left, right in zip(geometry.pages, geometry.pages[1:]):
            self.assertEqual(left.scaled_bottom, right.scaled_top)

    def test_aggregate_resource_limits_fail_closed(self) -> None:
        page = {
            "x": 0,
            "y": 0,
            "width": 100,
            "height": 100,
        }
        value = {
            "pages": [page],
            "pageCount": 1,
            "textCodeUnits": MAX_AGGREGATE_TEXT_CODE_UNITS,
            "elementCount": MAX_AGGREGATE_ELEMENTS,
            "x": 0,
            "y": 0,
            "width": 100,
            "height": 100,
        }
        _validate_aggregate_unit(value)

        invalid_values = {
            "pages": {
                **value,
                "pages": [page] * (MAX_AGGREGATE_PAGES + 1),
                "pageCount": MAX_AGGREGATE_PAGES + 1,
            },
            "width": {**value, "width": MAX_AGGREGATE_WIDTH + 1},
            "height": {**value, "height": MAX_AGGREGATE_HEIGHT + 1},
            "pixels": {**value, "width": MAX_AGGREGATE_PIXELS, "height": 2},
            "text": {**value, "textCodeUnits": MAX_AGGREGATE_TEXT_CODE_UNITS + 1},
            "elements": {**value, "elementCount": MAX_AGGREGATE_ELEMENTS + 1},
            "nonfinite": {**value, "width": float("inf")},
            "offset": {**value, "x": 1, "width": MAX_AGGREGATE_WIDTH},
        }
        for name, invalid in invalid_values.items():
            with self.subTest(name=name), self.assertRaises(CandidateCaptureError):
                _validate_aggregate_unit(invalid)

    def test_paged_html_captures_semantic_inventory(self) -> None:
        html = """
        <html><body>
          <div id="page1-div" style="position:relative;width:300px;height:200px;background:#fff">
            <span style="position:absolute;left:10px;top:10px;font:16px Arial">Hello</span>
            <a href="https://example.test/docs" style="position:absolute;left:10px;top:40px">Link</a>
            <img alt="logo" src="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=" style="position:absolute;left:80px;top:20px;width:20px;height:20px">
            <svg style="position:absolute;left:120px;top:20px;width:20px;height:20px"><rect width="20" height="20"/></svg>
          </div>
        </body></html>
        """
        with tempfile.TemporaryDirectory() as temp_dir:
            result = capture_html_units(
                html,
                DocumentFormat.DOCX,
                ("unit-1",),
                Path(temp_dir),
                source_track="blind",
                aggregate_paged_units=False,
            )

            inventory = parse_inventory(result.units[0].inventory, "unit-1")
            self.assertIn("Hello", [item.value for item in inventory.texts])
            self.assertIn("Link", [item.value for item in inventory.texts])
            self.assertEqual(
                {item.object_type for item in inventory.objects},
                {"image", "link", "svg"},
            )

    def test_blind_paged_html_rejects_isolation_script_lexical_state_alias(
        self,
    ) -> None:
        html = """
        <html><body>
          <div id="page1-div" style="position:relative;width:300px;height:200px">
            <span>Page 1</span>
          </div>
        </body></html>
        """
        lexically_exposed_isolation = """({index}) => {
  const leaked = [runtime, state];
  const active = leaked[1].nodes[index];
  if (!active || !leaked[0].document) {
    throw new Error("missing leaked isolation alias");
  }
  active.style.setProperty("display", "block", "important");
}"""
        with (
            tempfile.TemporaryDirectory() as temp_dir,
            patch(
                "evaluate.multiformat_candidate_browser.ISOLATE_DISCOVERED_UNIT_SCRIPT",
                _instrument_discovered_unit_isolation(lexically_exposed_isolation),
            ),
            self.assertRaisesRegex(CandidateCaptureError, "runtime is not defined"),
        ):
            capture_html_units(
                html,
                DocumentFormat.DOCX,
                ("unit-1",),
                Path(temp_dir),
                source_track="blind",
                aggregate_paged_units=False,
            )

    def test_blind_paged_html_uses_constant_isolation_state_access_without_changing_evidence(
        self,
    ) -> None:
        page_count = 300
        pages = "".join(
            f'<div id="page{ordinal}-div" '
            'style="position:relative;display:block !important;width:300px;'
            'height:200px;background:#fff">'
            f'<span style="position:absolute;left:10px;top:10px;font:16px Arial">'
            f"Page {ordinal}</span>"
            "</div>"
            for ordinal in range(1, page_count + 1)
        )
        html = f"<html><body>{pages}</body></html>"
        instrumented_isolation = _instrument_discovered_unit_isolation(
            ISOLATE_DISCOVERED_UNIT_SCRIPT
        )
        with tempfile.TemporaryDirectory() as temp_dir:
            output = Path(temp_dir)
            baseline = capture_html_units(
                html,
                DocumentFormat.DOCX,
                tuple(f"unit-{ordinal}" for ordinal in range(1, page_count + 1)),
                output / "baseline",
                source_track="blind",
                aggregate_paged_units=False,
            )

            with patch(
                "evaluate.multiformat_candidate_browser.ISOLATE_DISCOVERED_UNIT_SCRIPT",
                instrumented_isolation,
            ):
                result = capture_html_units(
                    html,
                    DocumentFormat.DOCX,
                    tuple(f"unit-{ordinal}" for ordinal in range(1, page_count + 1)),
                    output / "instrumented",
                    source_track="blind",
                    aggregate_paged_units=False,
                )

            self.assertEqual(len(result.units), page_count)
            for ordinal, (baseline_unit, unit) in enumerate(
                zip(baseline.units, result.units, strict=True), start=1
            ):
                self.assertEqual(
                    baseline_unit.inventory.read_bytes(), unit.inventory.read_bytes()
                )
                self.assertEqual(baseline_unit.png.read_bytes(), unit.png.read_bytes())
                self.assertEqual(png_dimensions(unit.png), (300, 200))
                inventory = parse_inventory(unit.inventory, f"unit-{ordinal}")
                self.assertEqual(
                    [item.value for item in inventory.texts], [f"Page {ordinal}"]
                )
            self.assertEqual(result.external_requests, ())
            self.assertEqual(baseline.external_requests, ())

    def test_conformance_paged_html_aggregates_all_pages_into_one_unit(self) -> None:
        pages = "".join(
            f'<div id="page{ordinal}-div" '
            'style="position:relative;width:300px;height:200px;background:#fff">'
            f'<span style="position:absolute;left:10px;top:10px">Page {ordinal}</span>'
            "</div>"
            for ordinal in range(1, 7)
        )
        with tempfile.TemporaryDirectory() as temp_dir:
            with (
                patch(
                    "evaluate.multiformat_candidate_browser.INITIALIZE_DISCOVERED_UNIT_ISOLATION_SCRIPT",
                    '() => { throw new Error("aggregate capture must not isolate"); }',
                ),
                patch(
                    "evaluate.multiformat_candidate_browser.ISOLATE_DISCOVERED_UNIT_SCRIPT",
                    '() => { throw new Error("aggregate capture must not transition"); }',
                ),
            ):
                result = capture_html_units(
                    f"<html><body>{pages}</body></html>",
                    DocumentFormat.DOC,
                    ("document-unit",),
                    Path(temp_dir),
                    source_track="conformance",
                    aggregate_paged_units=True,
                )

            self.assertEqual(len(result.units), 1)
            self.assertEqual(png_dimensions(result.units[0].png), (300, 1200))
            inventory = parse_inventory(result.units[0].inventory, "document-unit")
            self.assertEqual(
                [item.value for item in inventory.texts],
                [f"Page {ordinal}" for ordinal in range(1, 7)],
            )

    def test_spreadsheet_capture_requires_explicit_cell_coordinates(self) -> None:
        html = """
        <html><body>
          <div id="page1-div" style="position:relative;width:300px;height:200px">
            <div data-cell-coordinate="A1" data-worksheet="Sheet1">42</div>
            <div data-cell-coordinate="B2">must not invent worksheet</div>
          </div>
        </body></html>
        """
        with tempfile.TemporaryDirectory() as temp_dir:
            result = capture_html_units(
                html,
                DocumentFormat.XLSX,
                ("sheet-unit-1",),
                Path(temp_dir),
                source_track="conformance",
                aggregate_paged_units=True,
            )

            inventory = parse_inventory(
                result.units[0].inventory,
                "sheet-unit-1",
            )
            self.assertEqual(inventory.texts, ())
            self.assertEqual(len(inventory.cells), 1)
            self.assertEqual(inventory.cells[0].coordinate, "A1")
            self.assertEqual(inventory.cells[0].displayed_value, "42")

    def test_presentation_capture_renders_exact_960_by_540(self) -> None:
        html = """
        <html><body>
          <div class="slide" id="slide-1" data-slide="1"
               style="position:relative;width:960px;height:540px;background:#fff">
            <span style="position:absolute;left:100px;top:500px;font:20px Arial">Bottom</span>
          </div>
        </body></html>
        """
        with tempfile.TemporaryDirectory() as temp_dir:
            result = capture_html_units(
                html,
                DocumentFormat.PPTX,
                ("slide-1",),
                Path(temp_dir),
                source_track="conformance",
                aggregate_paged_units=False,
                expected_browser_version="Google Chrome for Testing 151.0.7922.34",
            )

            self.assertEqual(png_dimensions(result.units[0].png), (960, 540))
            inventory = parse_inventory(result.units[0].inventory, "slide-1")
            self.assertLess(inventory.texts[0].box.y, 540)

    def test_presentation_capture_normalizes_noncanonical_dimensions(self) -> None:
        html = """
        <html><body>
          <div class="slide" id="slide-1" data-slide="1"
               style="position:relative;width:960px;height:720px;background:#fff;
                      transform:scale(0.5);transform-origin:top left">
            <span style="position:absolute;left:100px;top:680px;font:20px Arial">Bottom</span>
          </div>
        </body></html>
        """
        with tempfile.TemporaryDirectory() as temp_dir:
            result = capture_html_units(
                html,
                DocumentFormat.PPTX,
                ("slide-1",),
                Path(temp_dir),
                source_track="conformance",
                aggregate_paged_units=False,
            )

            self.assertEqual(png_dimensions(result.units[0].png), (960, 540))
            inventory = parse_inventory(result.units[0].inventory, "slide-1")
            self.assertLess(inventory.texts[0].box.y, 540)

    def test_presentation_capture_preserves_order_across_hidden_slide_gaps(
        self,
    ) -> None:
        html = """
        <html><body>
          <div class="slide" id="slide-1" data-slide="1"
               style="position:relative;width:960px;height:540px"></div>
          <div class="slide" id="slide-3" data-slide="3"
               style="position:relative;width:960px;height:540px"></div>
        </body></html>
        """
        with tempfile.TemporaryDirectory() as temp_dir:
            result = capture_html_units(
                html,
                DocumentFormat.PPTX,
                ("visible-slide-1", "visible-slide-2"),
                Path(temp_dir),
                source_track="conformance",
                aggregate_paged_units=False,
            )

            self.assertEqual(len(result.units), 2)
            self.assertEqual(png_dimensions(result.units[1].png), (960, 540))

    def test_legacy_ppt_uses_native_page_container_as_slide(self) -> None:
        html = """
        <html><body>
          <div id="page1-div"
               style="position:relative;width:960px;height:540px;background:#fff">
            <span>Legacy slide</span>
          </div>
        </body></html>
        """
        with tempfile.TemporaryDirectory() as temp_dir:
            result = capture_html_units(
                html,
                DocumentFormat.PPT,
                ("slide-page-1",),
                Path(temp_dir),
                source_track="conformance",
                aggregate_paged_units=False,
            )

            self.assertEqual(png_dimensions(result.units[0].png), (960, 540))

    def test_presentation_capture_preserves_unsupported_vector_metadata(self) -> None:
        html = """
        <html><body>
          <div class="slide" id="slide-1" data-slide="1"
               style="position:relative;width:960px;height:540px">
            <img alt="unsupported-preview" src="data:image/x-emf;base64,AQID">
          </div>
        </body></html>
        """
        with tempfile.TemporaryDirectory() as temp_dir:
            result = capture_html_units(
                html,
                DocumentFormat.PPTX,
                ("slide-1",),
                Path(temp_dir),
                source_track="conformance",
                aggregate_paged_units=False,
            )

            inventory = parse_inventory(result.units[0].inventory, "slide-1")
            self.assertEqual(len(inventory.objects), 1)
            self.assertEqual(inventory.objects[0].object_type, "image")

    def test_external_request_attempt_fails_closed(self) -> None:
        html = """
        <html><body>
          <div id="page1-div" style="width:300px;height:200px">
            <img src="https://example.invalid/tracker.png">
          </div>
        </body></html>
        """
        with tempfile.TemporaryDirectory() as temp_dir:
            with self.assertRaisesRegex(
                CandidateCaptureError,
                "network request",
            ):
                capture_html_units(
                    html,
                    DocumentFormat.PDF,
                    ("unit-1",),
                    Path(temp_dir),
                    source_track="conformance",
                    aggregate_paged_units=True,
                )
