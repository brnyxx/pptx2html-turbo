import importlib.util
import unittest

from evaluate import synthetic_scene


class SyntheticSceneTests(unittest.TestCase):
    def test_scene_module_exists_for_independent_oracle(self) -> None:
        module = importlib.util.find_spec("evaluate.synthetic_scene")

        self.assertIsNotNone(module)

    def test_corpus_has_ten_unique_decks_with_ten_unique_scenes(self) -> None:
        # Given/When
        create_corpus = getattr(synthetic_scene, "create_synthetic_corpus", None)

        # Then
        self.assertTrue(callable(create_corpus))
        corpus = create_corpus()
        self.assertEqual(len(corpus), 10)
        self.assertEqual(
            [deck.name for deck in corpus],
            [f"synthetic_{index:02d}" for index in range(1, 11)],
        )
        self.assertTrue(all(len(deck.scenes) == 10 for deck in corpus))
        scenes = [scene for deck in corpus for scene in deck.scenes]
        self.assertEqual(len({scene.scene_id for scene in scenes}), 100)
        self.assertEqual(
            len({(scene.background, scene.rectangles) for scene in scenes}),
            100,
        )

    def test_corpus_regeneration_is_deterministic(self) -> None:
        # Given/When
        create_corpus = getattr(synthetic_scene, "create_synthetic_corpus", None)

        # Then
        self.assertTrue(callable(create_corpus))
        first = create_corpus()
        second = create_corpus()
        self.assertEqual(first, second)

    def test_scene_geometry_uses_in_bounds_whole_pixel_emu_values(self) -> None:
        # Given/When
        create_corpus = getattr(synthetic_scene, "create_synthetic_corpus", None)

        # Then
        self.assertTrue(callable(create_corpus))
        corpus = create_corpus()
        emu_per_pixel = getattr(synthetic_scene, "EMU_PER_PIXEL", None)
        canvas_width = getattr(synthetic_scene, "CANVAS_WIDTH_EMU", None)
        canvas_height = getattr(synthetic_scene, "CANVAS_HEIGHT_EMU", None)
        self.assertIsInstance(emu_per_pixel, int)
        self.assertIsInstance(canvas_width, int)
        self.assertIsInstance(canvas_height, int)
        for deck in corpus:
            for scene in deck.scenes:
                self.assertGreaterEqual(len(scene.rectangles), 6)
                for rectangle in scene.rectangles:
                    values = (
                        rectangle.x,
                        rectangle.y,
                        rectangle.width,
                        rectangle.height,
                    )
                    self.assertTrue(
                        all(value % emu_per_pixel == 0 for value in values),
                        scene.scene_id,
                    )
                    self.assertGreaterEqual(rectangle.x, 0)
                    self.assertGreaterEqual(rectangle.y, 0)
                    self.assertGreater(rectangle.width, 0)
                    self.assertGreater(rectangle.height, 0)
                    self.assertLessEqual(
                        rectangle.x + rectangle.width,
                        canvas_width,
                    )
                    self.assertLessEqual(
                        rectangle.y + rectangle.height,
                        canvas_height,
                    )


if __name__ == "__main__":
    unittest.main()
