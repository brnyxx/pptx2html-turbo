from __future__ import annotations

import os
from pathlib import Path
import unittest


class TimingBrowserTests(unittest.TestCase):
    def test_completion_timing_uses_exact_events_and_keeps_fallback_visible(self) -> None:
        html_value = os.environ.get("PPTX_TIMING_HTML")
        if html_value is None:
            self.skipTest("PPTX_TIMING_HTML is not set")
        html_path = Path(html_value).resolve()
        self.assertTrue(html_path.is_file())

        try:
            from playwright.sync_api import sync_playwright
        except ImportError:
            self.skipTest("Playwright is unavailable")

        screenshot_dir = Path(
            os.environ.get("PPTX_TIMING_SCREENSHOTS", "/tmp/pptx-timing-browser")
        )
        screenshot_dir.mkdir(parents=True, exist_ok=True)
        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(headless=True)
            page = browser.new_page(viewport={"width": 1100, "height": 820})
            page.goto(html_path.as_uri())
            page.evaluate(
                """() => {
                  window.__timingEvents = [];
                  const names = [
                    'pptx2html:timing-group',
                    'pptx2html:timing-effect',
                    'pptx2html:timing-group-complete',
                    'pptx2html:transition',
                    'pptx2html:transition-complete'
                  ];
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
            browser.close()

        self.assertEqual(
            [
                (event["name"], event.get("phase"), event["identity"])
                for event in events[:8]
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
            ],
        )
        self.assertEqual(events[-1]["name"], "pptx2html:transition-complete")
        self.assertEqual(events[-1]["transition"], "fade")


if __name__ == "__main__":
    unittest.main()
