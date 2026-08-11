use std::fmt::Write;

use super::{
    Position, RenderCtx, Size, UnresolvedElement, UnresolvedType, UnsupportedData, escape_html,
};

pub(super) fn render_unsupported(
    data: &UnsupportedData,
    pos: Position,
    size: Size,
    ctx: &RenderCtx<'_>,
    html: &mut String,
) {
    let mut coll = ctx.collector.borrow_mut();
    let placeholder_id = format!("unresolved-s{}-e{}", coll.current_slide_index, coll.counter);
    coll.counter += 1;

    let type_attr = match data.element_type {
        UnresolvedType::SmartArt => "smartart",
        UnresolvedType::OleObject => "ole",
        UnresolvedType::MathEquation => "math",
        UnresolvedType::CustomGeometry => "custom-geometry",
    };

    let escaped = escape_html(&data.label);
    let _ = writeln!(
        html,
        "<div class=\"unresolved-element\" id=\"{placeholder_id}\" \
                 data-type=\"{type_attr}\" data-slide=\"{}\">\
                 <span>[{escaped}]</span></div>",
        coll.current_slide_index
    );

    let pos_non_zero = pos.x.0 != 0 || pos.y.0 != 0;
    let size_non_zero = size.width.0 != 0 || size.height.0 != 0;
    let slide_idx = coll.current_slide_index;
    let elem = UnresolvedElement {
        slide_index: slide_idx,
        element_type: data.element_type.clone(),
        placeholder_id,
        position: if pos_non_zero { Some(pos) } else { None },
        size: if size_non_zero { Some(size) } else { None },
        raw_xml: data.raw_xml.clone(),
        data_model: None,
    };
    coll.elements.push(elem);

    drop(coll);
    html.push_str("</div>\n");
}
