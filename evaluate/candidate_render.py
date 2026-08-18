"""
Render pptx2html-rs HTML output to PNG screenshots via Playwright.

Takes HTML files produced by pptx2html-rs and captures per-slide screenshots
using headless Chromium.  Each slide is expected to be a top-level element
with class "slide" or a section/div with a data-slide-index attribute.

Output structure:
    candidates/{html_stem}/slide_0.png
    candidates/{html_stem}/slide_1.png
    ...
"""

from __future__ import annotations

import argparse
import logging
from pathlib import Path

logger = logging.getLogger(__name__)

# Default viewport matching standard slide dimensions (px)
DEFAULT_VIEWPORT_WIDTH = 960
DEFAULT_VIEWPORT_HEIGHT = 540


def render_html_to_pngs(
    html_path: Path,
    output_dir: Path,
    viewport_width: int = DEFAULT_VIEWPORT_WIDTH,
    viewport_height: int = DEFAULT_VIEWPORT_HEIGHT,
    output_width: int | None = None,
    output_height: int | None = None,
) -> list[Path]:
    """Render an HTML file to per-slide PNG screenshots.

    Uses Playwright headless Chromium to capture each slide element.
    Falls back to full-page screenshot if no slide elements are detected.

    Args:
        html_path: Path to the HTML file to render.
        output_dir: Directory to save PNG screenshots.
        viewport_width: Browser viewport width in pixels.
        viewport_height: Browser viewport height in pixels.

    Returns:
        List of paths to generated PNG files.
    """
    from playwright.sync_api import sync_playwright

    output_dir.mkdir(parents=True, exist_ok=True)
    png_paths: list[Path] = []

    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        context = browser.new_context(
            viewport={"width": viewport_width, "height": viewport_height},
            device_scale_factor=1,
        )
        page = context.new_page()

        # Load HTML file
        file_url = html_path.resolve().as_uri()
        page.goto(file_url, wait_until="networkidle")

        # Wait for rendering
        page.wait_for_timeout(500)

        # Try to find slide elements
        slide_selectors = [
            ".slide",
            "[data-slide-index]",
            "section.slide",
            "div.slide",
        ]

        slides = []
        for selector in slide_selectors:
            slides = page.query_selector_all(selector)
            if slides:
                logger.debug(
                    "Found %d slides with selector '%s'",
                    len(slides),
                    selector,
                )
                break

        if slides:
            # Screenshot each slide element individually
            for idx, slide_el in enumerate(slides):
                png_path = output_dir / f"slide_{idx}.png"

                dimensions = slide_el.bounding_box()
                if dimensions is None:
                    raise RuntimeError(f"Slide {idx} has no renderable bounding box")
                target_width = output_width or viewport_width
                target_height = output_height or viewport_height
                scale_x = target_width / dimensions["width"]
                scale_y = target_height / dimensions["height"]
                if abs(scale_x - scale_y) > 1e-9:
                    raise RuntimeError(
                        "Candidate output aspect ratio does not match slide: "
                        f"slide={dimensions['width']}x{dimensions['height']} "
                        f"output={target_width}x{target_height}"
                    )
                page.evaluate(
                    """([slide, scale, width, height]) => {
                        document.documentElement.style.margin = "0";
                        document.body.style.margin = "0";
                        document.body.style.overflow = "hidden";
                        slide.style.transformOrigin = "top left";
                        slide.style.transform = `scale(${scale})`;
                        const shell = slide.parentElement;
                        if (shell) {
                            shell.style.width = `${width}px`;
                            shell.style.height = `${height}px`;
                        }
                    }""",
                    [slide_el, scale_x, target_width, target_height],
                )
                slide_el.scroll_into_view_if_needed()
                page.screenshot(
                    path=str(png_path),
                    clip={
                        "x": 0,
                        "y": 0,
                        "width": target_width,
                        "height": target_height,
                    },
                )
                png_paths.append(png_path)
                logger.debug("  Captured slide %d -> %s", idx, png_path.name)
        else:
            # Fallback: single full-page screenshot
            logger.info(
                "No slide elements found in %s, taking full-page screenshot",
                html_path.name,
            )
            png_path = output_dir / "slide_0.png"
            page.screenshot(
                path=str(png_path),
                full_page=True,
            )
            png_paths.append(png_path)

        context.close()
        browser.close()

    logger.info(
        "Rendered %d screenshot(s) for %s",
        len(png_paths),
        html_path.name,
    )
    return png_paths


def render_directory(
    html_dir: Path,
    output_dir: Path,
    viewport_width: int = DEFAULT_VIEWPORT_WIDTH,
    viewport_height: int = DEFAULT_VIEWPORT_HEIGHT,
) -> dict[str, list[Path]]:
    """Render all HTML files in a directory to PNG screenshots.

    Args:
        html_dir: Directory containing HTML files.
        output_dir: Base directory for screenshot output.
        viewport_width: Browser viewport width in pixels.
        viewport_height: Browser viewport height in pixels.

    Returns:
        Dict mapping html_stem -> list of PNG paths.
    """
    from playwright.sync_api import Error as PlaywrightError

    html_files = sorted(html_dir.glob("*.html"))
    if not html_files:
        logger.error("No HTML files found in %s", html_dir)
        return {}

    logger.info(
        "Rendering %d HTML files to screenshots...", len(html_files)
    )

    results: dict[str, list[Path]] = {}

    for html_file in html_files:
        try:
            pngs = render_html_to_pngs(
                html_path=html_file,
                output_dir=output_dir / html_file.stem,
                viewport_width=viewport_width,
                viewport_height=viewport_height,
            )
            results[html_file.stem] = pngs
        except (OSError, PlaywrightError) as exc:
            logger.error(
                "Failed to render %s: %s", html_file.name, exc
            )

    logger.info("Done. Rendered %d HTML files.", len(results))
    return results


def main() -> None:
    """CLI entry point."""
    parser = argparse.ArgumentParser(
        description="Render pptx2html-rs HTML output to PNG via Playwright"
    )
    parser.add_argument(
        "--html-dir",
        type=Path,
        required=True,
        help="Directory containing HTML files",
    )
    parser.add_argument(
        "--output",
        type=Path,
        required=True,
        help="Output directory for PNG screenshots",
    )
    parser.add_argument(
        "--width",
        type=int,
        default=DEFAULT_VIEWPORT_WIDTH,
        help=f"Viewport width (default: {DEFAULT_VIEWPORT_WIDTH})",
    )
    parser.add_argument(
        "--height",
        type=int,
        default=DEFAULT_VIEWPORT_HEIGHT,
        help=f"Viewport height (default: {DEFAULT_VIEWPORT_HEIGHT})",
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="Enable debug logging",
    )

    args = parser.parse_args()

    logging.basicConfig(
        level=logging.DEBUG if args.verbose else logging.INFO,
        format="%(levelname)s: %(message)s",
    )

    render_directory(
        html_dir=args.html_dir.resolve(),
        output_dir=args.output.resolve(),
        viewport_width=args.width,
        viewport_height=args.height,
    )


if __name__ == "__main__":
    main()
