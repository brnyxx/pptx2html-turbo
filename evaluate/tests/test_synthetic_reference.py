import importlib.util
import tempfile
import unittest
from pathlib import Path

from evaluate import synthetic_reference
from evaluate.synthetic_scene import EMU_PER_PIXEL, create_synthetic_corpus


class SyntheticReferenceTests(unittest.TestCase):
    def test_reference_emitter_module_exists(self) -> None:
        module = importlib.util.find_spec("evaluate.synthetic_reference")

        self.assertIsNotNone(module)

    def test_emitter_writes_independent_html_for_all_scenes(self) -> None:
        # Given
        corpus = create_synthetic_corpus()
        write_corpus = getattr(
            synthetic_reference,
            "write_synthetic_reference_corpus",
            None,
        )

        # When/Then
        self.assertTrue(callable(write_corpus))
        with tempfile.TemporaryDirectory() as tmp:
            paths = write_corpus(corpus, Path(tmp))

            self.assertEqual(
                [path.name for path in paths],
                [f"synthetic_{index:02d}.html" for index in range(1, 11)],
            )
            for deck, path in zip(corpus, paths, strict=True):
                html = path.read_text(encoding="utf-8")
                self.assertEqual(html.count('class="slide"'), 10)
                self.assertEqual(
                    html.count('class="synthetic-rectangle"'),
                    80,
                )
                for scene in deck.scenes:
                    self.assertIn(
                        f'data-scene-id="{scene.scene_id}"',
                        html,
                    )
                rectangle = deck.scenes[0].rectangles[0]
                self.assertIn(
                    (
                        f"left:{rectangle.x / EMU_PER_PIXEL:.1f}px;"
                        f"top:{rectangle.y / EMU_PER_PIXEL:.1f}px;"
                        f"width:{rectangle.width / EMU_PER_PIXEL:.1f}px;"
                        f"height:{rectangle.height / EMU_PER_PIXEL:.1f}px;"
                        f"background:#{rectangle.fill}"
                    ),
                    html,
                )

    def test_reference_emitter_has_no_candidate_or_pptx_dependencies(self) -> None:
        # Given/When
        source = Path(synthetic_reference.__file__).read_text(encoding="utf-8")

        # Then
        for forbidden in (
            "candidate_render",
            "strict_pixel_compare",
            "pptx",
            "crates",
        ):
            self.assertNotIn(forbidden, source)

    def test_cli_writes_reference_html_to_requested_directory(self) -> None:
        # Given
        main = getattr(synthetic_reference, "main", None)

        # When/Then
        self.assertTrue(callable(main))
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "references"
            exit_code = main(["--output", str(output)])

            self.assertEqual(exit_code, 0)
            self.assertEqual(len(list(output.glob("synthetic_*.html"))), 10)


if __name__ == "__main__":
    unittest.main()
