// Auto-split from renderer/geometry.rs (mechanical move, no logic edits).
// Family: custom_geom

use super::arc_math::polar_ellipse_offset;
use super::shared::{CustomGeomPathSvg, CustomGeomSvg};
use crate::model::{CustomGeometry, GeometryPath, PathCommand};
use std::fmt::Write;

pub fn custom_geometry_svg(
    geom: &CustomGeometry,
    shape_w: f64,
    shape_h: f64,
) -> Option<CustomGeomSvg> {
    if geom.paths.is_empty() {
        return None;
    }
    let mut result_paths = Vec::with_capacity(geom.paths.len());
    for path in &geom.paths {
        let d = geometry_path_to_svg(path, shape_w, shape_h);
        result_paths.push(CustomGeomPathSvg {
            d,
            fill: path.fill.clone(),
            stroke: true,
        });
    }
    Some(CustomGeomSvg {
        paths: result_paths,
    })
}

pub(super) fn geometry_path_to_svg(path: &GeometryPath, shape_w: f64, shape_h: f64) -> String {
    let path_w = if path.width > 0.0 {
        path.width
    } else {
        shape_w
    };
    let path_h = if path.height > 0.0 {
        path.height
    } else {
        shape_h
    };
    let sx = shape_w / path_w;
    let sy = shape_h / path_h;
    let mut d = String::with_capacity(256);
    let mut cur_x = 0.0_f64;
    let mut cur_y = 0.0_f64;
    for cmd in &path.commands {
        match cmd {
            PathCommand::MoveTo { x, y } => {
                let px = x * sx;
                let py = y * sy;
                let _ = write!(d, "M{px:.2},{py:.2} ");
                cur_x = px;
                cur_y = py;
            }
            PathCommand::LineTo { x, y } => {
                let px = x * sx;
                let py = y * sy;
                let _ = write!(d, "L{px:.2},{py:.2} ");
                cur_x = px;
                cur_y = py;
            }
            PathCommand::CubicBezTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                let _ = write!(
                    d,
                    "C{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} ",
                    x1 * sx,
                    y1 * sy,
                    x2 * sx,
                    y2 * sy,
                    x * sx,
                    y * sy
                );
                cur_x = x * sx;
                cur_y = y * sy;
            }
            PathCommand::QuadBezTo { x1, y1, x, y } => {
                let _ = write!(
                    d,
                    "Q{:.2},{:.2} {:.2},{:.2} ",
                    x1 * sx,
                    y1 * sy,
                    x * sx,
                    y * sy
                );
                cur_x = x * sx;
                cur_y = y * sy;
            }
            PathCommand::ArcTo {
                wr,
                hr,
                start_angle,
                swing_angle,
            } => {
                let rx = wr * sx;
                let ry = hr * sy;
                if rx.abs() < 0.001 || ry.abs() < 0.001 {
                    continue;
                }
                let st_deg = start_angle / 60000.0;
                let sw_deg = swing_angle / 60000.0;
                if sw_deg.abs() < 0.001 {
                    continue;
                }
                let st_rad = st_deg.to_radians();
                let end_rad = (st_deg + sw_deg).to_radians();
                let Some(start_offset) = polar_ellipse_offset(*wr, *hr, st_rad) else {
                    continue;
                };
                let Some(end_offset) = polar_ellipse_offset(*wr, *hr, end_rad) else {
                    continue;
                };
                let end_x = cur_x + (end_offset.0 - start_offset.0) * sx;
                let end_y = cur_y + (end_offset.1 - start_offset.1) * sy;
                let large_arc = if sw_deg.abs() > 180.0 { 1 } else { 0 };
                let sweep = if sw_deg > 0.0 { 1 } else { 0 };
                let _ = write!(
                    d,
                    "A{rx:.2},{ry:.2} 0 {large_arc},{sweep} {end_x:.2},{end_y:.2} "
                );
                cur_x = end_x;
                cur_y = end_y;
            }
            PathCommand::Close => {
                d.push_str("Z ");
            }
        }
    }
    d.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::geometry_path_to_svg;
    use crate::model::{GeometryPath, PathCommand, PathFill};

    #[test]
    fn unequal_radii_arc_converts_source_angles_before_anisotropic_scaling() {
        let path = GeometryPath {
            width: 40.0,
            height: 20.0,
            commands: vec![
                PathCommand::MoveTo { x: 0.0, y: 10.0 },
                PathCommand::ArcTo {
                    wr: 20.0,
                    hr: 10.0,
                    start_angle: 10_800_000.0,
                    swing_angle: -2_700_000.0,
                },
            ],
            fill: PathFill::Norm,
        };

        assert_eq!(
            geometry_path_to_svg(&path, 80.0, 60.0),
            "M0.00,30.00 A40.00,30.00 0 0,0 22.11,56.83"
        );
    }
}
