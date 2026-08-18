from __future__ import annotations

import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path


class TimingBrowserTests(unittest.TestCase):
    def test_completion_timing_uses_exact_events_and_keeps_fallback_visible(
        self,
    ) -> None:
        from playwright.sync_api import sync_playwright

        root = Path(__file__).resolve().parents[2]
        temporary = tempfile.TemporaryDirectory(prefix="pptx-timing-browser-")
        self.addCleanup(temporary.cleanup)
        html_value = os.environ.get("PPTX_TIMING_HTML")
        if html_value is None:
            work = Path(temporary.name)
            decks = work / "decks"
            html_path = work / "timing.html"
            subprocess.run(
                [
                    "python3",
                    "-m",
                    "evaluate.create_completion_decks",
                    "--output-dir",
                    str(decks),
                ],
                cwd=root,
                check=True,
            )
            subprocess.run(
                [
                    "cargo",
                    "run",
                    "-q",
                    "-p",
                    "pptx2html-cli",
                    "--",
                    str(decks / "timing-transitions.pptx"),
                    "-o",
                    str(html_path),
                ],
                cwd=root,
                check=True,
            )
        else:
            html_path = Path(html_value).resolve()
        self.assertTrue(html_path.is_file())

        screenshot_dir = Path(
            os.environ.get("PPTX_TIMING_SCREENSHOTS", ".omo/evidence/task-19-timing")
        )
        screenshot_dir.mkdir(parents=True, exist_ok=True)
        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(headless=True)
            page = browser.new_page(viewport={"width": 1100, "height": 820})
            page.goto(html_path.as_uri())
            page.evaluate(
                """() => {
                  window.__timingEvents = [];
                  window.__timingClicks = 0;
                  const names = [
                    'pptx2html:timing-group',
                    'pptx2html:timing-effect',
                    'pptx2html:timing-group-complete',
                    'pptx2html:transition',
                    'pptx2html:transition-complete'
                  ];
                  document.querySelector('#slide-1').addEventListener(
                    'click', event => {
                      if (event.target === document.querySelector('#slide-1')) window.__timingClicks++;
                    });
                  for (const name of names) {
                    document.addEventListener(name, event =>
                      window.__timingEvents.push({name, ...event.detail}));
                  }
                  window.__waitExact = (name, identity, slide) =>
                    new Promise((resolve, reject) => {
                      const listener = event => {
                        if (event.detail.identity !== identity || event.detail.slide !== slide) return;
                        clearTimeout(timeout);
                        document.removeEventListener(name, listener);
                        resolve(event.detail);
                      };
                      const timeout = setTimeout(() => {
                        document.removeEventListener(name, listener);
                        reject(new Error(`bounded event timeout: ${name}/${identity}`));
                      }, 3000);
                      document.addEventListener(name, listener);
                    });
                }"""
            )
            self.assertEqual(page.evaluate("window.__timingEvents"), [])
            page.evaluate(
                """() => {
                  window.__initialActionTransition = window.__waitExact(
                    'pptx2html:transition-complete', 'slide-transition-0', 2);
                }"""
            )
            page.locator("#slide-1 .shape-action-surface").click()
            page.wait_for_url("**#slide-2")
            page.evaluate(
                """async () => {
                  await window.__initialActionTransition;
                  delete window.__initialActionTransition;
                }"""
            )
            self.assertFalse(
                any(
                    event["name"].startswith("pptx2html:timing-group")
                    for event in page.evaluate("window.__timingEvents")
                )
            )
            page.evaluate(
                """() => {
                  for (const href of ['https://example.test/', 'mailto:timing@example.test']) {
                    const anchor = document.createElement('a');
                    anchor.href = href;
                    anchor.dataset.action = 'external';
                    anchor.className = 'shape-action-surface';
                    anchor.addEventListener('click', event => event.preventDefault());
                    document.querySelector('#slide-1').append(anchor);
                    anchor.dispatchEvent(new MouseEvent('click', {bubbles: true, cancelable: true}));
                    anchor.remove();
                  }
                }"""
            )
            self.assertFalse(
                any(
                    event["name"].startswith("pptx2html:timing-group")
                    for event in page.evaluate("window.__timingEvents")
                )
            )
            page.evaluate(
                """async () => {
                  const reset = window.__waitExact(
                    'pptx2html:transition-complete', 'slide-transition-0', 1);
                  location.hash = 'slide-1';
                  await reset;
                }"""
            )
            page.evaluate("window.__timingEvents=[]")
            page.screenshot(path=str(screenshot_dir / "initial.png"), full_page=True)

            initial = page.eval_on_selector_all(
                "#slide-1 .shape",
                """elements => elements.map(element => ({
                  id: element.dataset.pptxShapeId || null,
                  visibility: getComputedStyle(element).visibility,
                  text: element.textContent.trim()
                }))""",
            )
            self.assertEqual(
                {item["id"] for item in initial if item["visibility"] == "hidden"},
                {"2", "3"},
            )
            self.assertTrue(
                any(
                    item["id"] is None
                    and item["visibility"] == "visible"
                    and "unsupported stays visible" in item["text"]
                    for item in initial
                )
            )

            page.evaluate(
                """async () => {
                  const done = window.__waitExact(
                    'pptx2html:timing-group-complete', 'timing-10-group-0', 1);
                  document.querySelector('#slide-1').click();
                  await done;
                }"""
            )
            page.screenshot(
                path=str(screenshot_dir / "group-1-complete.png"), full_page=True
            )
            page.evaluate(
                """async () => {
                  const done = window.__waitExact(
                    'pptx2html:timing-group-complete', 'timing-20-group-1', 1);
                  document.querySelector('#slide-1').click();
                  await done;
                }"""
            )
            page.screenshot(
                path=str(screenshot_dir / "group-2-complete.png"), full_page=True
            )
            event_count = page.evaluate("window.__timingEvents.length")
            page.evaluate("document.querySelector('#slide-1').click()")
            self.assertEqual(page.evaluate("window.__timingEvents.length"), event_count)
            self.assertEqual(page.evaluate("window.__timingClicks"), 3)
            page.evaluate(
                """async () => {
                  const cut = window.__waitExact(
                    'pptx2html:transition-complete', 'slide-transition-0', 2);
                  location.hash = 'slide-2';
                  await cut;
                  const fade = window.__waitExact(
                    'pptx2html:transition-complete', 'slide-transition-0', 1);
                  location.hash = 'slide-1';
                  await fade;
                }"""
            )
            page.screenshot(
                path=str(screenshot_dir / "fade-complete.png"), full_page=True
            )
            events = page.evaluate("window.__timingEvents")
            fallback_json = page.eval_on_selector(
                "#pptx2html-diagnostics",
                "node => JSON.parse(node.textContent).filter(item => item.code === 'PRESENTATIONML_TIMING_FALLBACK')",
            )
            (screenshot_dir / "fallback.json").write_text(
                json.dumps(fallback_json, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
            (screenshot_dir / "events.json").write_text(
                json.dumps(events, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
            browser.close()

        self.assertEqual(
            [
                (event["name"], event.get("phase"), event["identity"])
                for event in events
            ],
            [
                ("pptx2html:timing-group", "start", "timing-10-group-0"),
                ("pptx2html:timing-effect", "start", "timing-11-effect-0"),
                ("pptx2html:timing-effect", "start", "timing-13-effect-1"),
                ("pptx2html:timing-effect", "complete", "timing-13-effect-1"),
                ("pptx2html:timing-effect", "complete", "timing-11-effect-0"),
                ("pptx2html:timing-effect", "start", "timing-15-effect-2"),
                ("pptx2html:timing-effect", "complete", "timing-15-effect-2"),
                ("pptx2html:timing-group-complete", None, "timing-10-group-0"),
                ("pptx2html:timing-group", "start", "timing-20-group-1"),
                ("pptx2html:timing-effect", "start", "timing-21-effect-3"),
                ("pptx2html:timing-effect", "complete", "timing-21-effect-3"),
                ("pptx2html:timing-group-complete", None, "timing-20-group-1"),
                ("pptx2html:transition", "start", "slide-transition-0"),
                ("pptx2html:transition-complete", None, "slide-transition-0"),
                ("pptx2html:transition", "start", "slide-transition-0"),
                ("pptx2html:transition-complete", None, "slide-transition-0"),
            ],
        )
        self.assertEqual(events[1]["delay"], 25)
        self.assertEqual(events[-1]["transition"], "fade")
        self.assertTrue(fallback_json)
        self.assertTrue(
            any("<p:animMotion" in item["raw_reference"] for item in fallback_json)
        )


if __name__ == "__main__":
    unittest.main()
