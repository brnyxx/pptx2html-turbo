from __future__ import annotations

from evaluate.multiformat_corpus_items import require_keys
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_schema import JsonValue, integer_value


def validate_capture_rendering(
    values: dict[str, JsonValue],
    document_format: str,
) -> None:
    require_keys(values, {"dpi", "width", "height"}, "capture.rendering")
    if document_format in {"ppt", "pptx"}:
        valid = (
            values.get("dpi") is None
            and integer_value(values, "width") == 960
            and integer_value(values, "height") == 540
        )
    else:
        valid = (
            integer_value(values, "dpi") == 144
            and values.get("width") is None
            and values.get("height") is None
        )
    if not valid:
        raise MetricError("artifact.dimension", document_format)
